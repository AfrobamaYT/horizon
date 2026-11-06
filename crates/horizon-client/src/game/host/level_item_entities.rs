//! The game's side of the `level_item_entities` area (`horizon-mod-api/src/host/level_item_entities.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl LevelItemEntitiesBackendMut for Game<'_> {
    fn level_pop_resource(
        &mut self,
        level: LevelRef,
        pos: [i32; 3],
        stack: ItemStack,
    ) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "level_pop_resource",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_pop_resource(level, pos, stack)
    }
    fn level_spawn_item(
        &mut self,
        level: LevelRef,
        x: f64,
        y: f64,
        z: f64,
        stack: ItemStack,
        pickup_delay: u32,
    ) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "level_spawn_item",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_spawn_item(level, x, y, z, stack, pickup_delay)
    }
    fn level_spawn_item_with_motion(
        &mut self,
        level: LevelRef,
        x: f64,
        y: f64,
        z: f64,
        stack: ItemStack,
        motion: [f64; 3],
        pickup_delay: u32,
    ) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "level_spawn_item_with_motion",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_spawn_item_with_motion(level, x, y, z, stack, motion, pickup_delay)
    }
    fn level_award_experience(
        &mut self,
        level: LevelRef,
        x: f64,
        y: f64,
        z: f64,
        amount: u32,
    ) -> bool {
        mod_api::__private::check_access(
            "level_award_experience",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_award_experience(level, x, y, z, amount)
    }
}
