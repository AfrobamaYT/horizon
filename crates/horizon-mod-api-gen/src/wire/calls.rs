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
