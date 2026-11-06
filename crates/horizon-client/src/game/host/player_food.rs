//! The game's side of the `player_food` area (`horizon-mod-api/src/host/player_food.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl PlayerFoodBackend for Game<'_> {
    fn player_needs_food(&self, player: PlayerRef) -> Option<bool> {
        mod_api::__private::check_access(
            "player_needs_food",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_needs_food(player)
    }
    fn player_has_enough_food(&self, player: PlayerRef) -> Option<bool> {
        mod_api::__private::check_access(
            "player_has_enough_food",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_has_enough_food(player)
    }
    fn player_can_eat(&self, player: PlayerRef, always: bool) -> Option<bool> {
        mod_api::__private::check_access(
            "player_can_eat",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_can_eat(player, always)
    }
    fn player_experience(&self, player: PlayerRef) -> Option<NativePlayerExperience> {
        mod_api::__private::check_access(
            "player_experience",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_experience(player)
    }
    fn player_xp_needed_for_next_level(&self, player: PlayerRef) -> Option<i32> {
        mod_api::__private::check_access(
            "player_xp_needed_for_next_level",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_xp_needed_for_next_level(player)
    }
}
impl PlayerFoodBackendMut for Game<'_> {
    fn player_feed(
        &mut self,
        player: PlayerRef,
        nutrition: i32,
        saturation_modifier: f32,
    ) -> bool {
        mod_api::__private::check_access(
            "player_feed",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_feed(player, nutrition, saturation_modifier)
    }
    fn player_add_exhaustion(&mut self, player: PlayerRef, amount: f32) -> bool {
        mod_api::__private::check_access(
            "player_add_exhaustion",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_add_exhaustion(player, amount)
    }
    fn player_set_food_level(&mut self, player: PlayerRef, level: i32) -> bool {
        mod_api::__private::check_access(
            "player_set_food_level",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_food_level(player, level)
    }
    fn player_set_saturation(&mut self, player: PlayerRef, saturation: f32) -> bool {
        mod_api::__private::check_access(
            "player_set_saturation",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_saturation(player, saturation)
    }
    fn player_set_exhaustion(&mut self, player: PlayerRef, exhaustion: f32) -> bool {
        mod_api::__private::check_access(
            "player_set_exhaustion",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_exhaustion(player, exhaustion)
    }
    fn player_set_experience_points(&mut self, player: PlayerRef, points: i32) -> bool {
        mod_api::__private::check_access(
            "player_set_experience_points",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_experience_points(player, points)
    }
    fn player_set_experience_levels(&mut self, player: PlayerRef, levels: i32) -> bool {
        mod_api::__private::check_access(
            "player_set_experience_levels",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_experience_levels(player, levels)
    }
}
