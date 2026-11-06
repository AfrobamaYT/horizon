//! Parts-kind fragments assembled at their original lexical scope.
use crate::catalog::Catalog;
use quote::ToTokens;
use std::{fs, path::Path};
use syn::Item;

pub(crate) const DIRECTORY: &str = "crates/horizon-mod-api/src/part_table";
pub(crate) const OUTPUT: &str = "part_table_generated.rs";

fn key(item: &Item) -> String {
    match item {
        Item::Trait(i) => format!("trait {}", i.ident),
        Item::Impl(i) => format!(
            "impl {} {}",
            i.self_ty.to_token_stream(),
            i.trait_
                .as_ref()
                .map(|(_, p, _)| p.to_token_stream().to_string())
                .unwrap_or_default()
        ),
        Item::Mod(i) => format!("mod {}", i.ident),
        Item::Fn(i) => format!("value {}", i.sig.ident),
        Item::Const(i) => format!("value {}", i.ident),
        Item::Static(i) => format!("value {}", i.ident),
        Item::Struct(i) => format!("type {}", i.ident),
        Item::Enum(i) => format!("type {}", i.ident),
        Item::Type(i) => format!("type {}", i.ident),
        _ => item.to_token_stream().to_string(),
    }
}

fn merge(out: &mut Vec<Item>, input: Vec<Item>) -> Result<(), String> {
    macro_rules! members {
        ($old:ident, $new:ident, $kind:ident, $identity:ident) => {{
            let members = std::mem::take(&mut $new.items);
            let mut header = $old.clone();
            header.items.clear();
            if header.to_token_stream().to_string() != $new.to_token_stream().to_string() {
                return Err(format!("incompatible envelope: {}", $identity));
            }
            for member in members {
                if $old.items.iter().any(|m| match (m, &member) {
                    (syn::$kind::Fn(a), syn::$kind::Fn(b)) => a.sig.ident == b.sig.ident,
                    (syn::$kind::Const(a), syn::$kind::Const(b)) => a.ident == b.ident,
                    (syn::$kind::Type(a), syn::$kind::Type(b)) => a.ident == b.ident,
                    _ => m.to_token_stream().to_string() == member.to_token_stream().to_string(),
                }) {
                    return Err(format!("duplicate member in {}", $identity));
                }
                $old.items.push(member);
            }
        }};
    }
    for item in input {
        let identity = key(&item);
        let position = if let Some(position) = out.iter().position(|old| key(old) == identity) {
            position
        } else {
            let mut header = item.clone();
            match &mut header {
                Item::Trait(i) => i.items.clear(),
                Item::Impl(i) => i.items.clear(),
                Item::Mod(i) if i.content.is_some() => i.content.as_mut().unwrap().1.clear(),
                _ => {
                    out.push(item);
                    continue;
                }
            }
            out.push(header);
            out.len() - 1
        };
        match (&mut out[position], item) {
            (Item::Trait(old), Item::Trait(mut new)) => members!(old, new, TraitItem, identity),
            (Item::Impl(old), Item::Impl(mut new)) => members!(old, new, ImplItem, identity),
            (Item::Mod(old), Item::Mod(mut new))
                if old.content.is_some() && new.content.is_some() =>
            {
                let (_, members) = new.content.take().unwrap();
                let mut header = old.clone();
                header.content = None;
                if header.to_token_stream().to_string() != new.to_token_stream().to_string() {
                    return Err(format!("incompatible envelope: {identity}"));
                }
                merge(&mut old.content.as_mut().unwrap().1, members)?;
            }
            _ => return Err(format!("duplicate item: {identity}")),
        }
    }
    Ok(())
}

pub(crate) fn source(root: &Path) -> Result<String, String> {
    let dir = root.join(DIRECTORY);
    let mut items = Vec::new();
    for kind in Catalog::modules(&dir).map_err(|e| e.to_string())? {
        let path = dir.join(format!("{kind}.rs"));
        let source = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let file = syn::parse_file(&source).map_err(|e| format!("{}: {e}", path.display()))?;
        merge(&mut items, file.items)?;
    }
    Ok(prettyplease::unparse(&syn::File {
        shebang: None,
        attrs: Vec::new(),
        items,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_envelopes_merge_and_reject_drift() {
        let parse = |s: &str| syn::parse_file(s).unwrap().items;
        let mut items = parse(
            "pub trait R { fn block(&self); } impl R for P { fn block(&self) {} } pub mod tests { fn block() {} }",
        );
        merge(&mut items, parse("pub trait R { fn item(&self); } impl R for P { fn item(&self) {} } pub mod tests { fn item() {} }")).unwrap();
        assert_eq!(items.len(), 3);
        let Item::Trait(r) = &items[0] else { panic!() };
        assert_eq!(r.items.len(), 2);
        for bad in [
            "pub trait R { fn block(&self); }",
            "pub unsafe trait R { fn fluid(&self); }",
            "impl R for P { fn item(&self) {} }",
            "impl R for P where P: Copy { fn fluid(&self) {} }",
            "pub trait S { fn same(&self); fn same(&self, x: u32); }",
            "impl P { const SAME: u32 = 0; const SAME: u32 = 1; }",
            "pub mod tests { fn block() {} }",
            "mod tests { fn fluid() {} }",
        ] {
            assert!(merge(&mut items.clone(), parse(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn parts_root_and_shared_fixtures_stay_support_only() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let api = root.join("crates/horizon-mod-api/src");
        let source = fs::read_to_string(api.join("part_table.rs")).unwrap();
        let file = syn::parse_file(&source).unwrap();
        assert!(
            file.items
                .iter()
                .all(|i| matches!(i, Item::Use(_) | Item::Macro(_)))
        );
        assert!(source.contains(OUTPUT));
        assert!(!api.join("part_table/mod.rs").exists());
        let source = fs::read_to_string(api.join("part_table/shared.rs")).unwrap();
        assert!(source.starts_with("//! Shared table state, cross-kind registration"));
        let file = syn::parse_file(&source).unwrap();
        let fixtures = file
            .items
            .iter()
            .find_map(|i| match i {
                Item::Mod(i) if i.ident == "tests" => i.content.as_ref(),
                _ => None,
            })
            .unwrap();
        for item in &fixtures.1 {
            if let Item::Fn(f) = item {
                assert!(
                    [
                        "register_game_keys",
                        "block_key",
                        "effect_key",
                        "type_key",
                        "here",
                        "on",
                        "registry_keys_reject_unknown_unattached_requests_with_the_requesting_mod",
                        "invalid_lookup_ids_refuse_registration_naming_the_requesting_mod",
                        "a_host_without_a_backend_refuses_handle_calls",
                        "open",
                        "close",
                        "game_thread",
                    ]
                    .contains(&f.sig.ident.to_string().as_str()),
                    "kind-specific fixture in shared: {}",
                    f.sig.ident
                );
            }
        }
    }

    #[test]
    fn a_new_parts_kind_needs_no_shared_list() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../target/parts-discovery-{}",
            std::process::id()
        ));
        let dir = root.join(DIRECTORY);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("zeta.rs"),
            "impl P { fn zeta() { if true { return; } if false { return; } } }",
        )
        .unwrap();
        assert!(!source(&root).unwrap().contains("} if "));
        fs::write(dir.join("alpha.rs"), "impl P { fn alpha() {} }").unwrap();
        let added = source(&root).unwrap();
        assert!(added.find("alpha") < added.find("zeta"));
        fs::remove_file(dir.join("alpha.rs")).unwrap();
        assert!(!source(&root).unwrap().contains("alpha"));
        fs::remove_dir_all(root).unwrap();
    }
}
