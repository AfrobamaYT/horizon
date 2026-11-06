//! The `level_effects` area: host methods the game implements (`game/host/level_effects.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' server writes of the `level_effects` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait LevelEffectsBackendMut {
    /// Side: server.
    /// Plays sound event `sound` at `at` for the players in this level
    /// within range (16 blocks, times the volume above 1), as
    /// `ServerLevel.playSound(null, ...)` does, under `source`'s volume
    /// slider. A `minecraft:` event must be registered; a mod's goes by name
    /// and is heard where its resources define it. True means queued, not
    /// heard.
    fn level_play_sound(
        &mut self,
        level: LevelRef,
        sound: &str,
        at: [f64; 3],
        source: SoundSource,
        volume: f32,
        pitch: f32,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::level_play_sound",
            "level_play_sound",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sends `count` particles around `at` to the players in this level
    /// within 32 blocks, as `ServerLevel.sendParticles` does: each offset is
    /// a Gaussian times `spread`, and `speed` scales their motion (a zero
    /// count sends one particle moving along `spread`). Block particle types
    /// need `state`; item, trail and vibration particles aren't supported.
    /// True means queued, not drawn.
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
        crate::phase::unprovided(
            "BackendMut::level_send_particles",
            "level_send_particles",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
