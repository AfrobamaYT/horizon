//! The game's side of the `level_entities_in` area (`horizon-mod-api/src/host/level_entities_in.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl LevelEntitiesInBackend for Game<'_> {
    fn level_entities_in(
        &self,
        level: LevelRef,
        min: [f64; 3],
        max: [f64; 3],
    ) -> Option<Vec<EntityRef>> {
        mod_api::__private::check_access(
            "level_entities_in",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_entities_in(level, min, max)
    }
}
