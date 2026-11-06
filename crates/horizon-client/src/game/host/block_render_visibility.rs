//! The game's side of the `block_render_visibility` area (`horizon-mod-api/src/host/block_render_visibility.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl BlockRenderVisibilityClientHost for Game<'_> {
    fn refresh_render_visibility(&mut self) {
        mod_api::__private::check_access(
            "refresh_render_visibility",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_refresh_render_visibility()
    }
}
