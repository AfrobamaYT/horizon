//! The game's side of the `level_reads` area (`horizon-mod-api/src/host/level_reads.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl LevelReadsBackend for Game<'_> {
    fn level_block_state(&self, level: LevelRef, pos: [i32; 3]) -> Option<BlockStateId> {
        mod_api::__private::check_access(
            "level_block_state",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_state(level, pos)
    }
    fn level_block_transform(
        &self,
        level: LevelRef,
        player: PlayerRef,
        hand: Hand,
        stack: &ItemStack,
        pos: [i32; 3],
        hit: BlockHit,
    ) -> Option<BlockTransformPreview> {
        mod_api::__private::check_access(
            "level_block_transform",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_block_transform(level, player, hand, stack, pos, hit)
    }
    fn level_block(&self, level: LevelRef, pos: [i32; 3]) -> Option<String> {
        mod_api::__private::check_access("level_block", mod_api::__private::LoadAccess::WorldOnly);
        self.native_level_block(level, pos)
    }
    fn level_block_property(&self, level: LevelRef, pos: [i32; 3], key: &str) -> Option<String> {
        mod_api::__private::check_access(
            "level_block_property",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_block_property(level, pos, key)
    }
    fn level_in_place(&self, level: LevelRef) -> bool {
        mod_api::__private::check_access(
            "level_in_place",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_in_place(level)
    }
    fn level_is_loaded(&self, level: LevelRef, pos: [i32; 3]) -> bool {
        mod_api::__private::check_access(
            "level_is_loaded",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_is_loaded(level, pos)
    }
    fn level_brightness(&self, level: LevelRef, layer: LightLayer, pos: [i32; 3]) -> Option<i32> {
        mod_api::__private::check_access(
            "level_brightness",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_brightness(level, layer, pos)
    }
    fn level_height(&self, level: LevelRef) -> Option<(i32, i32)> {
        mod_api::__private::check_access("level_height", mod_api::__private::LoadAccess::WorldOnly);
        self.native_level_height(level)
    }
    fn level_game_time(&self, level: LevelRef) -> Option<i64> {
        mod_api::__private::check_access(
            "level_game_time",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_game_time(level)
    }
    fn level_day_time(&self, level: LevelRef) -> Option<i64> {
        mod_api::__private::check_access(
            "level_day_time",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_day_time(level)
    }
    fn level_fluid_flow(&self, level: LevelRef, pos: [i32; 3]) -> Option<Vec3> {
        mod_api::__private::check_access(
            "level_fluid_flow",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_fluid_flow(level, pos)
    }
    fn level_in_world_bounds(&self, level: LevelRef, pos: [i32; 3]) -> Option<bool> {
        mod_api::__private::check_access(
            "level_in_world_bounds",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_in_world_bounds(level, pos)
    }
    fn level_in_spawnable_bounds(&self, level: LevelRef, pos: [i32; 3]) -> Option<bool> {
        mod_api::__private::check_access(
            "level_in_spawnable_bounds",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_in_spawnable_bounds(level, pos)
    }
}
