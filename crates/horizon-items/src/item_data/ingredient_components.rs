//! Borrowed live component queries for decoded ingredient predicates.
use super::*;

/// Visit present component names without serializing or allocating.
pub fn visit_components(stack: &ItemStack, mut visit: impl FnMut(&str)) {
    for id in MODELLED {
        if stack.carries_component(None, id) {
            visit(id);
        }
    }
    for id in [
        "minecraft:item_name",
        "minecraft:rarity",
        "minecraft:tooltip_display",
    ] {
        if stack.carries_component(None, id) {
            visit(id);
        }
    }
    if let Some(data) = stack.data() {
        for (id, _) in &data.other {
            if !id.starts_with('!') && !MODELLED.contains(&id.as_str()) {
                visit(id);
            }
        }
    }
}

/// Whether a patch explicitly removed this component, without building a prefixed key.
pub fn removed(stack: &ItemStack, id: &str) -> bool {
    !stack.carries_component(None, id)
        && stack.data().is_some_and(|data| {
            data.other
                .iter()
                .any(|(key, _)| key.strip_prefix('!') == Some(id))
        })
}

/// Equality in the decoded representation used by the shared component codecs.
/// Presence is checked by the caller, so empty decoded enchantment maps compare equal.
pub fn component_equal(id: &str, a: &ItemStack, b: &ItemStack) -> bool {
    match id {
        "minecraft:damage" => a.damage == b.damage,
        "minecraft:repair_cost" => {
            a.data().map_or(0, |d| d.repair_cost) == b.data().map_or(0, |d| d.repair_cost)
        }
        "minecraft:enchantments" => levels_equal(
            a.data().map_or(&[], |d| d.enchantments.as_slice()),
            b.data().map_or(&[], |d| d.enchantments.as_slice()),
        ),
        "minecraft:stored_enchantments" => levels_equal(
            a.data().map_or(&[], |d| d.stored_enchantments.as_slice()),
            b.data().map_or(&[], |d| d.stored_enchantments.as_slice()),
        ),
        "minecraft:custom_name" => {
            a.data().and_then(|d| d.custom_name.as_ref())
                == b.data().and_then(|d| d.custom_name.as_ref())
        }
        "minecraft:max_damage" | "minecraft:max_stack_size" => {
            a.data()
                .and_then(|d| d.other(id))
                .and_then(crate::properties::component_int)
                == b.data()
                    .and_then(|d| d.other(id))
                    .and_then(crate::properties::component_int)
        }
        "minecraft:block_entity_data" => block_entity_equal(
            a.data().and_then(|d| d.other(id)),
            b.data().and_then(|d| d.other(id)),
        ),
        "minecraft:item_name" => item_name_equal(a, b),
        "minecraft:rarity" => rarity(a) == rarity(b),
        _ => a.data().and_then(|d| d.other(id)) == b.data().and_then(|d| d.other(id)),
    }
}

fn java_is(value: &JavaString, text: &str) -> bool {
    value.0.iter().copied().eq(text.encode_utf16())
}
fn implicit_ominous_name(stack: &ItemStack) -> bool {
    stack
        .data()
        .is_some_and(|data| data.ominous_banner && data.other("minecraft:item_name").is_none())
}
fn is_ominous_name(tag: Option<&Tag>) -> bool {
    let Some(Tag::Compound(map)) = tag else {
        return false;
    };
    let mut translate = false;
    map.iter().all(|(key, value)| {
        if java_is(key, "translate") {
            translate = matches!(value, Tag::String(value) if java_is(value, "block.minecraft.ominous_banner"));
            translate
        } else if java_is(key, "type") {
            matches!(value, Tag::String(value) if java_is(value, "translatable"))
        } else if java_is(key, "extra") || java_is(key, "with") {
            matches!(value, Tag::List(values) if values.is_empty())
        } else { false }
    }) && translate
}
fn item_name_equal(a: &ItemStack, b: &ItemStack) -> bool {
    let a_name = a.data().and_then(|data| data.other("minecraft:item_name"));
    let b_name = b.data().and_then(|data| data.other("minecraft:item_name"));
    match (implicit_ominous_name(a), implicit_ominous_name(b)) {
        (true, true) => true,
        (true, false) => is_ominous_name(b_name),
        (false, true) => is_ominous_name(a_name),
        (false, false) => a_name.map(text_value) == b_name.map(text_value),
    }
}
fn rarity(stack: &ItemStack) -> Option<u8> {
    let data = stack.data()?;
    match data.other("minecraft:rarity") {
        Some(Tag::String(value)) => ["common", "uncommon", "rare", "epic"]
            .iter()
            .position(|text| java_is(value, text))
            .map(|index| index as u8),
        Some(_) => None,
        None => data.ominous_banner.then_some(1),
    }
}

