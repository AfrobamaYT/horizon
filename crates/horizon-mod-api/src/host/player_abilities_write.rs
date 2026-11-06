//! The `player_abilities_write` area: host methods the game implements (`game/host/player_abilities_write.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' server writes of the `player_abilities_write` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait PlayerAbilitiesWriteBackendMut {
    /// Side: server.
    /// Stores a server player's `Abilities` and tells their client
    /// (`ServerPlayer.onUpdateAbilities`). They stay as written until the
    /// player's game mode changes (`GameType.updatePlayerAbilities`), and are
    /// saved with the player. A speed that is not finite refuses the write
    /// (vanilla's setters take any float). Returns false for a
    /// stale or unavailable player, a displaced outer player, and a speed
    /// that is not finite.
    fn player_set_abilities(
        &mut self,
        player: PlayerRef,
        abilities: NativePlayerAbilities,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_abilities",
            "player_set_abilities",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
