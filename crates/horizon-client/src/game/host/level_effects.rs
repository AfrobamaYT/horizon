//! The game's side of the `level_effects` area (`horizon-mod-api/src/host/level_effects.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl LevelEffectsBackendMut for Game<'_> {
    fn level_play_sound(
        &mut self,
        level: LevelRef,
        sound: &str,
        at: [f64; 3],
        source: SoundSource,
        volume: f32,
        pitch: f32,
    ) -> bool {
        mod_api::__private::check_access(
            "level_play_sound",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_play_sound(level, sound, at, source, volume, pitch)
    }
    fn level_send_particles(
        &mut self,
        level: LevelRef,
        options: &ParticleOptions,
        state: Option<BlockStateId>,
        at: [f64; 3],
        spread: [f64; 3],
        speed: f64,
        count: i32,
    ) -> bool {
        mod_api::__private::check_access(
            "level_send_particles",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_send_particles(level, options, state, at, spread, speed, count)
    }
}
