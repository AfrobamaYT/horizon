//! `ServerLevel.playSeededSound` for the players a server serves: the mods'
//! `play_level_sound_at_position` and `play_level_sound_at_entity`
//! (NeoForge's `PlayLevelSoundEvent`), then the sound packet.

use std::borrow::Cow;

use mod_api::{HookId, Identifier, SoundSource};

use super::Game;
use super::native_hooks::level_sound_playable;

/// A sound the server's level is about to play, as vanilla asked for it.
pub(super) struct LevelSound<'a> {
    pub name: &'a str,
    pub at: [f64; 3],
    pub volume: f32,
    pub pitch: f32,
    pub source: SoundSource,
    /// The entity vanilla binds the sound to (`playSound(except, sourceEntity, ...)`);
    /// none for a sound at a position, as `Entity.playSound`'s.
    pub emitter: Option<u32>,
}

impl<'a> LevelSound<'a> {
    /// A sound at `at` in the source its event name belongs to.
    pub fn new(name: &'a str, at: [f64; 3], volume: f32, pitch: f32) -> Self {
        let source = SoundSource::ALL
            .get(horizon_client::settings::sound_source(name))
            .copied()
            .unwrap_or(SoundSource::Master);
        Self {
            name,
            at,
            volume,
            pitch,
            source,
            emitter: None,
        }
    }

    pub fn in_source(self, source: SoundSource) -> Self {
        Self { source, ..self }
    }

    pub fn bound_to(self, emitter: u32) -> Self {
        Self {
            emitter: Some(emitter),
            ..self
        }
    }
}

/// A sound after the mods: what goes in the packet.
pub(super) struct HeardSound<'a> {
    pub name: Cow<'a, str>,
    pub source: SoundSource,
    pub volume: f32,
    pub pitch: f32,
}

/// Whether a mod listens to either level sound hook.
fn listened() -> bool {
    mod_api::__private::listened(HookId::PlayLevelSoundAtPosition)
        || mod_api::__private::listened(HookId::PlayLevelSoundAtEntity)
}

impl Game<'_> {
    /// `Level.playSound`'s packet for the players near the sound's position in
    /// the level in place, but the player on connection `except`, once the
    /// mods allowed it.
    pub(super) fn send_level_sound(&mut self, sound: LevelSound<'_>, except: Option<u32>) {
        let at = sound.at;
        let dimension = self.dim.id();
        let Some(heard) = self.ask_level_sound(dimension, sound) else {
            return;
        };
        if let Some(host) = self.lan.as_ref() {
            host.hub.sound(
                &heard.name,
                at,
                heard.volume,
                heard.pitch,
                heard.source as i32,
                except,
            );
        }
    }

    /// `ServerLevel.playSeededSound`'s `EventHooks.onPlaySoundAtEntity` or
    /// `onPlaySoundAtPosition`: `play_level_sound_at_entity` when the sound's
    /// emitter is an entity a mod can name, `play_level_sound_at_position`
    /// otherwise. None when a mod cancels it or leaves it unplayable (a volume
    /// or pitch that isn't finite and at least 0). With no listener, the sound
    /// as asked, borrowed.
    pub(super) fn ask_level_sound<'a>(
        &mut self,
        dimension: &str,
        sound: LevelSound<'a>,
    ) -> Option<HeardSound<'a>> {
        let LevelSound {
            name,
            at,
            volume,
            pitch,
            source,
            emitter,
        } = sound;
        let asked = HeardSound {
            name: Cow::Borrowed(name),
            source,
            volume,
            pitch,
        };
        if !listened() {
            return Some(asked);
        }
        // The mods see a namespaced id (`minecraft:entity.pig.ambient`).
        let Ok(mut id) = Identifier::parse(name) else {
            return Some(asked);
        };
        let (mut source_now, mut volume_now, mut pitch_now, mut cancel) =
            (source, volume, pitch, false);
        match emitter.and_then(|id| self.native_entity(id)) {
            Some(entity) => self.native_play_level_sound_at_entity(
                mod_api::__private::PlayLevelSoundAtEntityCall {
                    entity,
                    sound: &mut id,
                    source: &mut source_now,
                    volume: &mut volume_now,
                    pitch: &mut pitch_now,
                    original_volume: volume,
                    original_pitch: pitch,
                    cancel: &mut cancel,
                },
            ),
            None => {
                let level = self.native_level(dimension)?;
                self.native_play_level_sound_at_position(
                    mod_api::__private::PlayLevelSoundAtPositionCall {
                        level,
                        position: at,
                        sound: &mut id,
                        source: &mut source_now,
                        volume: &mut volume_now,
                        pitch: &mut pitch_now,
                        original_volume: volume,
                        original_pitch: pitch,
                        cancel: &mut cancel,
                    },
                );
            }
        }
        if cancel || !level_sound_playable(at, volume_now, pitch_now) {
            return None;
        }
        Some(HeardSound {
            name: Cow::Owned(id.to_string()),
            source: source_now,
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
                .push(("position", e.sound.to_string(), None));
            match e.sound.to_string().as_str() {
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
                .push(("entity", e.sound.to_string(), Some(e.entity.id())));
            *e.sound = mod_api::Identifier::parse("minecraft:entity.cow.ambient").unwrap();
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
                let (b, rx_b) = guest(22, 22, [1.0, 64.0, 3.0]);
                game.lan.as_mut().unwrap().hub.guests.push(b);
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

                // `Player.playSound` for the first guest (eating): the other
                // guest hears it, and the mods were asked first.
                game.as_guest(21, |game| {
                    game.guest_player_sounds.push((
                        "minecraft:entity.generic.eat".into(),
                        at,
                        1.0,
                        1.0,
                    ));
                })
                .unwrap();
                let got = packets(&rx_b);
                let Some(Clientbound::Sound(p)) = got.last() else {
                    panic!("expected the guest's sound")
                };
                assert_eq!(p.source, SoundSource::Players as i32);
                assert!(packets(&rx).is_empty(), "not the guest who made it");

                assert_eq!(
                    *heard.lock().unwrap(),
                    [
                        ("position", "minecraft:block.chest.locked".into(), None),
                        ("position", "minecraft:entity.pig.ambient".into(), None),
                        ("entity", "minecraft:entity.player.burp".into(), Some(40)),
                        ("position", "minecraft:entity.generic.eat".into(), None),
                    ]
                );
            });
        });
    }
}
