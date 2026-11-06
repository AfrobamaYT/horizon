//! The `player_death_location` area: host methods the game implements (`game/host/player_death_location.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `player_death_location` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait PlayerDeathLocationBackend {
    /// Side: both.
    /// The dimension and block of the player's last death
    /// (`Player.getLastDeathLocation`).
    fn player_last_death_location(&self, player: PlayerRef) -> Option<GlobalPos> {
        crate::phase::unprovided(
            "Backend::player_last_death_location",
            "player_last_death_location",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `player_death_location` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait PlayerDeathLocationBackendMut {
    /// Side: server.
    /// `Player.setLastDeathLocation`: stores where the player died, or
    /// forgets it for None. The client hears of it with the player's next
    /// respawn, as in vanilla. False in the cases a read gives None.
    fn player_set_last_death_location(
        &mut self,
        player: PlayerRef,
        location: Option<GlobalPos>,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_last_death_location",
            "player_set_last_death_location",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
