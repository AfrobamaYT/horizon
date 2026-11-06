//! The `player_food` area: host methods the game implements (`game/host/player_food.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `player_food` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait PlayerFoodBackend {
    /// Side: both.
    /// `FoodData.needsFood`: the food level is below 20.
    fn player_needs_food(&self, player: PlayerRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_needs_food",
            "player_needs_food",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `FoodData.hasEnoughFood`: the food level is above 6, enough to
    /// sprint.
    fn player_has_enough_food(&self, player: PlayerRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_has_enough_food",
            "player_has_enough_food",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Player.canEat(always)`: an invulnerable player (creative), `always`
    /// (food that can always be eaten) or a food level below 20.
    fn player_can_eat(&self, player: PlayerRef, always: bool) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_can_eat",
            "player_can_eat",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Player.experienceLevel`, `experienceProgress` and `totalExperience`.
    fn player_experience(&self, player: PlayerRef) -> Option<NativePlayerExperience> {
        crate::phase::unprovided(
            "Backend::player_experience",
            "player_experience",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Player.getXpNeededForNextLevel`: the points the current level needs
    /// to fill.
    fn player_xp_needed_for_next_level(&self, player: PlayerRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::player_xp_needed_for_next_level",
            "player_xp_needed_for_next_level",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `player_food` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait PlayerFoodBackendMut {
    /// Side: server.
    /// `FoodData.eat(nutrition, saturationModifier)`: adds `nutrition` to
    /// the food level (0 to 20) and `nutrition * modifier * 2` to the
    /// saturation, which stays at most the food level.
    fn player_feed(
        &mut self,
        player: PlayerRef,
        nutrition: i32,
        saturation_modifier: f32,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_feed",
            "player_feed",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Player.causeFoodExhaustion`: adds exhaustion (at most 40), as
    /// sprinting or jumping does. An invulnerable player (creative) or a
    /// spectator gets none, and the call still returns true.
    fn player_add_exhaustion(&mut self, player: PlayerRef, amount: f32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_add_exhaustion",
            "player_add_exhaustion",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `FoodData.setFoodLevel`: sets the food level as it is, unclamped,
    /// as vanilla does.
    fn player_set_food_level(&mut self, player: PlayerRef, level: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_food_level",
            "player_set_food_level",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `FoodData.setSaturation`: sets the saturation as it is, unclamped.
    fn player_set_saturation(&mut self, player: PlayerRef, saturation: f32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_saturation",
            "player_set_saturation",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets the exhaustion as it is (Paper's `HumanEntity#setExhaustion`;
    /// vanilla's `FoodData` only adds to it).
    fn player_set_exhaustion(&mut self, player: PlayerRef, exhaustion: f32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_exhaustion",
            "player_set_exhaustion",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ServerPlayer.setExperiencePoints`: sets the progress toward the next
    /// level to `points` of the points it needs, kept below a full bar. The
    /// level and the total are kept.
    fn player_set_experience_points(&mut self, player: PlayerRef, points: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_experience_points",
            "player_set_experience_points",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ServerPlayer.setExperienceLevels`: sets the level, keeping the
    /// progress and the total.
    fn player_set_experience_levels(&mut self, player: PlayerRef, levels: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_experience_levels",
            "player_set_experience_levels",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
