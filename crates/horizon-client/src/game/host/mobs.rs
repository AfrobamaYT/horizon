//! The game's side of the `mobs` area (`horizon-mod-api/src/host/mobs.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl MobsHost for Game<'_> {
    fn mob(&self, entity: EntityRef) -> Option<MobRef> {
        mod_api::__private::check_access("mob", mod_api::__private::LoadAccess::WorldOnly);
        self.native_mob_ref(entity)
    }
}
impl MobsServerHost for Game<'_> {
    fn kill_mob(&mut self, dimension: &str, id: u32) -> bool {
        mod_api::__private::check_access("kill_mob", mod_api::__private::LoadAccess::WorldOnly);
        self.native_kill_mob(dimension, id)
    }
    fn dragon(&self, entity: EntityRef) -> Option<DragonRef> {
        mod_api::__private::check_access("dragon", mod_api::__private::LoadAccess::WorldOnly);
        self.native_dragon_ref(entity)
    }
}
impl MobsBackend for Game<'_> {
    fn dragon_phase(&self, dragon: DragonRef) -> Option<NativeDragonPhase> {
        mod_api::__private::check_access("dragon_phase", mod_api::__private::LoadAccess::WorldOnly);
        self.native_dragon_phase(dragon)
    }
    fn dragon_alive_crystals(&self, dragon: DragonRef) -> Option<i32> {
        mod_api::__private::check_access(
            "dragon_alive_crystals",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_dragon_alive_crystals(dragon)
    }
    fn dragon_previously_killed(&self, dragon: DragonRef) -> Option<bool> {
        mod_api::__private::check_access(
            "dragon_previously_killed",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_dragon_previously_killed(dragon)
    }
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
    fn mob_is_done(&self, mob: MobRef) -> Option<bool> {
        mod_api::__private::check_access("mob_is_done", mod_api::__private::LoadAccess::WorldOnly);
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
    fn mob_can_use_portal(&self, mob: MobRef) -> Option<bool> {
        mod_api::__private::check_access(
            "mob_can_use_portal",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_can_use_portal(mob)
    }
    fn mob_leash_holder(&self, mob: MobRef) -> Option<MobLeashHolder> {
        mod_api::__private::check_access(
            "mob_leash_holder",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_leash_holder(mob)
    }
    fn mob_target(&self, mob: MobRef) -> Option<LivingRef> {
        mod_api::__private::check_access("mob_target", mod_api::__private::LoadAccess::WorldOnly);
        self.native_mob_target(mob)
    }
    fn mob_is_aggressive(&self, mob: MobRef) -> Option<bool> {
        mod_api::__private::check_access(
            "mob_is_aggressive",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_is_aggressive(mob)
    }
    fn mob_persistence_required(&self, mob: MobRef) -> Option<bool> {
        mod_api::__private::check_access(
            "mob_persistence_required",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_persistence_required(mob)
    }
    fn mob_requires_custom_persistence(&self, mob: MobRef) -> Option<bool> {
        mod_api::__private::check_access(
            "mob_requires_custom_persistence",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_requires_custom_persistence(mob)
    }
    fn mob_no_ai(&self, mob: MobRef) -> Option<bool> {
        mod_api::__private::check_access("mob_no_ai", mod_api::__private::LoadAccess::WorldOnly);
        self.native_mob_no_ai(mob)
    }
    fn mob_restrict_center(&self, mob: MobRef) -> Option<[i32; 3]> {
        mod_api::__private::check_access(
            "mob_restrict_center",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_restrict_center(mob)
    }
    fn mob_restrict_radius(&self, mob: MobRef) -> Option<i32> {
        mod_api::__private::check_access(
            "mob_restrict_radius",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_restrict_radius(mob)
    }
    fn mob_has_restriction(&self, mob: MobRef) -> Option<bool> {
        mod_api::__private::check_access(
            "mob_has_restriction",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_has_restriction(mob)
    }
    fn mob_is_within_restriction(&self, mob: MobRef, at: [i32; 3]) -> Option<bool> {
        mod_api::__private::check_access(
            "mob_is_within_restriction",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_is_within_restriction(mob, at)
    }
    fn mob_left_handed(&self, mob: MobRef) -> Option<bool> {
        mod_api::__private::check_access(
            "mob_left_handed",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_left_handed(mob)
    }
    fn mob_can_pick_up_loot(&self, mob: MobRef) -> Option<bool> {
        mod_api::__private::check_access(
            "mob_can_pick_up_loot",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_can_pick_up_loot(mob)
    }
    fn mob_no_action_time(&self, mob: MobRef) -> Option<i32> {
        mod_api::__private::check_access(
            "mob_no_action_time",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_no_action_time(mob)
    }
    fn mob_raid(&self, mob: MobRef) -> Option<RaidRef> {
        mod_api::__private::check_access("mob_raid", mod_api::__private::LoadAccess::WorldOnly);
        self.native_mob_raid(mob)
    }
}
impl MobsBackendMut for Game<'_> {
    fn dragon_set_phase(&mut self, dragon: DragonRef, phase: NativeDragonPhase) -> bool {
        mod_api::__private::check_access(
            "dragon_set_phase",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_dragon_set_phase(dragon, phase)
    }
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
    fn mob_move_to(&mut self, mob: MobRef, at: [f64; 3], speed: f64) -> bool {
        mod_api::__private::check_access("mob_move_to", mod_api::__private::LoadAccess::WorldOnly);
        self.native_mob_move_to(mob, at, speed)
    }
    fn mob_stop(&mut self, mob: MobRef) -> bool {
        mod_api::__private::check_access("mob_stop", mod_api::__private::LoadAccess::WorldOnly);
        self.native_mob_stop(mob)
    }
    fn mob_set_navigation_speed(&mut self, mob: MobRef, speed: f64) -> bool {
        mod_api::__private::check_access(
            "mob_set_navigation_speed",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_set_navigation_speed(mob, speed)
    }
    fn mob_set_pathfinding_malus(&mut self, mob: MobRef, kind: PathType, malus: f32) -> bool {
        mod_api::__private::check_access(
            "mob_set_pathfinding_malus",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_set_pathfinding_malus(mob, kind, malus)
    }
    fn mob_set_target(&mut self, mob: MobRef, target: Option<LivingRef>) -> bool {
        mod_api::__private::check_access(
            "mob_set_target",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_set_target(mob, target)
    }
    fn mob_set_aggressive(&mut self, mob: MobRef, value: bool) -> bool {
        mod_api::__private::check_access(
            "mob_set_aggressive",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_set_aggressive(mob, value)
    }
    fn mob_set_persistence_required(&mut self, mob: MobRef) -> bool {
        mod_api::__private::check_access(
            "mob_set_persistence_required",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_set_persistence_required(mob)
    }
    fn mob_set_no_ai(&mut self, mob: MobRef, no_ai: bool) -> bool {
        mod_api::__private::check_access(
            "mob_set_no_ai",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_set_no_ai(mob, no_ai)
    }
    fn mob_set_baby(&mut self, mob: MobRef, baby: bool) -> bool {
        mod_api::__private::check_access("mob_set_baby", mod_api::__private::LoadAccess::WorldOnly);
        self.native_mob_set_baby(mob, baby)
    }
    fn mob_look_at(&mut self, mob: MobRef, at: [f64; 3]) -> bool {
        mod_api::__private::check_access("mob_look_at", mod_api::__private::LoadAccess::WorldOnly);
        self.native_mob_look_at(mob, at)
    }
    fn mob_jump(&mut self, mob: MobRef) -> bool {
        mod_api::__private::check_access("mob_jump", mod_api::__private::LoadAccess::WorldOnly);
        self.native_mob_jump(mob)
    }
    fn mob_restrict_to(&mut self, mob: MobRef, at: [i32; 3], radius: i32) -> bool {
        mod_api::__private::check_access(
            "mob_restrict_to",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_restrict_to(mob, at, radius)
    }
    fn mob_clear_restriction(&mut self, mob: MobRef) -> bool {
        mod_api::__private::check_access(
            "mob_clear_restriction",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_clear_restriction(mob)
    }
    fn mob_set_left_handed(&mut self, mob: MobRef, left_handed: bool) -> bool {
        mod_api::__private::check_access(
            "mob_set_left_handed",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_set_left_handed(mob, left_handed)
    }
    fn mob_set_can_pick_up_loot(&mut self, mob: MobRef, can_pick_up: bool) -> bool {
        mod_api::__private::check_access(
            "mob_set_can_pick_up_loot",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_set_can_pick_up_loot(mob, can_pick_up)
    }
    fn mob_set_no_action_time(&mut self, mob: MobRef, ticks: i32) -> bool {
        mod_api::__private::check_access(
            "mob_set_no_action_time",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_set_no_action_time(mob, ticks)
    }
    fn mob_set_brain_target(&mut self, mob: MobRef, target: Option<LivingRef>) -> bool {
        mod_api::__private::check_access(
            "mob_set_brain_target",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_mob_set_brain_target(mob, target)
    }
}
