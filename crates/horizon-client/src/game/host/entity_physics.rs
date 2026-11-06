//! The game's side of the `entity_physics` area (`horizon-mod-api/src/host/entity_physics.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl EntityPhysicsBackend for Game<'_> {
    fn entity_no_physics(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_no_physics",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_no_physics(entity)
    }
}
