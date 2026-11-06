//! The game's side of the `living_health_writes` area (`horizon-mod-api/src/host/living_health_writes.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl LivingHealthWritesBackend for Game<'_> {
    fn living_kill_credit(&self, living: LivingRef) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "living_kill_credit",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_kill_credit(living)
    }
    fn living_last_attacker(&self, living: LivingRef) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "living_last_attacker",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_last_attacker(living)
    }
}
impl LivingHealthWritesBackendMut for Game<'_> {
    fn living_set_health(&mut self, living: LivingRef, health: f32) -> bool {
        mod_api::__private::check_access(
            "living_set_health",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_set_health(living, health)
    }
    fn living_heal(&mut self, living: LivingRef, amount: f32) -> bool {
        mod_api::__private::check_access(
            "living_heal",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_heal(living, amount)
    }
}
