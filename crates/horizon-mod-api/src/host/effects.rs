//! The `effects` area: host methods the game implements (`game/host/effects.rs`).
#![allow(unused_variables)]
use crate::*;
/// The client host's methods of the `effects` area, implemented by the game and reached through [`crate::ClientHost`].
pub trait EffectsClientHost {
    /// Side: client.
    /// A small local vanilla particle; caller should rate-limit emission.
    fn flame_particle(&mut self, at: [f64; 3], soul: bool) {
        crate::phase::unprovided(
            "ClientHost::flame_particle",
            "flame_particle",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// Plays `sound` for this player only (`SoundPlay`'s event: `minecraft:ui.button.click`,
    /// or a mod's own from its `assets/<id>/sounds.json`): nothing is sent to the server, and
    /// other players don't hear it. It takes `&self`, so a screen, key or mouse hook can play
    /// one: the sound is queued and starts at the next audio update (once a client tick),
    /// `after` ticks counted from then (0 to 1200, a minute; `SoundEngine.playDelayed`). `SoundPlay::ui` plays it at the listener as an interface sound
    /// (`SimpleSoundInstance.forUI`); `SoundPlay::at` at a position in the client's level,
    /// fading with distance (`volume` past 1 carries farther, 16 blocks times the volume), as
    /// thunder heard after the flash or a blast heard when its shock wave arrives. False,
    /// with nothing played, on a server or for a value out of range; an event the resources
    /// don't have plays nothing, as vanilla's.
    fn play_local_sound(&self, sound: &SoundPlay, after: u32) -> bool {
        crate::phase::unprovided(
            "ClientHost::play_local_sound",
            "play_local_sound",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// The particles alive in the client's level (`ParticleEngine.countParticles`): 0 once
    /// a clear is asked, and on a server.
    fn particle_count(&self) -> u32 {
        crate::phase::unprovided(
            "ClientHost::particle_count",
            "particle_count",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// `ParticleEngine.clearParticles`: every particle and emitter goes before the next
    /// frame is drawn or the next particle tick runs, whichever comes first. It takes
    /// `&self`, so a screen or render hook can ask, as with `native_play_local_sound`.
    /// False on a server.
    fn clear_particles(&self) -> bool {
        crate::phase::unprovided(
            "ClientHost::clear_particles",
            "clear_particles",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// ClientLevel.addParticle: submits one local particle through the ordinary provider, camera-distance, setting and population limits. False without a loaded client world or for non-finite input. True means submitted, not necessarily visible.
    fn add_particle(
        &mut self,
        options: &ParticleOptions,
        at: [f64; 3],
        velocity: [f64; 3],
    ) -> bool {
        crate::phase::unprovided(
            "ClientHost::add_particle",
            "add_particle",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
