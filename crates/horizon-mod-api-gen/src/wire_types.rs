//! Discovery of concrete, publicly reachable mod wire types.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
use syn::{Item, Visibility};

type Exports = BTreeMap<String, Vec<(String, String)>>;

/// Lists concrete wire types by a public path from the crate root, following re-exports.
/// Conditional declarations and uninstantiated generic types are excluded.
/// Uses the point scanner's module traversal, including explicit module paths.
pub fn public_types(krate: &str, lib: &Path) -> Result<Vec<String>, String> {
    let dir = lib.parent().ok_or("library has no source directory")?;
    let files = crate::points::module_files(krate.into(), lib.into(), dir.into(), true)?;
    let mut exports = BTreeMap::new();
    let mut types = BTreeSet::new();
    for file in files {
        let source = fs::read_to_string(file.path).map_err(|e| e.to_string())?;
        let parsed = syn::parse_file(&source).map_err(|e| e.to_string())?;
        items(krate, &file.module, &parsed.items, &mut exports, &mut types);
    }
    let mut found = BTreeSet::new();
    visit(krate, krate, &exports, &types, &mut Vec::new(), &mut found);
    Ok(found.into_iter().collect())
}

fn visit(
    module: &str,
    public: &str,
    exports: &Exports,
    types: &BTreeSet<String>,
    visiting: &mut Vec<String>,
    found: &mut BTreeSet<String>,
) {
    if visiting.iter().any(|m| m == module) {
        return;
    }
    visiting.push(module.into());
    for (name, target) in exports.get(module).into_iter().flatten() {
        if name == "*" {
            visit(target, public, exports, types, visiting, found);
            continue;
        }
        let path = format!("{public}::{name}");
        if types.contains(target) {
            found.insert(path);
        } else {
            visit(target, &path, exports, types, visiting, found);
        }
    }
    visiting.pop();
}

fn uses(
    krate: &str,
    module: &str,
    tree: &syn::UseTree,
    prefix: &str,
    out: &mut Vec<(String, String)>,
) {
    match tree {
        syn::UseTree::Path(p) => uses(
            krate,
            module,
            &p.tree,
            &format!("{prefix}{}::", p.ident),
            out,
        ),
        syn::UseTree::Group(g) => {
            for tree in &g.items {
                uses(krate, module, tree, prefix, out);
            }
        }
        other => {
            let (name, suffix) = match other {
                syn::UseTree::Name(n) => (n.ident.to_string(), n.ident.to_string()),
                syn::UseTree::Rename(n) => (n.rename.to_string(), n.ident.to_string()),
                syn::UseTree::Glob(_) => ("*".into(), String::new()),
                _ => return,
            };
            let raw = format!("{prefix}{suffix}");
            let mut parts: Vec<_> = module.split("::").collect();
            let mut path = raw.trim_end_matches("::");
            if let Some(rest) = path.strip_prefix("crate::") {
                parts = vec![krate];
                path = rest;
            } else if let Some(rest) = path.strip_prefix("self::") {
                path = rest;
            }
            while let Some(rest) = path.strip_prefix("super::") {
                parts.pop();
                path = rest;
            }
            out.push((name, format!("{}::{path}", parts.join("::"))));
        }
    }
}

fn items(
    krate: &str,
    module: &str,
    items: &[Item],
    exports: &mut Exports,
    types: &mut BTreeSet<String>,
) {
    let manual: BTreeSet<_> = items
        .iter()
        .filter_map(|item| {
            let Item::Impl(i) = item else { return None };
            let (_, path, _) = i.trait_.as_ref()?;
            if path.segments.last()?.ident != "Wire" || !i.generics.params.is_empty() {
                return None;
            }
            let syn::Type::Path(p) = &*i.self_ty else {
                return None;
            };
            (p.path.segments.len() == 1).then(|| p.path.segments[0].ident.to_string())
        })
        .collect();
    for item in items {
        let (ident, vis, attrs, generics) = match item {
            Item::Struct(i) => (&i.ident, &i.vis, &i.attrs, &i.generics),
            Item::Enum(i) => (&i.ident, &i.vis, &i.attrs, &i.generics),
            Item::Use(i)
                if matches!(i.vis, Visibility::Public(_))
                    && !i.attrs.iter().any(|a| a.path().is_ident("cfg")) =>
            {
                uses(
                    krate,
                    module,
                    &i.tree,
                    "",
                    exports.entry(module.into()).or_default(),
                );
                continue;
            }
            Item::Mod(i) if !i.attrs.iter().any(|a| a.path().is_ident("cfg")) => {
                let child = format!("{module}::{}", i.ident);
                if matches!(i.vis, Visibility::Public(_)) {
                    exports
                        .entry(module.into())
                        .or_default()
                        .push((i.ident.to_string(), child.clone()));
                }
                if let Some((_, content)) = &i.content {
                    self::items(krate, &child, content, exports, types);
                }
                continue;
            }
            _ => continue,
        };
        if !matches!(vis, Visibility::Public(_))
            || !generics.params.is_empty()
            || attrs.iter().any(|a| a.path().is_ident("cfg"))
        {
            continue;
        }
        let derived = attrs
            .iter()
            .filter(|a| a.path().is_ident("derive"))
            .any(|a| {
                a.parse_args_with(
                    syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
                )
                .is_ok_and(|paths| {
                    paths
                        .iter()
                        .any(|p| p.segments.last().is_some_and(|s| s.ident == "Wire"))
                })
            });
        if derived || manual.contains(&ident.to_string()) {
            let path = format!("{module}::{ident}");
            types.insert(path.clone());
            exports
                .entry(module.into())
                .or_default()
                .push((ident.to_string(), path));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_module_paths_and_manual_codecs_are_discovered() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../target/wire-discovery-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("lib.rs"), "#[derive(mod_api::Wire)] pub struct Root; #[derive(Wire)] struct Private; pub mod visible; mod hidden; pub use hidden::Hidden as Reexported; pub mod inline { #[derive(Wire)] pub enum Value { A } } pub struct Manual; impl wire::Wire for Manual {} #[cfg(test)] mod tests; ").unwrap();
        fs::write(root.join("visible.rs"), "#[derive(Wire)] pub struct Child; #[cfg(unix)] #[derive(Wire)] pub struct Conditional;").unwrap();
        fs::write(root.join("hidden.rs"), "#[derive(Wire)] pub struct Hidden;").unwrap();
        assert_eq!(
            public_types("library", &root.join("lib.rs")).unwrap(),
            vec![
                "library::Manual",
                "library::Reexported",
                "library::Root",
                "library::inline::Value",
                "library::visible::Child"
            ]
        );
    }
}
