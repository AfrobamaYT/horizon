//! The game's side of the `client_pause_read` area (`horizon-mod-api/src/host/client_pause_read.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl ClientPauseReadClientHost for Game<'_> {
    fn is_paused(&self) -> bool {
        mod_api::__private::check_access("is_paused", mod_api::__private::LoadAccess::ClientSetup);
        self.native_is_paused()
    }
}
