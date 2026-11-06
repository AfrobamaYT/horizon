//! The `menu_slots` area: host methods the game implements (`game/host/menu_slots.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `menu_slots` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait MenuSlotsBackend {
    /// Side: server.
    /// The storage menu slot `slot` shows (`Slot.container` and `getContainerSlot`): the
    /// player's inventory by `ENTITY_ITEMS` index (`InventoryMenu`'s armour slots 5..9 go head
    /// to feet), a block entity's or an entity's slot, the ender chest's, the crafting grid's
    /// or a mod's chest menu's.
    fn menu_slot_backing(&self, menu: MenuRef, slot: usize) -> Option<MenuSlotBacking> {
        crate::phase::unprovided(
            "Backend::menu_slot_backing",
            "menu_slot_backing",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// How many of `stack` menu slot `slot` holds (`Slot.mayPlace`, then `Slot.getMaxStackSize`):
    /// a furnace's fuel slot one bucket, an armour slot one piece that fits it, a crafting result
    /// none.
    fn menu_slot_holds(&self, menu: MenuRef, slot: usize, stack: &ItemStack) -> Option<u32> {
        crate::phase::unprovided(
            "Backend::menu_slot_holds",
            "menu_slot_holds",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether menu slot `slot` gives up what it holds (`Slot.mayPickup`): bound armour stays
    /// on outside creative.
    fn menu_slot_gives(&self, menu: MenuRef, slot: usize) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::menu_slot_gives",
            "menu_slot_gives",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
