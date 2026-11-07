//! The guest's side of the calls: `Guest`, the host as a mod reaches it, each imported method
//! encoding its arguments, crossing once and decoding the reply. Methods with a `&dyn` frame
//! object, a handler or a borrowed return are listed apart: they need the call frame.
use super::ty::{List, Str, Ty, Types};
use super::{Method, exports};
use std::fmt::Write;

/// `ty` as Rust names it in a trait method's signature.
pub(super) fn rust_ty(types: &Types<'_>, ty: &Ty, ret: bool) -> String {
    match ty {
        Ty::Unit => "()".into(),
        Ty::Prim(rust, _) | Ty::Wide(rust) => (*rust).into(),
        Ty::Duration => "std::time::Duration".into(),
        Ty::Handler(name, false) => format!("std::sync::Arc<dyn {}>", trait_path(name)),
        Ty::Handler(name, true) => format!("Box<dyn {}>", trait_path(name)),
        Ty::Str(Str::Owned) => "String".into(),
        Ty::Str(Str::Borrowed) => "&str".into(),
        Ty::Str(Str::Static) => "&'static str".into(),
        Ty::Str(Str::Boxed) => "Box<str>".into(),
        Ty::Str(Str::Shared) => "std::sync::Arc<str>".into(),
        Ty::Str(Str::Cow) => "std::borrow::Cow<'static, str>".into(),
        Ty::List(item, List::Set(set)) => format!("{set}<{}>", rust_ty(types, item, ret)),
        Ty::List(item, List::Vec) => format!("Vec<{}>", rust_ty(types, item, ret)),
        Ty::List(item, List::Slice) => format!("&[{}]", rust_ty(types, item, ret)),
        Ty::List(item, List::Boxed) => format!("Box<[{}]>", rust_ty(types, item, ret)),
        Ty::List(item, List::Shared) => format!("std::sync::Arc<[{}]>", rust_ty(types, item, ret)),
        Ty::List(item, List::Map(map)) => match &**item {
            Ty::Tuple(pair) => format!(
                "{map}<{}, {}>",
                rust_ty(types, &pair[0], ret),
                rust_ty(types, &pair[1], ret)
            ),
            _ => unreachable!("a map's items are pairs"),
        },
        Ty::Array(item, n) => format!("[{}; {n}]", rust_ty(types, item, ret)),
        Ty::Option(item) => format!("Option<{}>", rust_ty(types, item, ret)),
        Ty::Result(a, b) => format!(
            "Result<{}, {}>",
            rust_ty(types, a, ret),
            rust_ty(types, b, ret)
        ),
        Ty::Tuple(items) => format!(
            "({},)",
            items
                .iter()
                .map(|i| rust_ty(types, i, ret))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Ty::Ref(inner, true) => format!("&mut {}", rust_ty(types, inner, ret)),
        Ty::Ref(inner, false) => format!("&{}", rust_ty(types, inner, ret)),
        Ty::Boxed(inner) => format!("Box<{}>", rust_ty(types, inner, ret)),
        Ty::Shared(inner) => format!("std::sync::Arc<{}>", rust_ty(types, inner, ret)),
        Ty::Named(def) => types.defs[*def].rust.clone(),
        Ty::Dyn(name, mutable) => {
            // A returned shared one is the guest's unit, for any borrow; a mutable one, being
            // invariant, keeps the borrow's own lifetime.
            let lifetime = if ret && !mutable { "'static " } else { "" };
            let m = if *mutable { "mut " } else { "" };
            format!("&{lifetime}{m}dyn {}", trait_path(name))
        }
    }
}

pub(super) fn trait_path(owner: &str) -> String {
    if owner == super::ERASED {
        return "std::any::Any + Send + Sync".into();
    }
    // Every name in it is `mod_api`'s, but the prelude's (`ResourceHandler<ItemResource>`,
    // `EntityCapabilityProvider<dyn EnergyHandler, Option<Direction>>`).
    struct Qualify;
    impl syn::visit_mut::VisitMut for Qualify {
        fn visit_path_mut(&mut self, path: &mut syn::Path) {
            syn::visit_mut::visit_path_mut(self, path);
            let outside = ["Option", "Vec", "Box", "String", "Result", "crate", "std"];
            if path.leading_colon.is_none()
                && !outside.contains(&path.segments[0].ident.to_string().as_str())
            {
                path.segments.insert(0, syn::parse_quote!(crate));
            }
        }
    }
    let mut path: syn::Path = syn::parse_str(owner).expect("an owner is a path");
    syn::visit_mut::VisitMut::visit_path_mut(&mut Qualify, &mut path);
    super::ty::written(&path)
}

/// A method's Rust signature, its arguments named `p0…`.
fn rust_sig(types: &Types<'_>, method: &Method) -> String {
    let mut params = Vec::new();
    match method.receiver {
        Some(true) => params.push("&mut self".to_string()),
        Some(false) => params.push("&self".to_string()),
        None => {}
    }
    for (i, (_, ty)) in method.params.iter().enumerate() {
        params.push(format!("p{i}: {}", rust_ty(types, ty, false)));
    }
    let ret = match &method.ret {
        Ty::Unit => String::new(),
        ty => format!(" -> {}", rust_ty(types, ty, true)),
    };
    format!("fn {}({}){ret}", method.name, params.join(", "))
}

/// The ids of every imported method, in the order they are generated: the table both sides
/// share, so an id is never sent as a name.
pub(super) fn imports<'m>(methods: &'m [Method]) -> Vec<&'m Method> {
    methods
        .iter()
        .filter(|m| !exports().contains(&m.owner))
        .collect()
}

