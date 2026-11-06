//! The game's side of the `entity_state` area (`horizon-mod-api/src/host/entity_state.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl EntityStateBackend for Game<'_> {
    fn entity_is_shift_key_down(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_is_shift_key_down",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_is_shift_key_down(entity)
    }
    fn entity_is_crouching(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_is_crouching",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_is_crouching(entity)
    }
    fn entity_is_sprinting(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_is_sprinting",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_is_sprinting(entity)
    }
    fn entity_is_in_water(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_is_in_water",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_is_in_water(entity)
    }
    fn entity_is_in_lava(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_is_in_lava",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_is_in_lava(entity)
    }
    fn entity_fire_immune(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_fire_immune",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_fire_immune(entity)
    }
    fn entity_fire_ticks(&self, entity: EntityRef) -> Option<i32> {
        mod_api::__private::check_access(
            "entity_fire_ticks",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_fire_ticks(entity)
    }
    fn entity_portal_cooldown(&self, entity: EntityRef) -> Option<i32> {
        mod_api::__private::check_access(
            "entity_portal_cooldown",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_portal_cooldown(entity)
    }
    fn entity_air_supply(&self, entity: EntityRef) -> Option<i32> {
        mod_api::__private::check_access(
            "entity_air_supply",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_air_supply(entity)
    }
    fn entity_max_air_supply(&self, entity: EntityRef) -> Option<i32> {
        mod_api::__private::check_access(
            "entity_max_air_supply",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_max_air_supply(entity)
    }
    fn entity_is_silent(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_is_silent",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_is_silent(entity)
    }
    fn entity_glowing(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_glowing",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_glowing(entity)
    }
    fn entity_no_gravity(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_no_gravity",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_no_gravity(entity)
    }
    fn entity_ticks_frozen(&self, entity: EntityRef) -> Option<i32> {
        mod_api::__private::check_access(
            "entity_ticks_frozen",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_ticks_frozen(entity)
    }
    fn entity_tick_count(&self, entity: EntityRef) -> Option<i32> {
        mod_api::__private::check_access(
            "entity_tick_count",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_tick_count(entity)
    }
    fn entity_is_invisible(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_is_invisible",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_is_invisible(entity)
    }
    fn entity_pose(&self, entity: EntityRef) -> Option<&'static str> {
        mod_api::__private::check_access("entity_pose", mod_api::__private::LoadAccess::WorldOnly);
        self.native_entity_pose(entity)
    }
}
impl EntityStateBackendMut for Game<'_> {
    fn entity_set_remaining_fire_ticks(&mut self, entity: EntityRef, ticks: i32) -> bool {
        mod_api::__private::check_access(
            "entity_set_remaining_fire_ticks",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_remaining_fire_ticks(entity, ticks)
    }
    fn entity_clear_fire(&mut self, entity: EntityRef) -> bool {
        mod_api::__private::check_access(
            "entity_clear_fire",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_clear_fire(entity)
    }
    fn entity_set_air_supply(&mut self, entity: EntityRef, air: i32) -> bool {
        mod_api::__private::check_access(
            "entity_set_air_supply",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_air_supply(entity, air)
    }
    fn entity_set_silent(&mut self, entity: EntityRef, silent: bool) -> bool {
        mod_api::__private::check_access(
            "entity_set_silent",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_silent(entity, silent)
    }
    fn entity_set_glowing(&mut self, entity: EntityRef, glowing: bool) -> bool {
        mod_api::__private::check_access(
            "entity_set_glowing",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_glowing(entity, glowing)
    }
    fn entity_set_no_gravity(&mut self, entity: EntityRef, no_gravity: bool) -> bool {
        mod_api::__private::check_access(
            "entity_set_no_gravity",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_no_gravity(entity, no_gravity)
    }
    fn entity_set_ticks_frozen(&mut self, entity: EntityRef, ticks: i32) -> bool {
        mod_api::__private::check_access(
            "entity_set_ticks_frozen",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_ticks_frozen(entity, ticks)
    }
    fn entity_set_invisible(&mut self, entity: EntityRef, invisible: bool) -> bool {
        mod_api::__private::check_access(
            "entity_set_invisible",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_invisible(entity, invisible)
    }
}
