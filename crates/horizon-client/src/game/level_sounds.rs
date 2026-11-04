//! `ServerLevel.playSeededSound` for the players a server serves: the mods'
//! `play_level_sound_at_position` and `play_level_sound_at_entity`
//! (NeoForge's `PlayLevelSoundEvent`), then the sound packet.

use std::borrow::Cow;

use mod_api::{HookId, SoundSource};

use super::Game;
use super::native_hooks::mod_sound_ok;

/// A sound after the mods: what goes in the packet.
pub(super) struct HeardSound<'a> {
    pub name: Cow<'a, str>,
    pub source: usize,
    pub volume: f32,
    pub pitch: f32,
}

/// Whether a mod listens to either level sound hook.
fn listened() -> bool {
    mod_api::__private::listened(HookId::PlayLevelSoundAtPosition)
        || mod_api::__private::listened(HookId::PlayLevelSoundAtEntity)
}

impl Game<'_> {
    /// `Level.playSound`'s packet for the players near `at` in the level in
    /// place, but the player on connection `except`, once the mods allowed
    /// it. `source` is the sound's index in `SoundSource` order. `emitter` is the entity vanilla binds the sound to
    /// (`playSound(except, sourceEntity, ...)`); none for a sound at a
    /// position, as `Entity.playSound`'s.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn send_level_sound(
        &mut self,
        name: &str,
        at: [f64; 3],
        volume: f32,
        pitch: f32,
        source: usize,
        emitter: Option<u32>,
        except: Option<u32>,
    ) {
        let dimension = self.dim.id();
        let Some(sound) = self.ask_level_sound(dimension, name, at, source, volume, pitch, emitter)
        else {
            return;
        };
        if let Some(host) = self.lan.as_ref() {
            host.hub.sound(
                &sound.name,
                at,
                sound.volume,
                sound.pitch,
                sound.source as i32,
                except,
            );
        }
    }

    /// `ServerLevel.playSeededSound`'s `EventHooks.onPlaySoundAtEntity` or
    /// `onPlaySoundAtPosition`: `play_level_sound_at_entity` when `emitter`
    /// is an entity a mod can name, `play_level_sound_at_position` otherwise.
    /// None when a mod cancels it or leaves it unplayable (not a sound id,
    /// or a volume or pitch that isn't finite and at least 0). With no
    /// listener, the sound as asked, borrowed.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn ask_level_sound<'a>(
        &mut self,
        dimension: &str,
        name: &'a str,
        at: [f64; 3],
        source: usize,
        volume: f32,
        pitch: f32,
        emitter: Option<u32>,
    ) -> Option<HeardSound<'a>> {
        if !listened() {
            return Some(HeardSound {
                name: Cow::Borrowed(name),
                source,
                volume,
                pitch,
            });
        }
        // The mods see a namespaced id (`minecraft:entity.pig.ambient`).
        let mut sound = if name.contains(':') {
            name.to_owned()
        } else {
            format!("minecraft:{name}")
        };
        let mut source_now = SoundSource::ALL
            .get(source)
            .copied()
            .unwrap_or(SoundSource::Master);
        let (mut volume_now, mut pitch_now, mut cancel) = (volume, pitch, false);
        match emitter.and_then(|id| self.native_entity(id)) {
            Some(entity) => self.native_play_level_sound_at_entity(
                entity,
                &mut sound,
                &mut source_now,
                &mut volume_now,
                &mut pitch_now,
                volume,
                pitch,
                &mut cancel,
            ),
            None => {
                let level = self.native_level(dimension)?;
                self.native_play_level_sound_at_position(
                    level,
                    at,
                    &mut sound,
                    &mut source_now,
                    &mut volume_now,
                    &mut pitch_now,
                    volume,
                    pitch,
                    &mut cancel,
                );
            }
        }
        let source = SoundSource::ALL
            .iter()
            .position(|&s| s == source_now)
            .unwrap_or(0);
        if cancel || !mod_sound_ok(&sound, at, source, volume_now, pitch_now) {
            return None;
        }
        Some(HeardSound {
            name: Cow::Owned(sound),
            source,
            volume: volume_now,
            pitch: pitch_now,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::lan::LanSettings;
    use super::super::native_entity_refs::tests::with_game;
    use super::super::native_sound_refs::tests::{guest, packets};
    use horizon_client::lan::{NetConfig, hub::HubConfig};
    use horizon_client::mobs::Mob;
    use horizon_core::geometry::Vec3;
    use horizon_protocol::packets::play::Clientbound;
    use mod_api::{NativeSoundSource, ServerHost, SoundSource};
    use std::sync::{Arc, Mutex};

    /// Quietens and moves the locked chest, cancels the pig, and turns a
    /// sound bound to an entity into a cow's; logs what it was asked.
    struct Sounds(Arc<Mutex<Vec<(&'static str, String, Option<u32>)>>>);
    #[mod_api::__subscriptions]
    impl mod_api::Hooks for Sounds {
        fn play_level_sound_at_position(
            &self,
            _: &mut dyn ServerHost,
            e: &mut mod_api::PlayLevelSoundAtPositionEvent,
        ) -> mod_api::Flow {
            self.0
                .lock()
                .unwrap()
                .push(("position", e.sound.clone(), None));
            match e.sound.as_str() {
                "minecraft:block.chest.locked" => {
                    assert_eq!((e.original_volume, e.original_pitch), (1.0, 1.0));
                    (e.volume, e.pitch, *e.source) = (0.25, 2.0, SoundSource::Ambient);
                    mod_api::Flow::Continue
                }
                "minecraft:entity.pig.ambient" => mod_api::Flow::Cancel,
                _ => mod_api::Flow::Continue,
            }
        }
        fn play_level_sound_at_entity(
            &self,
            _: &mut dyn ServerHost,
            e: &mut mod_api::PlayLevelSoundAtEntityEvent,
        ) -> mod_api::Flow {
            self.0
                .lock()
                .unwrap()
                .push(("entity", e.sound.clone(), Some(e.entity.id())));
            *e.sound = "minecraft:entity.cow.ambient".into();
            mod_api::Flow::Continue
        }
    }

    /// `PlayLevelSoundEvent`: the tick's sounds are asked at a position, a
    /// sound bound to an entity at that entity; the players get the mod's
    /// sound, and none for a cancelled one.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_mod_changes_and_cancels_level_sounds() {
        let heard = Arc::default();
        let mods = vec![mod_api::ModEntry::new(
            "sounds",
            Box::new(Sounds(Arc::clone(&heard))),
        )]
        .into();
        mod_api::testing::with_mods(mods, || {
            with_game(|game| {
                let net = NetConfig {
                    listen: false,
                    announce: false,
                    lod_distance: 0,
                    ..NetConfig::lan(0, String::new(), 3)
                };
                let config = HubConfig {
                    default_game_mode: 0,
                    guest_commands: false,
                    force_game_mode: false,
                    hardcore: false,
                    view_distance: 2,
                    simulation_distance: 2,
                    dimension: game.dim.id().to_owned(),
                    allow_flight: true,
                    seed: 0,
                    max_players: 3,
                    integrated: true,
                    levels: game.level_shapes,
                };
                let settings = LanSettings {
                    port: 0,
                    guest_commands: false,
                    force_game_mode: false,
                };
                game.lan_host(settings, net, config).unwrap();
                let (a, rx) = guest(21, 21, [1.0, 64.0, 2.0]);
                game.lan.as_mut().unwrap().hub.guests.push(a);
                let at = [2.0, 64.0, 2.0];

                // The tick's sounds, at their positions.
                game.audio.capture = Some(vec![
                    ("minecraft:block.chest.locked".into(), at, 1.0, 1.0),
                    ("minecraft:entity.pig.ambient".into(), at, 1.0, 1.0),
                ]);
                game.lan_broadcast_effects();
                let got = packets(&rx);
                assert_eq!(got.len(), 1, "the pig is cancelled");
                let Clientbound::Sound(p) = &got[0] else {
                    panic!("expected a sound")
                };
                assert_eq!((p.volume, p.pitch), (0.25, 2.0));
                assert_eq!(p.source, SoundSource::Ambient as i32);

                // A mod's sound bound to a cow, asked at the cow.
                let kind = game.res.mob_types().get("minecraft:cow").unwrap().clone();
                game.mobs
                    .push(Mob::new(40, kind, Vec3::new(2.0, 64.0, 2.0), 0.0));
                let cow = game.native_entity(40).unwrap();
                assert!(game.native_entity_play_bound_sound(
                    cow,
                    "minecraft:entity.player.burp",
                    NativeSoundSource::Neutral,
                    1.0,
                    1.0
                ));
                game.flush_native_sounds();
                let got = packets(&rx);
                assert_eq!(got.len(), 1);
                let Clientbound::SoundEntity(p) = &got[0] else {
                    panic!("expected a bound sound")
                };
                assert_eq!(p.id, 40);
                let cow_ambient = game.lan.as_ref().unwrap().hub.sound_packet(
                    "minecraft:entity.cow.ambient",
                    at,
                    1.0,
                    1.0,
                    0,
                    0,
                );
                let Clientbound::Sound(expected) = cow_ambient else {
                    panic!("expected a sound")
                };
                assert_eq!(format!("{:?}", p.sound), format!("{:?}", expected.sound));

                assert_eq!(
                    *heard.lock().unwrap(),
                    [
                        ("position", "minecraft:block.chest.locked".into(), None),
                        ("position", "minecraft:entity.pig.ambient".into(), None),
                        ("entity", "minecraft:entity.player.burp".into(), Some(40)),
                    ]
                );
            });
        });
    }
}
