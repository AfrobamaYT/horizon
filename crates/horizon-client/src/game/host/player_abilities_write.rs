//! The game's side of the `player_abilities_write` area (`horizon-mod-api/src/host/player_abilities_write.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl PlayerAbilitiesWriteBackendMut for Game<'_> {
    fn player_set_abilities(
        &mut self,
        player: PlayerRef,
        abilities: NativePlayerAbilities,
    ) -> bool {
        mod_api::__private::check_access(
            "player_set_abilities",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_set_abilities(player, abilities)
    }
}
