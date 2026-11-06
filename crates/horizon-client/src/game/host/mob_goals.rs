//! The game's side of the `mob_goals` area (`horizon-mod-api/src/host/mob_goals.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl MobGoalsBackend for Game<'_> {
    fn mob_goals(&self, mob: MobRef) -> Option<Vec<NativeMobGoalEntry>> {
        mod_api::__private::check_access("mob_goals", mod_api::__private::LoadAccess::WorldOnly);
        self.native_mob_goals(mob)
    }
    fn mob_target_goals(&self, mob: MobRef) -> Option<Vec<NativeMobGoalEntry>> {
        mod_api::__private::check_access(
            "mob_target_goals",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_target_goals(mob)
    }
}
impl MobGoalsBackendMut for Game<'_> {
    fn mob_add_goal(&mut self, mob: MobRef, priority: i32, goal: &NativeGoal) -> bool {
        mod_api::__private::check_access("mob_add_goal", mod_api::__private::LoadAccess::WorldOnly);
        self.native_mob_add_goal(mob, priority, goal)
    }
    fn mob_add_target_goal(&mut self, mob: MobRef, priority: i32, goal: &NativeGoal) -> bool {
        mod_api::__private::check_access(
            "mob_add_target_goal",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_add_target_goal(mob, priority, goal)
    }
    fn mob_remove_goal(&mut self, mob: MobRef, id: &str) -> Option<usize> {
        mod_api::__private::check_access(
            "mob_remove_goal",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_remove_goal(mob, id)
    }
    fn mob_remove_target_goal(&mut self, mob: MobRef, id: &str) -> Option<usize> {
        mod_api::__private::check_access(
            "mob_remove_target_goal",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_remove_target_goal(mob, id)
    }
}
