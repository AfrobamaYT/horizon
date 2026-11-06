//! The game's side of the `menu_slots` area (`horizon-mod-api/src/host/menu_slots.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl MenuSlotsBackend for Game<'_> {
    fn menu_slot_backing(&self, menu: MenuRef, slot: usize) -> Option<MenuSlotBacking> {
        mod_api::__private::check_access(
            "menu_slot_backing",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_menu_slot_backing(menu, slot)
    }
    fn menu_slot_holds(&self, menu: MenuRef, slot: usize, stack: &ItemStack) -> Option<u32> {
        mod_api::__private::check_access(
            "menu_slot_holds",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_menu_slot_holds(menu, slot, stack)
    }
    fn menu_slot_gives(&self, menu: MenuRef, slot: usize) -> Option<bool> {
        mod_api::__private::check_access(
            "menu_slot_gives",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_menu_slot_gives(menu, slot)
    }
}
