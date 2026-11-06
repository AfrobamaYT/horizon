//! The game's side of the `screens` area (`horizon-mod-api/src/host/screens.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl ScreensClientHost for Game<'_> {
    fn mod_screen_open(&mut self, mod_id: &str) -> bool {
        mod_api::__private::check_access(
            "mod_screen_open",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mod_screen_open(mod_id)
    }
    fn mod_screen_close(&mut self, mod_id: &str) {
        mod_api::__private::check_access(
            "mod_screen_close",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mod_screen_close(mod_id)
    }
    fn mod_screen(&self) -> Option<&str> {
        mod_api::__private::check_access("mod_screen", mod_api::__private::LoadAccess::ClientSetup);
        self.native_mod_screen()
    }
    fn screen_title(&self) -> Option<String> {
        mod_api::__private::check_access(
            "screen_title",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_screen_title()
    }
    fn window_focused(&self) -> bool {
        mod_api::__private::check_access(
            "window_focused",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_window_focused()
    }
    fn observed_screen_id(&self) -> Option<ScreenId> {
        mod_api::__private::check_access(
            "observed_screen_id",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_observed_screen_id()
    }
    fn screen_pos(&self) -> Option<[i32; 3]> {
        mod_api::__private::check_access("screen_pos", mod_api::__private::LoadAccess::ClientSetup);
        self.native_screen_pos()
    }
    fn screen_extra_data(&self) -> Option<&[u8]> {
        mod_api::__private::check_access(
            "screen_extra_data",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_screen_extra_data()
    }
    fn screen_exclusion(&self) -> Vec<[f32; 4]> {
        mod_api::__private::check_access(
            "screen_exclusion",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_screen_exclusion()
    }
    fn screen_hovered_item(&self) -> Option<String> {
        mod_api::__private::check_access(
            "screen_hovered_item",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_screen_hovered_item()
    }
    fn apply_screen_blur(&self) -> bool {
        mod_api::__private::check_access(
            "apply_screen_blur",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_apply_screen_blur()
    }
    fn menu_blur_radius(&self) -> f32 {
        mod_api::__private::check_access(
            "menu_blur_radius",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_menu_blur_radius()
    }
    fn is_key_down(&self, key: &str) -> bool {
        mod_api::__private::check_access(
            "is_key_down",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_is_key_down(key)
    }
}
