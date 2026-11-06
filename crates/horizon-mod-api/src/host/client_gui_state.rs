//! The `client_gui_state` area: host methods the game implements (`game/host/client_gui_state.rs`).
#![allow(unused_variables)]
use crate::*;
/// The client host's methods of the `client_gui_state` area, implemented by the game and reached through [`ClientHost`].
pub trait ClientGuiStateClientHost {
    /// Side: client.
    /// The last window focus state reported by the window system.
    fn window_focused(&self) -> bool {
        crate::phase::unprovided(
            "ClientHost::window_focused",
            "window_focused",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// The same screen identifier provided to screen hooks.
    fn observed_screen_id(&self) -> Option<ScreenId> {
        crate::phase::unprovided(
            "ClientHost::observed_screen_id",
            "observed_screen_id",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
}
