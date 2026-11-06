//! The `local_sounds` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    /// Where a sound `play_local_sound` started is: waiting to start, playing, or over.
    #[helper(
        side = "client",
        receiver = "sound",
        target = "horizon_client::game::native_local_sounds::Game<'_>::native_sound_state"
    )]
    fn sound_state();
    /// Stops the sound for good: a waiting one never starts and a looping one never repeats.
    #[helper(
        side = "client",
        receiver = "sound",
        target = "horizon_client::game::native_local_sounds::Game<'_>::native_sound_stop"
    )]
    fn sound_stop();
    /// Fades the sound's volume to a target over some ticks; a fade to 0 stops it at the end.
    #[helper(
        side = "client",
        receiver = "sound",
        target = "horizon_client::game::native_local_sounds::Game<'_>::native_sound_fade"
    )]
    fn sound_fade();
    /// Every sound event the client's resources have (`SoundManager.getAvailableSounds`).
    #[helper(
        side = "client",
        target = "horizon_client::game::native_local_sounds::Game<'_>::native_sound_events"
    )]
    fn sound_events();
    /// The output devices sound can play on (`SoundEngine.getAvailableSoundDevices`).
    #[helper(
        side = "client",
        target = "horizon_client::game::native_local_sounds::Game<'_>::native_audio_devices"
    )]
    fn audio_devices();
    /// The output device the options ask for (`Options.soundDevice`).
    #[helper(
        side = "client",
        target = "horizon_client::game::native_local_sounds::Game<'_>::native_audio_device"
    )]
    fn audio_device();
}
