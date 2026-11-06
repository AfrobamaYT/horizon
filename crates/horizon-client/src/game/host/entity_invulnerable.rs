//! The game's side of the `entity_invulnerable` area (`horizon-mod-api/src/host/entity_invulnerable.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl EntityInvulnerableBackend for Game<'_> {
    fn entity_permanently_invulnerable(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_permanently_invulnerable",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_permanently_invulnerable(entity)
    }
    fn entity_damage_cooldown_time(&self, entity: EntityRef) -> Option<i32> {
        mod_api::__private::check_access(
            "entity_damage_cooldown_time",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_damage_cooldown_time(entity)
    }
}
impl EntityInvulnerableBackendMut for Game<'_> {
    fn entity_set_invulnerable(&mut self, entity: EntityRef, invulnerable: bool) -> bool {
        mod_api::__private::check_access(
            "entity_set_invulnerable",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_invulnerable(entity, invulnerable)
    }
    fn entity_set_damage_cooldown_time(&mut self, entity: EntityRef, ticks: i32) -> bool {
        mod_api::__private::check_access(
            "entity_set_damage_cooldown_time",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_set_damage_cooldown_time(entity, ticks)
    }
}
