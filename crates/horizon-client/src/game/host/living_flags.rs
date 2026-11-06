//! The game's side of the `living_flags` area (`horizon-mod-api/src/host/living_flags.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl LivingFlagsBackend for Game<'_> {
    fn living_gliding_ticks(&self, living: LivingRef) -> Option<i32> {
        mod_api::__private::check_access(
            "living_gliding_ticks",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_gliding_ticks(living)
    }
    fn living_is_gliding(&self, living: LivingRef) -> Option<bool> {
        mod_api::__private::check_access(
            "living_is_gliding",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_is_gliding(living)
    }
    fn living_is_sleeping(&self, living: LivingRef) -> Option<bool> {
        mod_api::__private::check_access(
            "living_is_sleeping",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_is_sleeping(living)
    }
    fn living_sleeping_pos(&self, living: LivingRef) -> Option<[i32; 3]> {
        mod_api::__private::check_access(
            "living_sleeping_pos",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_sleeping_pos(living)
    }
    fn living_is_jumping(&self, living: LivingRef) -> Option<bool> {
        mod_api::__private::check_access(
            "living_is_jumping",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_is_jumping(living)
    }
    fn living_is_climbing(&self, living: LivingRef) -> Option<bool> {
        mod_api::__private::check_access(
            "living_is_climbing",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_is_climbing(living)
    }
    fn living_baby(&self, living: LivingRef) -> Option<bool> {
        mod_api::__private::check_access("living_baby", mod_api::__private::LoadAccess::WorldOnly);
        self.native_living_baby(living)
    }
}
impl LivingFlagsBackendMut for Game<'_> {
    fn living_stop_gliding(&mut self, living: LivingRef) -> bool {
        mod_api::__private::check_access(
            "living_stop_gliding",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_stop_gliding(living)
    }
    fn living_start_sleeping(&mut self, living: LivingRef, pos: [i32; 3]) -> bool {
        mod_api::__private::check_access(
            "living_start_sleeping",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_start_sleeping(living, pos)
    }
    fn living_stop_sleeping(&mut self, living: LivingRef) -> bool {
        mod_api::__private::check_access(
            "living_stop_sleeping",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_stop_sleeping(living)
    }
    fn living_set_sleeping_pos(&mut self, living: LivingRef, pos: [i32; 3]) -> bool {
        mod_api::__private::check_access(
            "living_set_sleeping_pos",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_set_sleeping_pos(living, pos)
    }
    fn living_clear_sleeping_pos(&mut self, living: LivingRef) -> bool {
        mod_api::__private::check_access(
            "living_clear_sleeping_pos",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_clear_sleeping_pos(living)
    }
    fn living_set_is_jumping(&mut self, living: LivingRef, jumping: bool) -> bool {
        mod_api::__private::check_access(
            "living_set_is_jumping",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_set_is_jumping(living, jumping)
    }
}
