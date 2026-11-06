//! The game's side of the `entity_moves` area (`horizon-mod-api/src/host/entity_moves.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl EntityMovesBackend for Game<'_> {
    fn entity_head_yaw(&self, entity: EntityRef) -> Option<f32> {
        mod_api::__private::check_access(
            "entity_head_yaw",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_head_yaw(entity)
    }
    fn entity_direction(&self, entity: EntityRef) -> Option<&'static str> {
        mod_api::__private::check_access(
            "entity_direction",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_direction(entity)
    }
    fn entity_dimensions(&self, entity: EntityRef) -> Option<[f64; 2]> {
        mod_api::__private::check_access(
            "entity_dimensions",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_dimensions(entity)
    }
}
impl EntityMovesBackendMut for Game<'_> {
    fn entity_teleport_to(
        &mut self,
        entity: EntityRef,
        pos: [f64; 3],
        rotation: Option<[f32; 2]>,
    ) -> bool {
        mod_api::__private::check_access(
            "entity_teleport_to",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_teleport_to(entity, pos, rotation)
    }
    fn entity_set_rotation(&mut self, entity: EntityRef, yaw: f32, pitch: f32) -> bool {
        mod_api::__private::check_access(
            "entity_set_rotation",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_rotation(entity, yaw, pitch)
    }
    fn entity_set_head_yaw(&mut self, entity: EntityRef, yaw: f32) -> bool {
        mod_api::__private::check_access(
            "entity_set_head_yaw",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_head_yaw(entity, yaw)
    }
    fn entity_push(&mut self, entity: EntityRef, delta: [f64; 3]) -> bool {
        mod_api::__private::check_access("entity_push", mod_api::__private::LoadAccess::WorldOnly);
        self.native_entity_push(entity, delta)
    }
}
