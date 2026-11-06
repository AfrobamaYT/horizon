//! The game's side of the `player_modes` area (`horizon-mod-api/src/host/player_modes.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl PlayerModesBackend for Game<'_> {
    fn player_permission_level(&self, player: PlayerRef) -> Option<u8> {
        mod_api::__private::check_access(
            "player_permission_level",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_permission_level(player)
    }
    fn player_has_infinite_materials(&self, player: PlayerRef) -> Option<bool> {
        mod_api::__private::check_access(
            "player_has_infinite_materials",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_has_infinite_materials(player)
    }
    fn player_can_use_game_master_blocks(&self, player: PlayerRef) -> Option<bool> {
        mod_api::__private::check_access(
            "player_can_use_game_master_blocks",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_can_use_game_master_blocks(player)
    }
    fn player_previous_game_mode(&self, player: PlayerRef) -> Option<Option<GameMode>> {
        mod_api::__private::check_access(
            "player_previous_game_mode",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_previous_game_mode(player)
    }
    fn player_is_secondary_use_active(&self, player: PlayerRef) -> Option<bool> {
        mod_api::__private::check_access(
            "player_is_secondary_use_active",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_is_secondary_use_active(player)
    }
}
