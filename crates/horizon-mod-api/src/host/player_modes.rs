//! The `player_modes` area: host methods the game implements (`game/host/player_modes.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `player_modes` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait PlayerModesBackend {
    /// Side: server.
    /// The player's permission level, 0 to 4 (`Player.permissions()`,
    /// `PlayerList.getProfilePermissions`): an op's level, the
    /// single-player owner's 4 with commands allowed. A displaced connected
    /// player keeps their permission level even while their body is unavailable.
    /// None for a stale or unavailable player.
    fn player_permission_level(&self, player: PlayerRef) -> Option<u8> {
        crate::phase::unprovided(
            "Backend::player_permission_level",
            "player_permission_level",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Player.hasInfiniteMaterials`: the `instabuild` ability, which
    /// creative mode gives.
    fn player_has_infinite_materials(&self, player: PlayerRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_has_infinite_materials",
            "player_has_infinite_materials",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Player.canUseGameMasterBlocks`: creative and a permission level of
    /// at least 2, as command blocks, structure blocks and jigsaws ask.
    fn player_can_use_game_master_blocks(&self, player: PlayerRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_can_use_game_master_blocks",
            "player_can_use_game_master_blocks",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ServerPlayerGameMode.getPreviousGameModeForPlayer`: the mode the
    /// player last left (`previousPlayerGameType`, saved with the player), or
    /// `Some(None)` when they never changed mode. None for a stale or
    /// unavailable player.
    fn player_previous_game_mode(&self, player: PlayerRef) -> Option<Option<GameMode>> {
        crate::phase::unprovided(
            "Backend::player_previous_game_mode",
            "player_previous_game_mode",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Player.isSecondaryUseActive`: whether the player uses items and
    /// blocks as with sneak held, as vanilla reads it from the shift-key
    /// flag (the same read as `EntityRef::is_shift_key_down`). None for a
    /// stale or unavailable player.
    fn player_is_secondary_use_active(&self, player: PlayerRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_is_secondary_use_active",
            "player_is_secondary_use_active",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
