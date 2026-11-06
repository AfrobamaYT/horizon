//! The `enchantments` area: host methods the game implements (`game/host/enchantments.rs`).
#![allow(unused_variables)]
use crate::*;
/// The client host's methods of the `enchantments` area, implemented by the game and reached through [`ClientHost`].
pub trait EnchantmentsClientHost {
    /// Side: client.
    /// Registered enchantments, in registry order.
    fn enchantments(&self) -> Vec<EnchantmentInfo> {
        crate::phase::unprovided(
            "ClientHost::enchantments",
            "enchantments",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// `Enchantment.getMinCost(level)`: the enchanting power needed, or None for an unknown id.
    fn enchantment_min_cost(&self, id: &str, level: u32) -> Option<i32> {
        crate::phase::unprovided(
            "ClientHost::enchantment_min_cost",
            "enchantment_min_cost",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// Whether enchantment `id` can go on `item` (`Enchantment.isSupportedItem`).
    fn enchantment_supports(&self, id: &str, item: &str) -> bool {
        crate::phase::unprovided(
            "ClientHost::enchantment_supports",
            "enchantment_supports",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
}
