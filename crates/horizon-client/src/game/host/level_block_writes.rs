//! The game's side of the `level_block_writes` area (`horizon-mod-api/src/host/level_block_writes.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl LevelBlockWritesBackendMut for Game<'_> {
    fn level_set_block(
        &mut self,
        level: LevelRef,
        pos: [i32; 3],
        state: BlockStateId,
        flags: NativeBlockUpdateFlags,
    ) -> bool {
        mod_api::__private::check_access(
            "level_set_block",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_set_block(level, pos, state, flags)
    }
    fn level_remove_block(&mut self, level: LevelRef, pos: [i32; 3], moved: bool) -> bool {
        mod_api::__private::check_access(
            "level_remove_block",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_remove_block(level, pos, moved)
    }
    fn level_destroy_block(
        &mut self,
        level: LevelRef,
        pos: [i32; 3],
        drop: bool,
        tool: Option<&ItemStack>,
        collect: bool,
    ) -> Option<Vec<ItemStack>> {
        mod_api::__private::check_access(
            "level_destroy_block",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_destroy_block(level, pos, drop, tool, collect)
    }
    fn level_set_break_progress(
        &mut self,
        level: LevelRef,
        breaker: i32,
        pos: [i32; 3],
        progress: i32,
    ) -> bool {
        mod_api::__private::check_access(
            "level_set_break_progress",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_set_break_progress(level, breaker, pos, progress)
    }
}
