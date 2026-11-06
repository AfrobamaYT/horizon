//! The `mob_goals` area: host methods the game implements (`game/host/mob_goals.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `mob_goals` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait MobGoalsBackend {
    /// Side: server.
    /// `GoalSelector.getRunningGoals` read wider: every goal of the goal
    /// selector as a `NativeMobGoalEntry` (the priority, the kind id and
    /// whether it runs), in registration order. None for a stale or
    /// unavailable mob, a client, and the dragon.
    fn mob_goals(&self, mob: MobRef) -> Option<Vec<NativeMobGoalEntry>> {
        crate::phase::unprovided(
            "Backend::mob_goals",
            "mob_goals",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Every goal of the target selector, as `goals` reads the goal
    /// selector's.
    fn mob_target_goals(&self, mob: MobRef) -> Option<Vec<NativeMobGoalEntry>> {
        crate::phase::unprovided(
            "Backend::mob_target_goals",
            "mob_target_goals",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `mob_goals` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait MobGoalsBackendMut {
    /// Side: server.
    /// `GoalSelector.addGoal(priority, goal)` on a mob's goal selector.
    /// Refuses a stale or unavailable mob, a client, and the dragon (it
    /// keeps no selector); a goal the mob's vanilla class does not hold (a
    /// `PathfinderMob` goal for a bat, ghast or phantom, `BreedGoal` or
    /// `FollowParentGoal` for a mob that is no animal); and non-finite
    /// numbers, negative speeds and distances, a probability outside
    /// 0..=1, a negative interval, and an unknown species id (in `prey`,
    /// `what` or `only`) or empty `items`. The target selector's kinds
    /// (`nearest_target`, `hurt_by_target`) go in through
    /// `add_target_goal`; both selectors take them, as vanilla's two
    /// selectors do.
    fn mob_add_goal(&mut self, mob: MobRef, priority: i32, goal: &NativeGoal) -> bool {
        crate::phase::unprovided(
            "BackendMut::mob_add_goal",
            "mob_add_goal",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `GoalSelector.addGoal` on the mob's target selector. The same rules
    /// as `add_goal`.
    fn mob_add_target_goal(&mut self, mob: MobRef, priority: i32, goal: &NativeGoal) -> bool {
        crate::phase::unprovided(
            "BackendMut::mob_add_target_goal",
            "mob_add_target_goal",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `GoalSelector.removeGoal` on the goal selector, by kind: every goal
    /// whose id, as `goals` reads it, is `id` goes (`removeAllGoals`), a
    /// running one stopped first. How many went; None for a stale or
    /// unavailable mob, a client, and the dragon. Nothing in a mob's tick
    /// resolves, as its copy is held out there: add what a mod needs in
    /// `entity_join`, which fires for a loaded chunk's mobs too.
    fn mob_remove_goal(&mut self, mob: MobRef, id: &str) -> Option<usize> {
        crate::phase::unprovided(
            "BackendMut::mob_remove_goal",
            "mob_remove_goal",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `GoalSelector.removeGoal` on the target selector, by kind, as
    /// `remove_goal` does on the goal selector.
    fn mob_remove_target_goal(&mut self, mob: MobRef, id: &str) -> Option<usize> {
        crate::phase::unprovided(
            "BackendMut::mob_remove_target_goal",
            "mob_remove_target_goal",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
