//! The game's side of the `player_xp_give` area (`horizon-mod-api/src/host/player_xp_give.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl PlayerXpGiveBackendMut for Game<'_> {
    fn player_give_experience_points(&mut self, player: PlayerRef, points: u32) -> bool {
        mod_api::__private::check_access(
            "player_give_experience_points",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_give_experience_points(player, points)
    }
    fn player_give_levels(&mut self, player: PlayerRef, levels: i32) -> bool {
        mod_api::__private::check_access(
            "player_give_levels",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_give_levels(player, levels)
    }
}