fn has_frame(ty: &Ty) -> bool {
    match ty {
        Ty::Dyn(..) | Ty::Handler(..) => true,
        Ty::List(t, _)
        | Ty::Option(t)
        | Ty::Array(t, _)
        | Ty::Ref(t, _)
        | Ty::Boxed(t)
        | Ty::Shared(t) => has_frame(t),
        Ty::Result(a, b) => has_frame(a) || has_frame(b),
        Ty::Tuple(v) => v.iter().any(has_frame),
        _ => false,
    }
}

fn borrowed_ret(ty: &Ty) -> bool {
    matches!(
        ty,
        Ty::Ref(..) | Ty::Str(Str::Borrowed) | Ty::List(_, List::Slice)
    )
}

/// A plain method: nothing in it needs the call frame.
fn plain(m: &Method) -> bool {
    !m.params
        .iter()
        .any(|p| has_frame(&p.1) || matches!(&p.1, Ty::Ref(_, true)))
        && !has_frame(&m.ret)
        && !borrowed_ret(&m.ret)
}

/// `Guest` and its impls for every plain import, and the table of ids.
pub(super) fn guest(types: &Types<'_>, methods: &[Method]) -> String {
    let mut out =
        String::from("// @generated by horizon-mod-api-gen; do not edit.\npub struct Guest;\n");
    let all = imports(methods);
    let mut owners: Vec<&str> = Vec::new();
    for m in &all {
        let key = m.via.as_deref().unwrap_or(m.owner);
        if !owners.contains(&key) {
            owners.push(key);
        }
    }
    for owner in owners {
        writeln!(out, "impl {} for Guest {{", trait_path(owner)).unwrap();
        for (id, m) in all
            .iter()
            .enumerate()
            .filter(|(_, m)| m.via.as_deref().unwrap_or(m.owner) == owner)
        {
            if !plain(m) {
                continue;
            }
            let put: String = (0..m.params.len())
                .map(|i| format!("w.put(&p{i});"))
                .collect();
            let take = if m.ret == Ty::Unit {
                ""
            } else {
                "r.take().expect(\"the host answered with a value of the wrong type\")"
            };
            writeln!(
                out,
                "{} {{ let mut w = crate::wire::guest::begin(); {put} let mut r = crate::wire::guest::host({id}u32, w); {take} }}",
                rust_sig(types, m)
            )
            .unwrap();
        }
        out.push_str("}\n");
    }
    out
}

