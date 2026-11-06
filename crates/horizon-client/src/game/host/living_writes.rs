//! The game's side of the `living_writes` area (`horizon-mod-api/src/host/living_writes.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl LivingWritesBackend for Game<'_> {
    fn living_item_use_remaining(&self, living: LivingRef) -> Option<i32> {
        mod_api::__private::check_access(
            "living_item_use_remaining",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_item_use_remaining(living)
    }
}
impl LivingWritesBackendMut for Game<'_> {
    fn living_swing(
        &mut self,
        living: LivingRef,
        hand: Hand,
        send_to_self: bool,
    ) -> bool {
        mod_api::__private::check_access(
            "living_swing",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_swing(living, hand, send_to_self)
    }
    fn living_arrow_count(&mut self, living: LivingRef) -> Option<i32> {
        mod_api::__private::check_access(
            "living_arrow_count",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_arrow_count(living)
    }
    fn living_set_arrow_count(&mut self, living: LivingRef, count: i32) -> bool {
        mod_api::__private::check_access(
            "living_set_arrow_count",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_set_arrow_count(living, count)
    }
    fn living_stinger_count(&mut self, living: LivingRef) -> Option<i32> {
        mod_api::__private::check_access(
            "living_stinger_count",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_stinger_count(living)
    }
    fn living_set_stinger_count(&mut self, living: LivingRef, count: i32) -> bool {
        mod_api::__private::check_access(
            "living_set_stinger_count",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_set_stinger_count(living, count)
    }
    fn living_set_arrow_removal_ticks(&mut self, living: LivingRef, ticks: i32) -> bool {
        mod_api::__private::check_access(
            "living_set_arrow_removal_ticks",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_set_arrow_removal_ticks(living, ticks)
    }
    fn living_set_stinger_removal_ticks(
        &mut self,
        living: LivingRef,
        ticks: i32,
    ) -> bool {
        mod_api::__private::check_access(
            "living_set_stinger_removal_ticks",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_set_stinger_removal_ticks(living, ticks)
    }
    fn living_set_item_use_remaining(&mut self, living: LivingRef, ticks: i32) -> bool {
        mod_api::__private::check_access(
            "living_set_item_use_remaining",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_living_set_item_use_remaining(living, ticks)
    }
}
