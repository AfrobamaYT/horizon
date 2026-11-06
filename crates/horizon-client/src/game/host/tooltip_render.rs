//! The game's side of the `tooltip_render` area (`horizon-mod-api/src/host/tooltip_render.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl TooltipRenderClientHost for Game<'_> {
    fn is_key_down(&self, key: &str) -> bool {
        mod_api::__private::check_access(
            "is_key_down",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_is_key_down(key)
    }
}
