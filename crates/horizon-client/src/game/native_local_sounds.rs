//! Client mods' own sounds (`play_local_sound`) and their `SoundRef` handles: what each is
//! doing, stopping and fading it, and the sounds and devices the client has.
use super::Game;
use horizon_client::sound_engine::InstanceId;
use horizon_world::identifier::Identifier;
use mod_api::__private::game::*;
use mod_api::{SoundPlay, SoundRef, SoundState};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

/// What a mod asked of one of its sounds, applied at the next audio update.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Change {
    Stop,
    Fade { volume: f32, ticks: u32 },
}
impl Change {
    /// It ends the sound when applied: a stop, or a fade to 0 at once.
    fn stops(self) -> bool {
        match self {
            Change::Stop => true,
            Change::Fade { volume, ticks } => ticks == 0 && volume <= 0.0,
        }
    }
}

/// Client mods' sounds: those asked for since the last audio update, the changes asked of
/// them, and each started one's engine instance by its handle.
pub(super) struct ModSounds {
    /// The last handle given; handles are never reused.
    last: Cell<u64>,
    /// Sounds asked for (`native_play_local_sound`), with their handle and the ticks each
    /// waits, until the next audio update plays them (`apply_mod_sounds`).
    pub(super) queued: RefCell<Vec<(u64, SoundPlay, u32)>>,
    /// Stops and fades asked for since the last audio update, in order.
    changes: RefCell<Vec<(u64, Change)>>,
    /// Each started sound's engine instance, until the engine lets it go.
    started: HashMap<u64, InstanceId>,
}
impl Default for ModSounds {
    fn default() -> Self {
        Self {
            last: Cell::new(0),
            queued: RefCell::default(),
            changes: RefCell::default(),
            started: HashMap::new(),
        }
    }
}
impl ModSounds {
    /// Queues `sound` to start at the next audio update, `after` ticks from then.
    pub(super) fn queue(&self, sound: &SoundPlay, after: u32) -> SoundRef {
        let id = self.last.get() + 1;
        self.last.set(id);
        self.queued.borrow_mut().push((id, sound.clone(), after));
        sound_ref(id)
    }
}

