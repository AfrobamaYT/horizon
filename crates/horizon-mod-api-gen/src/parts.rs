//! Discover ordinary Parts modules, without joining their runtime bodies.
use crate::catalog::Catalog;
use std::{fs, path::Path};
use syn::{Item, Visibility};

pub(crate) const DIRECTORY: &str = "crates/horizon-mod-api/src/parts";
pub(crate) const OUTPUT: &str = "parts_modules.rs";

fn visibility(item: &Item) -> Option<&Visibility> {
    match item {
        Item::Const(i) => Some(&i.vis),
        Item::Enum(i) => Some(&i.vis),
        Item::Fn(i) => Some(&i.vis),
        Item::Static(i) => Some(&i.vis),
        Item::Struct(i) => Some(&i.vis),
        Item::Trait(i) => Some(&i.vis),
        Item::Type(i) => Some(&i.vis),
        Item::Use(i) => Some(&i.vis),
        _ => None,
    }
}

fn reexport(items: &[Item], module: &str) -> String {
    let mut visible = items.iter().filter_map(visibility);
    let public = visible.clone().any(|v| matches!(v, Visibility::Public(_)));
    if !public && !visible.any(|v| !matches!(v, Visibility::Inherited)) {
        return String::new();
    }
    let scope = if public { "pub" } else { "pub(crate)" };
    // Re-exports form the internal cross-kind namespace, even for a kind whose
    // helpers currently have no sibling consumer in one feature configuration.
    format!("#[allow(unused_imports)] {scope} use {module}::*;\n")
}

pub(crate) fn source(root: &Path) -> Result<String, String> {
    let dir = root.join(DIRECTORY);
    let mut modules = String::new();
    let mut fixtures = String::new();
    for kind in Catalog::modules(&dir).map_err(|e| e.to_string())? {
        if kind == "mod" {
            continue;
        }
        let path = dir.join(format!("{kind}.rs"));
        let source = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let file = syn::parse_file(&source).map_err(|e| format!("{}: {e}", path.display()))?;
        let path = path.to_string_lossy();
        modules.push_str(&format!("#[path = {path:?}] mod {kind};\n"));
        modules.push_str(&reexport(&file.items, &kind));
        for item in &file.items {
            if let Item::Mod(module) = item
                && module.ident == "tests"
                && !matches!(module.vis, Visibility::Inherited)
                && let Some((_, items)) = &module.content
            {
                fixtures.push_str(&reexport(items, &format!("super::{kind}::tests")));
            }
        }
    }
    modules.push_str(&format!(
        "/// Shared Parts fixtures for engine tests.\n#[cfg(any(test, feature = \"testing\"))]\npub mod tests {{\n{fixtures}}}\n"
    ));
    Ok(modules)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_parts_kind_needs_no_shared_list() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../target/parts-discovery-{}",
            std::process::id()
        ));
        let dir = root.join(DIRECTORY);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("mod.rs"), "").unwrap();
        fs::write(
            dir.join("zeta.rs"),
            "pub struct Last; #[cfg(test)] mod tests { fn local_test() {} }",
        )
        .unwrap();
        let first = source(&root).unwrap();
        assert!(first.contains("mod zeta;"));
        assert!(first.contains("pub use zeta::*;"));
        assert!(!first.contains("super::zeta::tests"));
        fs::write(
            dir.join("alpha.rs"),
            "pub struct First; #[cfg(test)] pub mod tests { pub fn fixture() {} }",
        )
        .unwrap();
        let added = source(&root).unwrap();
        syn::parse_file(&added).unwrap();
        assert!(added.find("mod alpha;") < added.find("mod zeta;"));
        assert!(added.contains("pub use super::alpha::tests::*;"));
        assert!(!added.contains("struct First"));
        fs::remove_file(dir.join("alpha.rs")).unwrap();
        let removed = source(&root).unwrap();
        assert!(!removed.contains("alpha"));
        fs::remove_dir_all(root).unwrap();
    }
}
