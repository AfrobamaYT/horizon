//! The game's side of the `level_events` area (`horizon-mod-api/src/host/level_events.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl LevelEventsBackendMut for Game<'_> {
    fn level_level_event(
        &mut self,
        level: LevelRef,
        id: i32,
        pos: [i32; 3],
        data: i32,
    ) -> bool {
        mod_api::__private::check_access(
            "level_level_event",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_level_event(level, id, pos, data)
    }
    fn level_game_event(&mut self, level: LevelRef, event: &str, at: [f64; 3]) -> bool {
        mod_api::__private::check_access(
            "level_game_event",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_game_event(level, event, at)
    }
    fn level_explode(
        &mut self,
        level: LevelRef,
        center: [f64; 3],
        radius: f32,
        fire: bool,
        interaction: NativeExplosionInteraction,
    ) -> bool {
        mod_api::__private::check_access(
            "level_explode",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_explode(level, center, radius, fire, interaction)
    }
    fn level_strike_lightning(
        &mut self,
        level: LevelRef,
        at: [f64; 3],
        visual_only: bool,
    ) -> bool {
        mod_api::__private::check_access(
            "level_strike_lightning",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_strike_lightning(level, at, visual_only)
    }
}
