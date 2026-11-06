//! The game's side of the `player_read` area (`horizon-mod-api/src/host/player_read.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl PlayerReadHost for Game<'_> {
    fn player(&self, uuid: u128) -> Option<PlayerRef> {
        mod_api::__private::check_access("player", mod_api::__private::LoadAccess::WorldOnly);
        self.native_player_ref(uuid)
    }
}
impl PlayerReadServerHost for Game<'_> {
    fn fake_player(&mut self, level: LevelRef, uuid: u128, name: &str) -> Option<PlayerRef> {
        mod_api::__private::check_access("fake_player", mod_api::__private::LoadAccess::WorldOnly);
        self.native_fake_player(level, uuid, name)
    }
}
impl PlayerReadBackend for Game<'_> {
    fn player_is_fake(&self, player: PlayerRef) -> Option<bool> {
        mod_api::__private::check_access(
            "player_is_fake",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_is_fake(player)
    }
    fn player_inventory(&self, player: PlayerRef) -> Option<Vec<ItemStack>> {
        mod_api::__private::check_access(
            "player_inventory",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_inventory(player)
    }
    fn player_game_mode(&self, player: PlayerRef) -> Option<GameMode> {
        mod_api::__private::check_access(
            "player_game_mode",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_game_mode(player)
    }
    fn player_is_creative(&self, player: PlayerRef) -> Option<bool> {
        mod_api::__private::check_access(
            "player_is_creative",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_is_creative(player)
    }
    fn player_is_spectator(&self, player: PlayerRef) -> Option<bool> {
        mod_api::__private::check_access(
            "player_is_spectator",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_is_spectator(player)
    }
    fn player_may_build(&self, player: PlayerRef) -> Option<bool> {
        mod_api::__private::check_access(
            "player_may_build",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_may_build(player)
    }
    fn player_food(&self, player: PlayerRef) -> Option<NativePlayerFood> {
        mod_api::__private::check_access("player_food", mod_api::__private::LoadAccess::WorldOnly);
        self.native_player_food(player)
    }
    fn player_abilities(&self, player: PlayerRef) -> Option<NativePlayerAbilities> {
        mod_api::__private::check_access(
            "player_abilities",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_abilities(player)
    }
}
impl PlayerReadBackendMut for Game<'_> {
    fn player_remove_fake(&mut self, player: PlayerRef) -> bool {
        mod_api::__private::check_access(
            "player_remove_fake",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_remove_fake(player)
    }
}
