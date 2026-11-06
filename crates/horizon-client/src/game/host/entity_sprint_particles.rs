//! The game's side of the `entity_sprint_particles` area (`horizon-mod-api/src/host/entity_sprint_particles.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl EntitySprintParticlesClientBackendMut for Game<'_> {
    fn entity_sprint_particles(&mut self, entity: EntityRef) -> bool {
        mod_api::__private::check_access(
            "entity_sprint_particles",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_sprint_particles(entity)
    }
}
