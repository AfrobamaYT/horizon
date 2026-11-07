//! The game's side of the `effects` area (`horizon-mod-api/src/host/effects.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl EffectsClientHost for Game<'_> {
    fn flame_particle(&mut self, at: [f64; 3], soul: bool) {
        mod_api::__private::check_access(
            "flame_particle",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_flame_particle(at, soul)
    }
    fn play_local_sound(&self, sound: &SoundPlay, after: u32) -> Option<SoundRef> {
        mod_api::__private::check_access(
            "play_local_sound",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_play_local_sound(sound, after)
    }
    fn particle_count(&self) -> u32 {
        mod_api::__private::check_access(
            "particle_count",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_particle_count()
    }
    fn clear_particles(&self) -> bool {
        mod_api::__private::check_access(
            "clear_particles",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_clear_particles()
    }
    fn add_particle(
        &mut self,
        options: &ParticleOptions,
        at: [f64; 3],
        velocity: [f64; 3],
    ) -> bool {
        mod_api::__private::check_access("add_particle", mod_api::__private::LoadAccess::WorldOnly);
        self.native_add_particle(options, at, velocity)
    }
    fn sound_events(&self) -> Vec<Identifier> {
        mod_api::__private::check_access(
            "sound_events",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_sound_events()
    }
    fn audio_devices(&self) -> Vec<String> {
        mod_api::__private::check_access(
            "audio_devices",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_audio_devices()
    }
    fn audio_device(&self) -> String {
        mod_api::__private::check_access(
            "audio_device",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_audio_device()
    }
}
impl EffectsClientBackend for Game<'_> {
    fn sound_state(&self, sound: SoundRef) -> Option<SoundState> {
        mod_api::__private::check_access(
            "sound_state",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_sound_state(sound)
    }
    fn sound_stop(&self, sound: SoundRef) -> bool {
        mod_api::__private::check_access("sound_stop", mod_api::__private::LoadAccess::ClientSetup);
        self.native_sound_stop(sound)
    }
    fn sound_fade(&self, sound: SoundRef, volume: f32, ticks: u32) -> bool {
        mod_api::__private::check_access("sound_fade", mod_api::__private::LoadAccess::ClientSetup);
        self.native_sound_fade(sound, volume, ticks)
    }
}
