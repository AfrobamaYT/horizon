//! The game's side of the `screen_transparent_background` area (`horizon-mod-api/src/host/screen_transparent_background.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl ScreenTransparentBackgroundClientHost for Game<'_> {
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
}
