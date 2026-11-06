//! The game's side of the `living_health` area (`horizon-mod-api/src/host/living_health.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl LivingHealthBackend for Game<'_> {
    fn living_health(&self, living: LivingRef) -> Option<f32> {
        mod_api::__private::check_access(
            "living_health",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_health(living)
    }
    fn living_max_health(&self, living: LivingRef) -> Option<f32> {
        mod_api::__private::check_access(
            "living_max_health",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_max_health(living)
    }
    fn living_alive(&self, living: LivingRef) -> Option<bool> {
        mod_api::__private::check_access(
            "living_alive",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_alive(living)
    }
}
