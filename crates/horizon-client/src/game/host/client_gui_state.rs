//! The game's side of the `client_gui_state` area (`horizon-mod-api/src/host/client_gui_state.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl ClientGuiStateClientHost for Game<'_> {
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
}
