//! The game's side of the `entity_handles` area (`horizon-mod-api/src/host/entity_handles.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl EntityHandlesHost for Game<'_> {
    fn entity(&self, id: u32) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "entity",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity(id)
    }
    fn level(&self, dimension: &str) -> Option<LevelRef> {
        mod_api::__private::check_access(
            "level",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level(dimension)
    }
}
impl EntityHandlesBackend for Game<'_> {
    fn entity_position(&self, entity: EntityRef) -> Option<[f64; 3]> {
        mod_api::__private::check_access(
            "entity_position",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_position(entity)
    }
    fn entity_block_position(&self, entity: EntityRef) -> Option<[i32; 3]> {
        mod_api::__private::check_access(
            "entity_block_position",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_block_position(entity)
    }
    fn entity_rotation(&self, entity: EntityRef) -> Option<[f32; 2]> {
        mod_api::__private::check_access(
            "entity_rotation",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_rotation(entity)
    }
    fn entity_kind(&self, entity: EntityRef) -> Option<String> {
        mod_api::__private::check_access(
            "entity_kind",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_kind(entity)
    }
    fn entity_level(&self, entity: EntityRef) -> Option<LevelRef> {
        mod_api::__private::check_access(
            "entity_level",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_level(entity)
    }
}
impl EntityHandlesBackendMut for Game<'_> {
    fn entity_uuid(&mut self, entity: EntityRef) -> Option<u128> {
        mod_api::__private::check_access(
            "entity_uuid",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_uuid(entity)
    }
}
