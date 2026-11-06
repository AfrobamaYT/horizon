//! The `item_container` area: host methods the game implements (`game/host/item_container.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `item_container` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait ItemContainerBackend {
    /// Side: both.
    /// `Capabilities.Item.ITEM`, read: the slots of one item of `stack`,
    /// empty ones included, kept in its `minecraft:container`. None for an
    /// item without slots, and for an empty stack.
    fn item_container_slots(&self, stack: &ItemStack) -> Option<Vec<Option<ItemStack>>> {
        crate::phase::unprovided(
            "Backend::item_container_slots",
            "item_container_slots",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `ResourceHandler.insert`: as much of `item` as fits into one item of
    /// `stack`, topping up the slots that hold the same item and components
    /// first, then the first empty ones (`SimpleContainer.addItem`'s order),
    /// no slot past its limit (99, or the item's own when less). A shulker
    /// box never goes in. The count that went in, and the single item it
    /// becomes (the item itself when none did). The stack given is not
    /// changed: shrink it by one and hand out the result, or leave the result
    /// unused to simulate. None where the item has no slots.
    fn item_container_insert(
        &self,
        stack: &ItemStack,
        item: &ItemStack,
    ) -> Option<(u32, ItemStack)> {
        crate::phase::unprovided(
            "Backend::item_container_insert",
            "item_container_insert",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `ResourceHandler.extract`: up to `max` out of slot `slot` of one item
    /// of `stack`. What came out (none when the slot is empty or past the
    /// item's slots), and the single item it becomes. The stack given is not
    /// changed, as with `native_item_container_insert`. None where the item
    /// has no slots.
    fn item_container_extract(
        &self,
        stack: &ItemStack,
        slot: usize,
        max: u32,
    ) -> Option<(Option<ItemStack>, ItemStack)> {
        crate::phase::unprovided(
            "Backend::item_container_extract",
            "item_container_extract",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
