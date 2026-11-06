//! The game's side of the `entity_riding` area (`horizon-mod-api/src/host/entity_riding.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl EntityRidingBackend for Game<'_> {
    fn entity_vehicle(&self, entity: EntityRef) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "entity_vehicle",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_vehicle(entity)
    }
    fn entity_is_passenger(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_is_passenger",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_is_passenger(entity)
    }
    fn entity_root_vehicle(&self, entity: EntityRef) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "entity_root_vehicle",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_root_vehicle(entity)
    }
    fn entity_passengers(&self, entity: EntityRef) -> Option<Vec<EntityRef>> {
        mod_api::__private::check_access(
            "entity_passengers",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_passengers(entity)
    }
    fn entity_is_vehicle(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_is_vehicle",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_is_vehicle(entity)
    }
}
impl EntityRidingBackendMut for Game<'_> {
    fn entity_start_riding(&mut self, entity: EntityRef, vehicle: EntityRef, force: bool) -> bool {
        mod_api::__private::check_access(
            "entity_start_riding",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_start_riding(entity, vehicle, force)
    }
    fn entity_stop_riding(&mut self, entity: EntityRef) -> bool {
        mod_api::__private::check_access(
            "entity_stop_riding",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_stop_riding(entity)
    }
    fn entity_eject_passengers(&mut self, entity: EntityRef) -> bool {
        mod_api::__private::check_access(
            "entity_eject_passengers",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_eject_passengers(entity)
    }
}
