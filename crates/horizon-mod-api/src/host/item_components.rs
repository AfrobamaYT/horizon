//! The `item_components` area: host methods the game implements (`game/host/item_components.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `item_components` area, implemented by the game and reached through [`Host`].
pub trait ItemComponentsHost {
    /// Side: both.
    /// Item `item`'s default value of component `component`, in its prototype
    /// (`Item.components().get`): `minecraft:max_damage` on an iron sword is `250`, data
    /// packs' and native mods' changes included. Read from the item report's JSON, so a
    /// number is an `Int`, `Long` or `Double` and a boolean a `Byte`. None where the item
    /// hasn't the component, and for an unknown item.
    fn item_default_component(&self, item: &str, component: &str) -> Option<Tag> {
        crate::phase::unprovided(
            "Host::item_default_component",
            "item_default_component",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Item `item`'s cooking-fuel burn time in ticks (`FuelValues.burnDuration` of its `cooking_fuel`
    /// component, the data pack's context int provider resolved), None where it is no fuel. API
    /// @API:item_components@.
    fn item_burn_time(&self, item: &str) -> Option<i32> {
        crate::phase::unprovided(
            "Host::item_burn_time",
            "item_burn_time",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Item `item`'s chance to raise a composter's level (its `compostable` component's layers provider
    /// resolved), None where it is not compostable. API @API:item_components@.
    fn item_compost_chance(&self, item: &str) -> Option<f32> {
        crate::phase::unprovided(
            "Host::item_compost_chance",
            "item_compost_chance",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Whether enchantments `a` and `b` (ids, `minecraft:sharpness`) may be on one item
    /// (`Enchantment.areCompatible`), by the world's enchantment definitions, data packs' included:
    /// false for the same enchantment twice or where either lists the other as exclusive; an id
    /// that names no enchantment excludes nothing.
    fn enchantments_compatible(&self, a: &str, b: &str) -> bool {
        crate::phase::unprovided(
            "Host::enchantments_compatible",
            "enchantments_compatible",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' reads of the `item_components` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait ItemComponentsBackend {
    /// Side: both.
    /// Whether an enchantment on `stack` has effect component `effect`, by the world's
    /// enchantment definitions (`EnchantmentHelper.has`), behind
    /// `ItemResource::has_enchantment_effect`, through the opaque backend. API
    /// @API:item_components@.
    fn item_has_enchantment_effect(&self, stack: &ItemStack, effect: &str) -> bool {
        crate::phase::unprovided(
            "Backend::item_has_enchantment_effect",
            "item_has_enchantment_effect",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The slot `stack` is equipped to (`Equippable.slot`): the item's `equippable` component
    /// after the stack's own patch, which may add, change or remove it. `head`, `chest`, `legs`,
    /// `feet`, `body`, `mainhand`, `offhand` or `saddle`; none where the stack is not equippable.
    fn item_equippable_slot(&self, stack: &ItemStack) -> Option<String> {
        crate::phase::unprovided(
            "Backend::item_equippable_slot",
            "item_equippable_slot",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
