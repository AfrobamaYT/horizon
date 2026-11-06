//! The game's side of the `menu_extra_data` area (`horizon-mod-api/src/host/menu_extra_data.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl MenuExtraDataClientHost for Game<'_> {
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
}
