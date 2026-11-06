//! The game's side of the `menu_refs` area (`horizon-mod-api/src/host/menu_refs.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl MenuRefsClientHost for Game<'_> {
    fn local_player(&self) -> Option<PlayerRef> {
        mod_api::__private::check_access(
            "local_player",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_local_player()
    }
}
impl MenuRefsBackend for Game<'_> {
    fn player_container_menu(&self, player: PlayerRef) -> Option<MenuRef> {
        mod_api::__private::check_access(
            "player_container_menu",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_menu_ref(player)
    }
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
}
impl MenuRefsClientBackendMut for Game<'_> {
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
