//! The game's side of the `screen_hovered_item` area (`horizon-mod-api/src/host/screen_hovered_item.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl ScreenHoveredItemClientHost for Game<'_> {
    fn screen_hovered_item(&self) -> Option<String> {
        mod_api::__private::check_access(
            "screen_hovered_item",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_screen_hovered_item()
    }
}