fn block_entity_equal(a: Option<&Tag>, b: Option<&Tag>) -> bool {
    let (Some(Tag::Compound(a)), Some(Tag::Compound(b))) = (a, b) else {
        return a == b;
    };
    if a.len() != b.len() {
        return false;
    }
    a.iter().all(|(key, value)| {
        let Some(other) = b.get(key) else {
            return false;
        };
        if key.0.iter().copied().eq("id".encode_utf16()) {
            let (Tag::String(a), Tag::String(b)) = (value, other) else {
                return false;
            };
            let a_prefix = if a.0.contains(&u16::from(b':')) {
                ""
            } else {
                "minecraft:"
            };
            let b_prefix = if b.0.contains(&u16::from(b':')) {
                ""
            } else {
                "minecraft:"
            };
            a_prefix
                .encode_utf16()
                .chain(a.0.iter().copied())
                .eq(b_prefix.encode_utf16().chain(b.0.iter().copied()))
        } else {
            value == other
        }
    })
}

fn levels_equal(a: &[(Arc<str>, u32)], b: &[(Arc<str>, u32)]) -> bool {
    a.len() == b.len() && a.iter().all(|entry| b.contains(entry))
}
// Borrow the same plain-literal normalization as collapsed_text, without cloning a tag.
fn text_value(tag: &Tag) -> &Tag {
    let Tag::Compound(map) = tag else { return tag };
    let key_is = |key: &JavaString, value: &str| key.0.iter().copied().eq(value.encode_utf16());
    let Some((_, text @ Tag::String(_))) = map.iter().find(|(key, _)| key_is(key, "text")) else {
        return tag;
    };
    if map.iter().all(|(key, value)| {
        key_is(key, "text")
            || key_is(key, "type") && matches!(value, Tag::String(value) if key_is(value, "text"))
            || key_is(key, "extra") && matches!(value, Tag::List(values) if values.is_empty())
    }) {
        text
    } else {
        tag
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decoded_component_equality_borrows_maps_and_plain_text() {
        let mut a = ItemStack::new("minecraft:stone", 1);
        let mut b = ItemStack::new("minecraft:stone", 1);
        a.data_mut().enchantments = vec![
            ("minecraft:sharpness".into(), 1),
            ("minecraft:unbreaking".into(), 2),
        ];
        b.data_mut().enchantments = vec![
            ("minecraft:unbreaking".into(), 2),
            ("minecraft:sharpness".into(), 1),
        ];
        assert!(component_equal("minecraft:enchantments", &a, &b));
        a.data_mut()
            .set_other("minecraft:item_name", Some(Tag::String("Named".into())));
        b.data_mut().set_other(
            "minecraft:item_name",
            Some(Tag::compound([("text", Tag::String("Named".into()))])),
        );
        assert!(component_equal("minecraft:item_name", &a, &b));
        b.data_mut().enchantments[0].1 = 3;
        assert!(!component_equal("minecraft:enchantments", &a, &b));
    }
}
