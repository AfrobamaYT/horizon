//! Host code. Runtime contract (on WasmMod and WasmHandler):
//! `fn call<R>(&self, class: CallClass, frame: Frame, id: u32, handler: Option<Id>,
//! encode: impl FnOnce(&mut Writer), decode: impl FnOnce(&mut Reader<'_>) -> Result<R, WireError>) -> Option<R>`.
//! `None` means refused/trapped; the runtime poisons the group before returning it.
use super::{
    Method,
    calls::*,
    ty::{Kind, Ty},
};
use heck::ToSnakeCase;
use std::fmt::Write;

fn snake(owner: &str) -> String {
    owner.rsplit("::").next().unwrap().to_snake_case()
}

fn frame(surface: &mut super::Surface<'_>) -> Result<String, String> {
    use crate::index::Found;
    let types = &mut surface.types;
    let api = types.index.module("mod_api", &[])?;
    let mut supers = Vec::new();
    for owner in super::IMPORTS {
        let path = owner.split("::").map(str::to_string).collect::<Vec<_>>();
        let Found::Item(id) = types.index.resolve_path(api, &path)? else {
            return Err(format!("{owner} is not a trait"));
        };
        let syn::Item::Trait(item) = types.index.item(id) else {
            return Err(format!("{owner} is not a trait"));
        };
        let parents = item
            .supertraits
            .iter()
            .filter_map(|b| match b {
                syn::TypeParamBound::Trait(t) => Some(t.path.segments.last()?.ident.to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();
        supers.push((*owner, parents));
    }
    let mut fields = String::new();
    let mut methods = String::new();
    for owner in super::IMPORTS {
        let name = snake(owner);
        let path = trait_path(owner);
        if let Some((_, from, expr)) = super::REACHED.iter().find(|(o, ..)| o == owner) {
            let mutable = from.ends_with("_mut")
                || (from.is_empty()
                    && surface
                        .methods
                        .iter()
                        .any(|m| m.owner == *owner && m.receiver == Some(true)));
            let body = if from.is_empty() {
                expr.replace("()", "(self)")
            } else {
                format!("let h = self.{from}()?; {expr}")
            };
            // Rewrite the old free frame accessors to methods, preserving their fallback order.
            let body = if from.is_empty() {
                body.replace("level_host(self)", "self.level_host()")
                    .replace("host(self)", "self.host()")
                    .replace(
                        "level_random_host_mut(self)",
                        "self.level_random_host_mut()",
                    )
                    .replace("host_mut(self)", "self.host_mut()")
            } else {
                body
            };
            writeln!(methods, "pub unsafe fn {name}{suffix}(&self) -> Result<&'static {m}dyn {path}, ServeError> {{ {body} }}", suffix=if mutable {"_mut"} else {""}, m=if mutable {"mut "} else {""}).unwrap();
            continue;
        }
        writeln!(
            fields,
            "{name}: Option<(*mut (dyn {path} + 'static), bool)>,"
        )
        .unwrap();
        writeln!(methods, "pub fn set_{name}(&mut self, v: &(dyn {path} + '_)) {{ self.{name} = Some((unsafe {{ std::mem::transmute(v as *const (dyn {path} + '_)) }}, false)); }} pub fn set_{name}_mut(&mut self, v: &mut (dyn {path} + '_)) {{ self.{name} = Some((unsafe {{ std::mem::transmute(v as *mut (dyn {path} + '_)) }}, true)); }}").unwrap();
        for mutable in [false, true] {
            let mut choices = Vec::new();
            for sub in super::IMPORTS {
                if super::REACHED.iter().any(|(o, ..)| o == sub) {
                    continue;
                }
                let mut stack = vec![sub.rsplit("::").next().unwrap().to_string()];
                let mut serves = false;
                while let Some(t) = stack.pop() {
                    if t == owner.rsplit("::").next().unwrap() {
                        serves = true;
                        break;
                    }
                    if let Some((_, parents)) = supers
                        .iter()
                        .find(|(o, _)| o.rsplit("::").next() == Some(t.as_str()))
                    {
                        stack.extend(parents.iter().cloned());
                    }
                }
                if serves {
                    choices.push(format!(
                        "self.{}{filter}.map(|(p, _)| unsafe {{ &{m}*p as &{m}dyn {path} }})",
                        snake(sub),
                        filter = if mutable { ".filter(|(_, m)| *m)" } else { "" },
                        m = if mutable { "mut " } else { "" }
                    ));
                }
            }
            let expr = choices.join(".or_else(|| ");
            let expr = format!("{expr}{}", ")".repeat(choices.len().saturating_sub(1)));
            writeln!(methods, "pub unsafe fn {name}{suffix}(&self) -> Result<&'static {m}dyn {path}, ServeError> {{ {expr}.ok_or(ServeError::Missing(\"{name}\")) }}", suffix=if mutable {"_mut"} else {""}, m=if mutable {"mut "} else {""}).unwrap();
        }
    }
    Ok(format!(
        "#[derive(Default)] pub struct Frame {{ {fields} }} impl Frame {{ {methods} }}"
    ))
}

fn fallback(m: &Method) -> &'static str {
    match &m.ret {
        Ty::Result(ok, _) if **ok == Ty::Unit => "Ok(())",
        Ty::Result(..) => "Err(refused())",
        _ => "NoAnswer::no_answer()",
    }
}

pub(super) fn generate(surface: &mut super::Surface<'_>) -> Result<String, String> {
    let mut code = String::from(
        "// @generated by horizon-mod-api-gen; do not edit.\nuse mod_api::wire::{Reader, Writer, WireError, Slots};\n#[derive(Debug)] pub enum ServeError { Decode(WireError), Missing(&'static str), Stale, Unsupported(u32) } impl From<WireError> for ServeError { fn from(e: WireError) -> Self { Self::Decode(e) } } fn missing(s: &'static str) -> ServeError { ServeError::Missing(s) }\n#[derive(Default)] pub struct Cx { pub frames: Vec<Frame>, pub held: Slots<Box<dyn std::any::Any + Send>> } impl Cx { pub fn frame(&self) -> Result<&Frame, ServeError> { self.frames.last().ok_or(ServeError::Missing(\"frame\")) } }\n",
    );
    code.push_str("use crate::{NoAnswer, refused};\n");
    code.push_str(&frame(surface)?);
    let types = &surface.types;
    code.push_str("pub unsafe fn serve(id: u32, cx: *mut Cx, bytes: &[u8], out: &mut Writer) -> Result<(), ServeError> { let mut input = Reader::new(bytes); match id {\n");
    for (id, m) in imports(&surface.methods).iter().enumerate() {
        if !plain(m) {
            continue;
        }
        let mut body = String::new();
        let mut args = Vec::new();
        for (i, (_, ty)) in m.params.iter().enumerate() {
            writeln!(
                body,
                "let p{i}: {} = input.take()?;",
                held(types, ty).unwrap()
            )
            .unwrap();
            args.push(format!(
                "{}p{i}",
                if matches!(
                    ty,
                    Ty::Ref(..)
                        | Ty::Str(super::ty::Str::Borrowed)
                        | Ty::List(_, super::ty::List::Slice)
                ) {
                    "&"
                } else {
                    ""
                }
            ));
        }
        writeln!(code, "{id} => {{ {body} let receiver = unsafe {{ (&*cx).frame()?.{}{}()? }}; let answer = receiver.{}({}); out.put(&answer); }},", snake(m.owner), if m.receiver == Some(true) {"_mut"} else {""}, m.name, args.join(",")).unwrap();
    }
    code.push_str("_ => return Err(ServeError::Unsupported(id)) } Ok(()) }\n");
    let mut gaps = Vec::new();
    for owner in super::exports() {
        let receiver = if super::mod_receiver(owner).is_some() {
            "WasmMod"
        } else {
            "WasmHandler"
        };
        let mut groups = vec![owner.to_string()];
        for m in surface.methods.iter().filter(|m| m.owner == *owner) {
            if let Some(via) = &m.via {
                if !groups.contains(via) {
                    groups.push(via.clone());
                }
            }
        }
        for group in groups {
            writeln!(code, "impl {} for crate::{receiver} {{", trait_path(&group)).unwrap();
            if group == *owner {
                for (o, method) in super::SUBSCRIBED {
                    if o == owner {
                        let body = if receiver == "WasmMod" {
                            "self.subscribed".to_string()
                        } else {
                            format!(
                                "self.subscriptions({owner:?}, {})",
                                surface
                                    .methods
                                    .iter()
                                    .filter(|m| super::exports().contains(&m.owner))
                                    .count()
                                    + super::SUBSCRIBED
                                        .iter()
                                        .position(|(o, _)| o == owner)
                                        .unwrap()
                            )
                        };
                        writeln!(
                            code,
                            "fn {method}(&self) -> &'static [Option<mod_api::HookId>] {{ {body} }}"
                        )
                        .unwrap();
                    }
                }
            }
            for (id, m) in surface
                .methods
                .iter()
                .filter(|m| super::exports().contains(&m.owner))
                .enumerate()
                .filter(|(_, m)| m.owner == *owner && m.via.as_deref().unwrap_or(m.owner) == group)
            {
                let sig = rust_sig(types, m);
                if !dispatchable(types, m) {
                    gaps.push(format!("{}::{}", m.owner, m.name));
                    writeln!(code, "{sig} {{ {} }}", fallback(m)).unwrap();
                    continue;
                }
                let (mut setup, mut put, mut take, mut apply) =
                    (String::new(), String::new(), String::new(), String::new());
                for (i, (_, ty)) in m.params.iter().enumerate() {
                    match ty {
                        Ty::Dyn(name, mutable) => {
                            write!(
                                setup,
                                "frame.set_{}{}(p{i});",
                                snake(name),
                                if *mutable { "_mut" } else { "" }
                            )
                            .unwrap();
                        }
                        Ty::Ref(inner, mutable) if matches!(&**inner, Ty::Named(d) if types.defs[*d].lifetimes != 0) =>
                        {
                            let Ty::Named(d) = &**inner else {
                                unreachable!()
                            };
                            let Kind::Record(fields) = &types.defs[*d].kind else {
                                unreachable!()
                            };
                            // The guest uses the same lent description to choose writable fields.
                            let mut own = Vec::new();
                            let mut refs = Vec::new();
                            for (j, f) in fields.iter().enumerate() {
                                let lent = lend(types, &f.ty, &format!("p{i}_{j}")).unwrap();
                                write!(put, "w.put(&p{i}.{});", f.rust).unwrap();
                                if *mutable && lent.mutable {
                                    let pair = (j, f, lent);
                                    if matches!(f.ty, Ty::Ref(..) | Ty::Option(..))
                                        && pair.2.expr != format!("p{i}_{j}")
                                    {
                                        refs.push(pair);
                                    } else {
                                        own.push(pair);
                                    }
                                }
                            }
                            for (j, f, lent) in own.into_iter().chain(refs) {
                                write!(take, "let b{i}_{j}: {} = r.take()?;", lent.owned).unwrap();
                                write!(
                                    apply,
                                    "{}p{i}.{} = b{i}_{j};",
                                    if matches!(f.ty, Ty::Ref(_, true)) {
                                        "*"
                                    } else {
                                        ""
                                    },
                                    f.rust
                                )
                                .unwrap();
                            }
                        }
                        _ => {
                            write!(put, "w.put(&p{i});").unwrap();
                            if matches!(ty, Ty::Ref(_, true)) {
                                write!(take, "let b{i}: {} = r.take()?;", held(types, ty).unwrap())
                                    .unwrap();
                                write!(apply, "*p{i} = b{i};").unwrap();
                            }
                        }
                    }
                }
                let class =
                    super::call_class(types, m).replace("super::Call::", "crate::CallClass::");
                writeln!(code,"{sig} {{ let mut frame = Frame::default(); {setup} self.call({class}, frame, {id}, {}, |w| {{ {put} }}, |r| {{ let answer: {} = r.take()?; {take} if !r.rest().is_empty() {{ return Err(WireError::Tag(u32::MAX)); }} {apply} Ok(answer) }}).unwrap_or_else(|| {}) }}",if receiver=="WasmMod" {"None"} else {"Some(self.index)"},rust_ty(types,&m.ret,true),fallback(m)).unwrap();
            }
            code.push_str("}\n");
        }
    }
    writeln!(code, "pub const UNSUPPORTED_EXPORTS: &[&str] = &{gaps:?};").unwrap();
    // The API is qualified as crate in the shared signature writer, but lives in mod_api here.
    Ok(code
        .replace("crate::", "mod_api::")
        .replace("mod_api::Wasm", "crate::Wasm")
        .replace("mod_api::CallClass", "crate::CallClass")
        .replace("mod_api::refused", "crate::refused")
        .replace("mod_api::NoAnswer", "crate::NoAnswer")
        .replace(
            "use mod_api::{NoAnswer, refused}",
            "use crate::{NoAnswer, refused}",
        ))
}

/// Generate the host boundary from the same source surface as the guest.
pub fn generate_host(root: &std::path::Path) -> Result<String, String> {
    let generated = crate::sources(root, crate::FillMode::Require)?;
    let providers = crate::capability_providers::generate(root, &[])?.0;
    let included = [
        ("mod_api_generated.rs", generated.hooks.as_str()),
        ("mod_api_generated_host.rs", generated.host.as_str()),
        ("mod_api_host_areas.rs", generated.areas.as_str()),
        ("capability_providers_generated.rs", providers.as_str()),
    ];
    let mut surface = super::surface(root, &included)?;
    let count = surface
        .methods
        .iter()
        .filter(|m| super::exports().contains(&m.owner))
        .count();
    let mut code = generate(&mut surface)?;
    writeln!(code, "pub const SUBSCRIBED: u32 = {count};").unwrap();
    code.push_str("pub fn hook_id(name: &str) -> Option<mod_api::HookId> { match name {\n");
    for hook in crate::Catalog::load(root)?.hooks {
        writeln!(
            code,
            "{:?} => Some(mod_api::HookId::{}),",
            hook.name,
            crate::hook_id(&hook.name)
        )
        .unwrap();
    }
    code.push_str("_ => None } } pub fn subscriptions_of(owner: &str, names: Vec<String>) -> &'static [Option<mod_api::HookId>] { names.into_iter().filter_map(|name| { let id = hook_id(&name); if id.is_none() { log::warn!(\"wasm {owner}: unknown hook {name}\"); } id.map(Some) }).collect::<Vec<_>>().leak() }\n");
    Ok(code)
}
