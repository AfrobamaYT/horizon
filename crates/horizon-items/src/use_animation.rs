//! The use animation a `consumable` component names.

use horizon_world::consumable::UseAnimation;

/// `ItemUseAnimation.CODEC`'s names; a consumable with none eats.
pub fn consumable_animation(v: &serde_json::Value) -> UseAnimation {
    match v
        .get("animation")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("eat")
    {
        "eat" => UseAnimation::Eat,
        "drink" => UseAnimation::Drink,
        "block" => UseAnimation::Block,
        "bow" => UseAnimation::Bow,
        "trident" => UseAnimation::Trident,
        "crossbow" => UseAnimation::Crossbow,
        "spyglass" => UseAnimation::Spyglass,
        "toot_horn" => UseAnimation::TootHorn,
        "brush" => UseAnimation::Brush,
        "bundle" => UseAnimation::Bundle,
        "spear" => UseAnimation::Spear,
        _ => UseAnimation::None,
    }
}