/// The type a decoded argument is held as: the owned form of a borrowed one.
fn held(types: &Types<'_>, ty: &Ty) -> Option<String> {
    Some(match ty {
        Ty::Str(Str::Borrowed) => "String".into(),
        Ty::List(item, List::Slice) => format!("Vec<{}>", held(types, item)?),
        Ty::Ref(inner, _) => held(types, inner)?,
        Ty::Named(def) if types.defs[*def].lifetimes != 0 => return None,
        Ty::Dyn(..) | Ty::Handler(..) => return None,
        Ty::Option(inner) => format!("Option<{}>", held(types, inner)?),
        Ty::List(item, List::Vec) => format!("Vec<{}>", held(types, item)?),
        other => rust_ty(types, other, false),
    })
}

/// How a field of a borrowed record is read into one: the expression building it from `local`,
/// the local's owned type, and whether the mod may change it (so it is sent back).
struct Lent {
    owned: String,
    expr: String,
    mutable: bool,
}

fn lend(types: &Types<'_>, ty: &Ty, local: &str) -> Option<Lent> {
    let (owned, expr, mutable) = match ty {
        Ty::Ref(inner, m) if !matches!(**inner, Ty::Dyn(..)) => (
            held(types, inner)?,
            format!("&{}{local}", if *m { "mut " } else { "" }),
            *m,
        ),
        Ty::Str(Str::Borrowed) => ("String".into(), format!("&{local}"), false),
        Ty::List(_, List::Slice) => (held(types, ty)?, format!("&{local}"), false),
        Ty::Option(inner) => match &**inner {
            Ty::Ref(t, m) => (
                format!("Option<{}>", held(types, t)?),
                format!("{local}.as_{}ref()", if *m { "m" } else { "" })
                    .replace("as_mref", "as_mut"),
                *m,
            ),
            Ty::Str(Str::Borrowed) | Ty::List(_, List::Slice) => {
                (held(types, ty)?, format!("{local}.as_deref()"), false)
            }
            _ => (held(types, ty)?, local.to_string(), true),
        },
        _ => (held(types, ty)?, local.to_string(), true),
    };
    Some(Lent {
        owned,
        expr,
        mutable,
    })
}

/// A record that borrows, as the statements reading it into locals, the expression building it
/// from them, and the statements sending back what a mod may change.
fn borrowed(
    types: &Types<'_>,
    def: usize,
    name: &str,
    mutable: bool,
) -> Option<(String, String, String)> {
    let d = &types.defs[def];
    let super::ty::Kind::Record(fields) = &d.kind else {
        return None;
    };
    let (mut read, mut build, mut back_ref, mut back_own) =
        (String::new(), Vec::new(), String::new(), String::new());
    for (i, f) in fields.iter().enumerate() {
        let local = format!("{name}_{i}");
        let lent = lend(types, &f.ty, &local)?;
        let m = if matches!(f.ty, Ty::Ref(_, true))
            || matches!(&f.ty, Ty::Option(t) if matches!(**t, Ty::Ref(_, true)))
        {
            "mut "
        } else {
            ""
        };
        writeln!(read, "let {m}{local}: {} = input.take()?;", lent.owned).unwrap();
        build.push(format!("{}: {}", f.rust, lent.expr));
        if mutable && lent.mutable {
            if matches!(f.ty, Ty::Ref(..) | Ty::Option(..)) && lent.expr != local {
                write!(back_ref, "out.put(&{local});").unwrap();
            } else {
                write!(back_own, "out.put(&{name}.{});", f.rust).unwrap();
            }
        }
    }
    let m = if mutable { "mut " } else { "" };
    read.push_str(&format!(
        "let {m}{name} = {} {{ {} }};",
        d.rust,
        build.join(", ")
    ));
    Some((read, format!("{name}"), format!("{back_own}{back_ref}")))
}

