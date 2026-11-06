//! The `player_game_mode_write` area: host methods the game implements (`game/host/player_game_mode_write.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' server writes of the `player_game_mode_write` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait PlayerGameModeWriteBackendMut {
    /// Side: server.
    /// Changes a player's game mode as `ServerPlayer.setGameMode` and
    /// `/gamemode` do, including the cancellable change hook and ability
    /// updates. Returns false for a stale or unavailable player, an active
    /// outer guest, no change, or a canceled change.
    fn player_set_game_mode(&mut self, player: PlayerRef, mode: GameMode) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_game_mode",
            "player_set_game_mode",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
