//! The game's side of the `level_signals` area (`horizon-mod-api/src/host/level_signals.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl LevelSignalsBackend for Game<'_> {
    fn level_signal(
        &self,
        level: LevelRef,
        pos: [i32; 3],
        direction: Direction,
    ) -> Option<i32> {
        mod_api::__private::check_access(
            "level_signal",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_signal(level, pos, direction)
    }
    fn level_direct_signal(
        &self,
        level: LevelRef,
        pos: [i32; 3],
        direction: Direction,
    ) -> Option<i32> {
        mod_api::__private::check_access(
            "level_direct_signal",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_direct_signal(level, pos, direction)
    }
    fn level_direct_signal_to(&self, level: LevelRef, pos: [i32; 3]) -> Option<i32> {
        mod_api::__private::check_access(
            "level_direct_signal_to",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_direct_signal_to(level, pos)
    }
    fn level_best_neighbor_signal(&self, level: LevelRef, pos: [i32; 3]) -> Option<i32> {
        mod_api::__private::check_access(
            "level_best_neighbor_signal",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_best_neighbor_signal(level, pos)
    }
    fn level_has_neighbor_signal(&self, level: LevelRef, pos: [i32; 3]) -> bool {
        mod_api::__private::check_access(
            "level_has_neighbor_signal",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_has_neighbor_signal(level, pos)
    }
}
