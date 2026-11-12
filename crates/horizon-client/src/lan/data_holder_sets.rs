//! A synced registry entry carries its holder sets in vanilla's forms: a pack
//! enchantment's NeoForge set objects (`neoforge:or`...) are resolved to the
//! id list a vanilla client decodes.
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// The enchantment fields vanilla types as holder sets, with their registry.
const FIELDS: [(&str, &str); 3] = [
    ("supported_items", "item"),
    ("primary_items", "item"),
    ("exclusive_set", "enchantment"),
];

type Entries = Vec<((String, String), Value, bool)>;

/// Replace every set object in `entries`' holder-set fields with its ids.
/// Entries with none are untouched, and no tag is read for them.
pub(super) fn expand_enchantments(
    entries: &mut Entries,
    resources: &std::path::Path,
    overlay: &horizon_world::loot::PackOverlay,
) -> Result<(), String> {
    let has_object = |v: &Value| FIELDS.iter().any(|(field, _)| v[*field].is_object());
    if !entries.iter().any(|(_, v, _)| has_object(v)) {
        return Ok(());
    }
    let tags = |kind| crate::items::load_tags(resources, kind, overlay).map_err(|e| e.to_string());
    let tags: HashMap<&str, HashMap<String, HashSet<String>>> = HashMap::from([
        ("item", tags("item")?),
        ("enchantment", tags("enchantment")?),
    ]);
    let items: Vec<String> = std::fs::read(resources.join("command-registries.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|v| serde_json::from_value(v["items"].clone()).ok())
        .unwrap_or_default();
    let enchantments: Vec<String> = entries
        .iter()
        .map(|((ns, path), _, _)| format!("{ns}:{path}"))
        .collect();
    for ((ns, path), value, _) in entries.iter_mut() {
        for (field, registry) in FIELDS {
            if !value[field].is_object() {
                continue;
            }
            let ids = || match registry {
                "item" => items.clone(),
                _ => enchantments.clone(),
            };
            let tag = |tag: &str| -> Result<Vec<String>, String> {
                let tag = crate::crafting::qualified(tag);
                Ok(tags[registry]
                    .get(&tag)
                    .map(|members| members.iter().cloned().collect())
                    .unwrap_or_default())
            };
            let lookup = horizon_world::holder_set::StringLookup {
                what: registry,
                ids: Some(&ids),
                tag: &tag,
                entry: &|id| Ok(vec![crate::crafting::qualified(id)]),
            };
            let resolved = horizon_world::holder_set::resolve(&value[field], &lookup)
                .map_err(|e| format!("enchantment {ns}:{path} {field}: {e}"))?;
            value[field] = Value::Array(resolved.into_iter().map(Value::String).collect());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_set_object_reaches_clients_as_ids_and_plain_forms_are_untouched() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("command-registries.json"),
            json!({"items": ["minecraft:stick", "minecraft:iron_sword"]}).to_string(),
        )
        .unwrap();
        let files = vec![(
            "tags/minecraft:item/swords".to_owned(),
            vec![
                json!({"values": ["minecraft:iron_sword"]})
                    .to_string()
                    .into_bytes(),
            ],
        )];
        let overlay = horizon_world::loot::PackOverlay {
            conditions: &horizon_world::conditions::LoadConditions::vanilla(),
            files: &files,
        };
        let mut entries = vec![
            (
                ("test".to_owned(), "sticky".to_owned()),
                json!({"supported_items": {"type": "neoforge:or",
                    "values": ["#minecraft:swords", "minecraft:stick"]}}),
                true,
            ),
            (
                ("test".to_owned(), "plain".to_owned()),
                json!({"supported_items": "#minecraft:swords"}),
                true,
            ),
        ];
        expand_enchantments(&mut entries, root.path(), &overlay).unwrap();
        assert_eq!(
            entries[0].1["supported_items"],
            json!(["minecraft:iron_sword", "minecraft:stick"])
        );
        assert_eq!(entries[1].1["supported_items"], json!("#minecraft:swords"));
    }
}
