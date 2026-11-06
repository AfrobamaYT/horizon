//! The game's side of the `client_session` area (`horizon-mod-api/src/host/client_session.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl ClientSessionClientBackend for Game<'_> {
    fn player_block_breaking(&self, player: PlayerRef) -> Option<BlockBreakingState> {
        mod_api::__private::check_access(
            "player_block_breaking",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_block_breaking(player)
    }
    fn player_ambient_mood(&self, player: PlayerRef) -> Option<f32> {
        mod_api::__private::check_access(
            "player_ambient_mood",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_ambient_mood(player)
    }
}
