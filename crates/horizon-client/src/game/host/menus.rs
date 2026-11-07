//! The game's side of the `menus` area (`horizon-mod-api/src/host/menus.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl MenusServerHost for Game<'_> {
    fn open_menu(
        &mut self,
        uuid: u128,
        rows: u8,
        title: Text,
        items: Vec<Option<ItemStack>>,
    ) -> Option<i32> {
        mod_api::__private::check_access("open_menu", mod_api::__private::LoadAccess::WorldOnly);
        self.native_open_menu(uuid, rows, title, items)
    }
    fn close_menu(&mut self, uuid: u128) {
        mod_api::__private::check_access("close_menu", mod_api::__private::LoadAccess::WorldOnly);
        self.native_close_menu(uuid)
    }
}
impl MenusClientHost for Game<'_> {
    fn local_player(&self) -> Option<PlayerRef> {
        mod_api::__private::check_access(
            "local_player",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_local_player()
    }
}
impl MenusBackend for Game<'_> {
    fn menu_container_id(&self, menu: MenuRef) -> Option<i32> {
        mod_api::__private::check_access(
            "menu_container_id",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_menu_container_id(menu)
    }
    fn menu_kind(&self, menu: MenuRef) -> Option<Option<String>> {
        mod_api::__private::check_access("menu_kind", mod_api::__private::LoadAccess::WorldOnly);
        self.native_menu_kind(menu)
    }
    fn menu_slot_count(&self, menu: MenuRef) -> Option<usize> {
        mod_api::__private::check_access(
            "menu_slot_count",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_menu_slot_count(menu)
    }
    fn menu_slot(&self, menu: MenuRef, slot: usize) -> Option<Option<ItemStack>> {
        mod_api::__private::check_access("menu_slot", mod_api::__private::LoadAccess::WorldOnly);
        self.native_menu_slot(menu, slot)
    }
    fn menu_carried(&self, menu: MenuRef) -> Option<Option<ItemStack>> {
        mod_api::__private::check_access("menu_carried", mod_api::__private::LoadAccess::WorldOnly);
        self.native_menu_carried(menu)
    }
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
impl MenusClientBackendMut for Game<'_> {
    fn menu_click(&mut self, menu: MenuRef, clicks: &[MenuClick]) -> bool {
        mod_api::__private::check_access("menu_click", mod_api::__private::LoadAccess::WorldOnly);
        self.native_menu_click(menu, clicks)
    }
    fn menu_close(&mut self, menu: MenuRef) -> bool {
        mod_api::__private::check_access("menu_close", mod_api::__private::LoadAccess::WorldOnly);
        self.native_menu_close(menu)
    }
    fn menu_click_button(&mut self, menu: MenuRef, button: i32) -> bool {
        mod_api::__private::check_access(
            "menu_click_button",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_menu_click_button(menu, button)
    }
}
