//! The game's side of the `player_criteria` area (`horizon-mod-api/src/host/player_criteria.rs`).
use crate::game::Game;
use std::sync::Arc;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl PlayerCriteriaBackendMut for Game<'_> {
    fn player_trigger_criterion(
        &mut self,
        player: PlayerRef,
        trigger: &Identifier,
        test: Arc<dyn CriterionTest>,
    ) -> Result<(), TriggerCriterionError> {
        mod_api::__private::check_access(
            "player_trigger_criterion",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_trigger_criterion(player, trigger, test)
    }
}
