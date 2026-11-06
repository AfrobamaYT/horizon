//! The `player_xp_give` area: host methods the game implements (`game/host/player_xp_give.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' server writes of the `player_xp_give` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait PlayerXpGiveBackendMut {
    /// Side: server.
    /// `ExperienceOrb.playerTouch`: Mending repairs the player's damaged
    /// items with `points` first (`ExperienceOrb.repairPlayerItems`), then
    /// only the rest goes to their experience
    /// (`Player.giveExperiencePoints`), with the level-up sound. No orb
    /// pickup sound. False, and nothing given, as the other writes; `points`
    /// above `i32::MAX` is refused before Mending repairs anything.
    fn player_give_experience_points(&mut self, player: PlayerRef, points: u32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_give_experience_points",
            "player_give_experience_points",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Player.giveExperienceLevels`: adds `levels`, or takes them off when
    /// negative, as the anvil and the enchanting table charge with. Below 0
    /// the level, the total and the progress toward the next level are all 0.
    fn player_give_levels(&mut self, player: PlayerRef, levels: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_give_levels",
            "player_give_levels",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
