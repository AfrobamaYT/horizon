//! The `level_sounds` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// A sound is about to play at `position` in `level` (NeoForge's
    /// `PlayLevelSoundEvent.AtPosition`, in `ServerLevel.playSeededSound`):
    /// every server sound not bound to an entity, mobs' own sounds included
    /// (`Entity.playSound` plays at the entity's position). A mod may change
    /// `sound`, `source`, `volume` and `pitch`, or set `cancel` and no player
    /// hears it; `original_volume` and `original_pitch` are as asked. A sound
    /// left unplayable (a volume or pitch that isn't finite and at least 0)
    /// is dropped. Sounds the server's level plays during
    /// the tick are asked as they go out, at the end of the tick, with the state
    /// then. `/playsound` sends its packet directly and isn't asked, as in
    /// vanilla. Server-side.
    #[hook(
        side = "server",
        contract = "cancel",
        out = "cancel",
        neoforge = "PlayLevelSoundEvent.AtPosition",
        cancellable = true
    )]
    fn play_level_sound_at_position(
        level: LevelRef,
        position: [f64; 3],
        sound: &mut Identifier,
        source: &mut SoundSource,
        volume: &mut f32,
        pitch: &mut f32,
        original_volume: f32,
        original_pitch: f32,
        cancel: &mut bool,
    );
    /// A sound bound to `entity` is about to play (NeoForge's
    /// `PlayLevelSoundEvent.AtEntity`, in `ServerLevel.playSeededSound`'s entity
    /// overload, `playSound(except, sourceEntity, ...)`): a goat horn and a mod's
    /// sound bound to an entity. The fields and their rules are
    /// `play_level_sound_at_position`'s. Server-side.
    #[hook(
        side = "server",
        contract = "cancel",
        out = "cancel",
        neoforge = "PlayLevelSoundEvent.AtEntity",
        cancellable = true
    )]
    fn play_level_sound_at_entity(
        entity: EntityRef,
        sound: &mut Identifier,
        source: &mut SoundSource,
        volume: &mut f32,
        pitch: &mut f32,
        original_volume: f32,
        original_pitch: f32,
        cancel: &mut bool,
    );
}
