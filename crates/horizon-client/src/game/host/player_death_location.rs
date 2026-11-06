//! The game's side of the `player_death_location` area (`horizon-mod-api/src/host/player_death_location.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl PlayerDeathLocationBackend for Game<'_> {
    fn player_last_death_location(&self, player: PlayerRef) -> Option<GlobalPos> {
        mod_api::__private::check_access(
            "player_last_death_location",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_last_death_location(player)
    }
}
impl PlayerDeathLocationBackendMut for Game<'_> {
    fn player_set_last_death_location(
        &mut self,
        player: PlayerRef,
        location: Option<GlobalPos>,
    ) -> bool {
        mod_api::__private::check_access(
            "player_set_last_death_location",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_last_death_location(player, location)
    }
}