impl Game<'_> {
    /// Plays the sounds client mods asked for since the last update, then applies their stops
    /// and fades, and forgets the instances the engine let go. Called from the audio update
    /// (`tick_sound_engine`), before the engine's own tick.
    pub(super) fn apply_mod_sounds(&mut self) {
        let sounds = &mut self.mod_sounds;
        for (id, sound, after) in sounds.queued.get_mut().drain(..) {
            let instance = self.audio.play_mod_sound(&sound, after);
            sounds.started.insert(id, instance);
        }
        for (id, change) in sounds.changes.get_mut().drain(..) {
            let Some(&instance) = sounds.started.get(&id) else {
                continue;
            };
            match change {
                Change::Stop => self.audio.stop_mod_sound(instance),
                Change::Fade { volume, ticks } => {
                    self.audio.fade_mod_sound(instance, volume, ticks)
                }
            }
            if change.stops() {
                sounds.started.remove(&id);
            }
        }
        let audio = &self.audio;
        sounds
            .started
            .retain(|_, instance| audio.mod_sound_state(*instance) != SoundState::Stopped);
    }
    /// For a client mod: where its sound is. A stop or a fade to 0 at once asked since the last
    /// audio update already reads `Stopped`. None on a server.
    pub(super) fn native_sound_state(&self, sound: SoundRef) -> Option<SoundState> {
        if !self.native_window() {
            return None;
        }
        let id = sound.fields().id;
        let sounds = &self.mod_sounds;
        if sounds
            .changes
            .borrow()
            .iter()
            .any(|&(changed, change)| changed == id && change.stops())
        {
            return Some(SoundState::Stopped);
        }
        if sounds
            .queued
            .borrow()
            .iter()
            .any(|(queued, ..)| *queued == id)
        {
            return Some(SoundState::Waiting);
        }
        Some(
            sounds
                .started
                .get(&id)
                .map_or(SoundState::Stopped, |&instance| {
                    self.audio.mod_sound_state(instance)
                }),
        )
    }
    /// For a client mod: stops its sound for good at the next audio update. A waiting sound
    /// never starts and a looping one never repeats. False, changing nothing, for a sound that
    /// is already over, and on a server.
    pub(super) fn native_sound_stop(&self, sound: SoundRef) -> bool {
        self.change_mod_sound(sound, Change::Stop)
    }
    /// For a client mod: from the next audio update, its sound's volume moves to `volume` (at
    /// or above 0, as `SoundPlay`'s) in equal steps over the next `ticks` ticks it plays, or at
    /// once for 0. A fade to 0 stops it when the fade ends, as `LoopSoundInstance.fadeOut` does;
    /// a fade up from a `SoundPlay` at volume 0 fades it in. A later fade replaces one in
    /// progress. False, changing nothing, for a volume that isn't a number at or above 0, a sound
    /// that is already over, and on a server.
    pub(super) fn native_sound_fade(&self, sound: SoundRef, volume: f32, ticks: u32) -> bool {
        if !volume.is_finite() || volume < 0.0 {
            return false;
        }
        self.change_mod_sound(sound, Change::Fade { volume, ticks })
    }
    fn change_mod_sound(&self, sound: SoundRef, change: Change) -> bool {
        match self.native_sound_state(sound) {
            Some(SoundState::Waiting | SoundState::Playing) => {
                let id = sound.fields().id;
                self.mod_sounds.changes.borrow_mut().push((id, change));
                true
            }
            _ => false,
        }
    }
    /// For a client mod: every sound event the client's resources have
    /// (`SoundManager.getAvailableSounds`), sorted. It builds a new list each call: ask from
    /// setup or a settings screen, not every frame. Empty on a server.
    pub(super) fn native_sound_events(&self) -> Vec<Identifier> {
        if !self.native_window() {
            return Vec::new();
        }
        self.audio
            .event_names()
            .iter()
            .filter_map(|name| Identifier::parse(name).ok())
            .collect()
    }
    /// For a client mod: the output devices sound can play on, by name
    /// (`SoundEngine.getAvailableSoundDevices`). Listing them probes every device (tens of
    /// milliseconds on some systems): ask from a settings screen, not every frame. Empty on a
    /// server and where sound is off.
    pub(super) fn native_audio_devices(&self) -> Vec<String> {
        if !self.native_window() {
            return Vec::new();
        }
        horizon_client::audio::output_devices()
    }
    /// For a client mod: the output device the options ask for (`Options.soundDevice`), empty
    /// for the system's default. The sound plays on the default when that one can't be opened.
    /// Changing it keeps every mod sound playing, with its handle. Empty on a server.
    pub(super) fn native_audio_device(&self) -> String {
        if !self.native_window() {
            return String::new();
        }
        self.audio.device().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use mod_api::__private::game::{Fields, sound_ref};
    use mod_api::{
        ClientHost, Flow, Hooks, ModEntry, RenderGuiPreEvent, SoundPlay, SoundRef, SoundState,
        testing::with_mods,
    };
    use std::sync::{Arc, Mutex};

    /// From a screen hook's read-only host: starts two sounds, stops one and fades the other,
    /// keeping their handles.
    struct Player(Arc<Mutex<Vec<SoundRef>>>);
    #[mod_api::__subscriptions]
    impl Hooks for Player {
        fn render_gui_pre(&self, host: &dyn ClientHost, _: &mut RenderGuiPreEvent) -> Flow {
            let click = SoundPlay::ui("minecraft:ui.button.click");
            let stopped = host.play_local_sound(&click, 40).unwrap();
            let faded = host.play_local_sound(&click.clone().looping(0), 0).unwrap();
            assert_eq!(stopped.state(host), Some(SoundState::Waiting), "queued");
            assert!(stopped.stop(host));
            assert_eq!(
                stopped.state(host),
                Some(SoundState::Stopped),
                "a stop reads at once"
            );
            assert!(!stopped.stop(host), "already over");
            assert!(!stopped.fade(host, 0.5, 10), "already over");
            assert!(!faded.fade(host, f32::NAN, 10));
            assert!(!faded.fade(host, -1.0, 10));
            assert!(faded.fade(host, 0.0, 20));
            assert_eq!(
                faded.state(host),
                Some(SoundState::Waiting),
                "fading out, not over"
            );
            assert!(
                host.play_local_sound(&SoundPlay::ui("not an id"), 0)
                    .is_none()
            );
            let events = host.sound_events();
            assert!(
                events
                    .iter()
                    .any(|e| e.to_string() == "minecraft:ui.button.click")
            );
            let _ = (host.audio_devices(), host.audio_device());
            self.0.lock().unwrap().extend([stopped, faded]);
            Flow::Continue
        }
    }

    /// `SoundRef`: a handle per play, its state, stop and fade asked from any client hook and
    /// applied at the next audio update; a stopped play never starts. Unknown handles and a
    /// server read as over or none.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_client_mod_follows_stops_and_fades_its_own_sounds() {
        let kept = Arc::new(Mutex::new(Vec::new()));
        let mods: mod_api::Mods =
            vec![ModEntry::new("player", Box::new(Player(Arc::clone(&kept))))].into();
        with_mods(mods, || {
            super::super::native_entity_refs::tests::with_client_game(|game| {
                game.native_render_gui_pre(&mut false);
                let [stopped, faded] = kept.lock().unwrap()[..] else {
                    panic!("two handles");
                };
                assert_ne!(stopped, faded, "a handle per play");
                game.tick_sound_engine();
                assert!(game.mod_sounds.queued.borrow().is_empty());
                assert_eq!(game.native_sound_state(stopped), Some(SoundState::Stopped));
                assert!(
                    !game.mod_sounds.started.contains_key(&stopped.fields().id),
                    "a stop lets the play go"
                );
                // Without an output device (a test machine) the engine starts nothing, so the
                // faded loop is over here; with one it plays until its fade ends.
                assert_ne!(game.native_sound_state(faded), Some(SoundState::Waiting));
                // A handle no play has, and a server.
                let unknown = sound_ref(999_999);
                assert_eq!(game.native_sound_state(unknown), Some(SoundState::Stopped));
                assert!(!game.native_sound_stop(unknown));
                game.options.execution_role =
                    super::super::execution_role::ExecutionRole::DedicatedServer;
                assert_eq!(game.native_sound_state(faded), None);
                assert!(
                    game.native_play_local_sound(&SoundPlay::ui("minecraft:ui.button.click"), 0)
                        .is_none()
                );
                assert!(game.native_sound_events().is_empty());
            });
        });
    }
}
