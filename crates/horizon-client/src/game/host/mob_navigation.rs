//! The game's side of the `mob_navigation` area (`horizon-mod-api/src/host/mob_navigation.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl MobNavigationBackend for Game<'_> {
    fn mob_is_done(&self, mob: MobRef) -> Option<bool> {
        mod_api::__private::check_access(
            "mob_is_done",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_is_done(mob)
    }
    fn mob_navigation_speed(&self, mob: MobRef) -> Option<f64> {
        mod_api::__private::check_access(
            "mob_navigation_speed",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_navigation_speed(mob)
    }
    fn mob_pathfinding_malus(&self, mob: MobRef, kind: PathType) -> Option<f32> {
        mod_api::__private::check_access(
            "mob_pathfinding_malus",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_pathfinding_malus(mob, kind)
    }
}
impl MobNavigationBackendMut for Game<'_> {
    fn mob_move_to(&mut self, mob: MobRef, at: [f64; 3], speed: f64) -> bool {
        mod_api::__private::check_access(
            "mob_move_to",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_move_to(mob, at, speed)
    }
    fn mob_stop(&mut self, mob: MobRef) -> bool {
        mod_api::__private::check_access(
            "mob_stop",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_stop(mob)
    }
    fn mob_set_navigation_speed(&mut self, mob: MobRef, speed: f64) -> bool {
        mod_api::__private::check_access(
            "mob_set_navigation_speed",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_set_navigation_speed(mob, speed)
    }
    fn mob_set_pathfinding_malus(
        &mut self,
        mob: MobRef,
        kind: PathType,
        malus: f32,
    ) -> bool {
        mod_api::__private::check_access(
            "mob_set_pathfinding_malus",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_set_pathfinding_malus(mob, kind, malus)
    }
}
