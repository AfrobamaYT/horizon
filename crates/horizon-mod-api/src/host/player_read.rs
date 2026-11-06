//! The `player_read` area: host methods the game implements (`game/host/player_read.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `player_read` area, implemented by the game and reached through [`Host`].
pub trait PlayerReadHost {
    /// Side: both.
    /// Resolve a real online player by UUID, never a fake. A fake is reached
    /// through the handle returned by its factory. During a nested player swap,
    /// outer players whose records are displaced return None.
    fn player(&self, uuid: u128) -> Option<PlayerRef> {
        crate::phase::unprovided(
            "Host::player",
            "player",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The server host's methods of the `player_read` area, implemented by the game and reached through [`ServerHost`].
pub trait PlayerReadServerHost {
    /// Side: server.
    /// Create or retrieve a connectionless player for the level currently in place.
    fn fake_player(&mut self, level: LevelRef, uuid: u128, name: &str) -> Option<PlayerRef> {
        crate::phase::unprovided(
            "ServerHost::fake_player",
            "fake_player",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' reads of the `player_read` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait PlayerReadBackend {
    /// Side: server.
    /// Whether a live player handle names a connectionless actor.
    fn player_is_fake(&self, player: PlayerRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_is_fake",
            "player_is_fake",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Inventory.items`: the non-empty main inventory stacks in slot order, including
    /// their components. None for a stale or unavailable player; on clients only the
    /// local player's inventory is available.
    fn player_inventory(&self, player: PlayerRef) -> Option<Vec<ItemStack>> {
        crate::phase::unprovided(
            "Backend::player_inventory",
            "player_inventory",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `ServerPlayerGameMode.getGameModeForPlayer`; Paper `HumanEntity#getGameMode`.
    /// None for stale, offline or unavailable players (including displaced outer players).
    fn player_game_mode(&self, player: PlayerRef) -> Option<GameMode> {
        crate::phase::unprovided(
            "Backend::player_game_mode",
            "player_game_mode",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Player.isCreative`; Paper `HumanEntity#getGameMode`.
    fn player_is_creative(&self, player: PlayerRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_is_creative",
            "player_is_creative",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Player.isSpectator`; Paper `HumanEntity#getGameMode`.
    fn player_is_spectator(&self, player: PlayerRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_is_spectator",
            "player_is_spectator",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Player.mayBuild`; Paper `HumanEntity#getGameMode`.
    fn player_may_build(&self, player: PlayerRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_may_build",
            "player_may_build",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `FoodData.getFoodLevel/getSaturationLevel/getExhaustionLevel`; Paper
    /// `HumanEntity#getFoodLevel/getSaturation/getExhaustion`. None until the
    /// guest's body exists, as well as for stale or unavailable players.
    fn player_food(&self, player: PlayerRef) -> Option<NativePlayerFood> {
        crate::phase::unprovided(
            "Backend::player_food",
            "player_food",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Player.getAbilities`; Paper `Player#getAllowFlight/isFlying/getFlySpeed/getWalkSpeed`.
    fn player_abilities(&self, player: PlayerRef) -> Option<NativePlayerAbilities> {
        crate::phase::unprovided(
            "Backend::player_abilities",
            "player_abilities",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `player_read` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait PlayerReadBackendMut {
    /// Side: server.
    /// Remove a fake without treating it as a disconnect or saving it.
    fn player_remove_fake(&mut self, player: PlayerRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_remove_fake",
            "player_remove_fake",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
