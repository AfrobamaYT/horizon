//! The game's side of the `player_actions` area (`horizon-mod-api/src/host/player_actions.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl PlayerActionsBackendMut for Game<'_> {
    fn player_perform_action(
        &mut self,
        player: PlayerRef,
        action: PlayerAction,
    ) -> Result<ActionOutcome, ActionRefused> {
        mod_api::__private::check_access(
            "player_perform_action",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_perform_action(player, action)
    }
    fn player_perform_action_scoped(
        &mut self,
        player: PlayerRef,
        action: PlayerAction,
        setup: PlayerActionSetup,
    ) -> Result<ScopedActionResult, ActionRefused> {
        mod_api::__private::check_access(
            "player_perform_action_scoped",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_perform_action_scoped(player, action, setup)
    }
}
