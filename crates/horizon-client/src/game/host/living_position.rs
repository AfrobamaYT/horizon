//! The game's side of the `living_position` area (`horizon-mod-api/src/host/living_position.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl LivingPositionBackend for Game<'_> {
    fn living_has_line_of_sight(&self, living: LivingRef, target: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "living_has_line_of_sight",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_has_line_of_sight(living, target)
    }
}
impl LivingPositionBackendMut for Game<'_> {
    fn living_random_teleport(
        &mut self,
        living: LivingRef,
        at: [f64; 3],
        avoid: Option<&str>,
    ) -> bool {
        mod_api::__private::check_access(
            "living_random_teleport",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_random_teleport(living, at, avoid)
    }
}
