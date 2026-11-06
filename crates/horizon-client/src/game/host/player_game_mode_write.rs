//! The game's side of the `player_game_mode_write` area (`horizon-mod-api/src/host/player_game_mode_write.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl PlayerGameModeWriteBackendMut for Game<'_> {
    fn player_set_game_mode(&mut self, player: PlayerRef, mode: GameMode) -> bool {
        mod_api::__private::check_access(
            "player_set_game_mode",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_game_mode(player, mode)
    }
}
