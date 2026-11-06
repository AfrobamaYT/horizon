//! The game's side of the `entity_motion` area (`horizon-mod-api/src/host/entity_motion.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl EntityMotionBackend for Game<'_> {
    fn entity_velocity(&self, entity: EntityRef) -> Option<[f64; 3]> {
        mod_api::__private::check_access(
            "entity_velocity",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_velocity(entity)
    }
    fn entity_on_ground(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_on_ground",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_on_ground(entity)
    }
    fn entity_bounding_box(&self, entity: EntityRef) -> Option<[[f64; 3]; 2]> {
        mod_api::__private::check_access(
            "entity_bounding_box",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_bounding_box(entity)
    }
    fn entity_eye_height(&self, entity: EntityRef) -> Option<f64> {
        mod_api::__private::check_access(
            "entity_eye_height",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_eye_height(entity)
    }
    fn entity_eye_position(&self, entity: EntityRef) -> Option<[f64; 3]> {
        mod_api::__private::check_access(
            "entity_eye_position",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_eye_position(entity)
    }
    fn entity_view_vector(&self, entity: EntityRef) -> Option<[f64; 3]> {
        mod_api::__private::check_access(
            "entity_view_vector",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_view_vector(entity)
    }
    fn entity_distance_to_sqr(&self, entity: EntityRef, pos: [f64; 3]) -> Option<f64> {
        mod_api::__private::check_access(
            "entity_distance_to_sqr",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_distance_to_sqr(entity, pos)
    }
}
impl EntityMotionBackendMut for Game<'_> {
    fn entity_set_velocity(&mut self, entity: EntityRef, velocity: [f64; 3]) -> bool {
        mod_api::__private::check_access(
            "entity_set_velocity",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_velocity(entity, velocity)
    }
}
