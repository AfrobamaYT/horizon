//! Moved here from the game so mods can name it.
#[allow(unused_imports)]
use crate::*;
/// One offer row as `EnchantmentLevelSetEvent` reports it: the server player
/// whose table it is (UUID and entity id), the row (0 to 2), the power (the
/// bookshelves counted), the item and the level `getEnchantmentCost` drew.
#[derive(Clone, Debug, PartialEq)]
pub struct EnchantmentRow<'a> {
    pub player: u128,
    pub player_entity_id: i32,
    pub row: i32,
    pub power: i32,
    pub item: &'a ItemStack,
    pub original_level: i32,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnvilResult {
    pub result: Option<ItemStack>,
    /// Level cost (the `cost` data slot).
    pub cost: i32,
    /// Material items consumed from the second slot on take.
    pub repair_item_count_cost: u32,
    pub only_renaming: bool,
}