/// Whether an export has what its dispatch arm needs: every argument decodes to an owned value
/// or is the frame's host, and the answer is owned.
fn dispatchable(types: &Types<'_>, m: &Method) -> bool {
    !has_frame(&m.ret)
        && !borrowed_ret(&m.ret)
        && m.params.iter().all(|(_, ty)| match ty {
            Ty::Dyn(..) => true,
            Ty::Ref(inner, m) => match &**inner {
                Ty::Named(d) if types.defs[*d].lifetimes != 0 => {
                    borrowed(types, *d, "e", *m).is_some()
                }
                _ => held(types, ty).is_some(),
            },
            _ => held(types, ty).is_some(),
        })
}

/// The guest's `hook`: one arm for every export a mod can be called through, reading the
/// arguments from the buffer, calling the mod, and writing the answer then each `&mut`
/// argument back whole (the host takes from it the fields a mod may change). The arms'
/// ids are the exports' positions; the exports that need more than that are returned by name.
pub(super) fn dispatch(types: &Types<'_>, methods: &[Method]) -> (String, Vec<String>) {
    let all: Vec<&Method> = methods
        .iter()
        .filter(|m| exports().contains(&m.owner))
        .collect();
    let mut arms = String::new();
    let mut apart = Vec::new();
    for (id, m) in all.iter().enumerate() {
        if !dispatchable(types, m) {
            apart.push(format!("{}::{}", m.owner, m.name));
            continue;
        }
        let receiver = match super::mod_receiver(m.owner) {
            Some(receiver) => receiver.to_string(),
            None => format!(
                "crate::wire::guest::handler::<dyn {}>(input.take()?)?",
                trait_path(m.owner)
            ),
        };
        let mut body = String::new();
        let mut args = Vec::new();
        let mut back = String::new();
        for (i, (_, ty)) in m.params.iter().enumerate() {
            match ty {
                Ty::Dyn(_, true) => args.push("&mut host".to_string()),
                Ty::Dyn(..) => args.push("&host".to_string()),
                Ty::Ref(inner, m) if matches!(&**inner, Ty::Named(d) if types.defs[*d].lifetimes != 0) =>
                {
                    let Ty::Named(d) = &**inner else {
                        unreachable!()
                    };
                    let (read, name, send) = borrowed(types, *d, &format!("p{i}"), *m).unwrap();
                    writeln!(body, "{read}").unwrap();
                    args.push(format!("&{}{name}", if *m { "mut " } else { "" }));
                    back.push_str(&send);
                }
                Ty::Ref(_, true) => {
                    writeln!(
                        body,
                        "let mut p{i}: {} = input.take()?;",
                        held(types, ty).unwrap()
                    )
                    .unwrap();
                    args.push(format!("&mut p{i}"));
                    write!(back, "out.put(&p{i});").unwrap();
                }
                Ty::Ref(..) | Ty::Str(Str::Borrowed) | Ty::List(_, List::Slice) => {
                    writeln!(
                        body,
                        "let p{i}: {} = input.take()?;",
                        held(types, ty).unwrap()
                    )
                    .unwrap();
                    args.push(format!("&p{i}"));
                }
                _ => {
                    writeln!(
                        body,
                        "let p{i}: {} = input.take()?;",
                        held(types, ty).unwrap()
                    )
                    .unwrap();
                    args.push(format!("p{i}"));
                }
            }
        }
        writeln!(
            arms,
            "{id} => {{ let receiver = {receiver}; {body} let answer = receiver.{}({}); out.put(&answer); {back} }}",
            m.name,
            args.join(", ")
        )
        .unwrap();
    }
    let code = format!(
        "// @generated by horizon-mod-api-gen; do not edit.\npub fn hook(id: u32, input: &mut crate::wire::Reader<'_>, out: &mut crate::wire::Writer) -> Result<(), crate::wire::WireError> {{\nlet mut host = Guest;\nmatch id {{\n{arms}_ => return Err(crate::wire::WireError::Tag(id)) }}\nOk(())\n}}\n"
    );
    (code, apart)
}
