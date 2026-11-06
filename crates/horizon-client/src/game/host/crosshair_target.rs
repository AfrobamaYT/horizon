//! The game's side of the `crosshair_target` area (`horizon-mod-api/src/host/crosshair_target.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl CrosshairTargetClientHost for Game<'_> {
    fn crosshair_target(&self) -> Option<NativeCrosshairTarget> {
        mod_api::__private::check_access(
            "crosshair_target",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_crosshair_target()
    }
}
