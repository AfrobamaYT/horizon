//! The `sound_refs` area: host methods the game implements (`game/host/sound_refs.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' server writes of the `sound_refs` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait SoundRefsBackendMut {
    /// Side: server.
    /// `StopSoundCommand` for one player: their client stops the playing
    /// sounds that match `sound` (an event id) and `source`; `None` matches
    /// any. True means sent.
    fn player_stop_sound(
        &mut self,
        player: PlayerRef,
        sound: Option<&str>,
        source: Option<SoundSource>,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_stop_sound",
            "player_stop_sound",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `PlaySoundCommand` for one player: plays sound event `sound` at the
    /// player, for them only, under `source`'s volume slider. The seed is the
    /// level random's `nextLong`, drawn once per call. True means sent, not
    /// heard.
    fn player_play_sound(
        &mut self,
        player: PlayerRef,
        sound: &str,
        source: SoundSource,
        volume: f32,
        pitch: f32,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_play_sound",
            "player_play_sound",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Level.playSound(null, entity, sound, source, volume, pitch)`: plays
    /// sound event `sound` bound to the entity, so that it follows it, for
    /// the players within range (16 blocks, times the volume above 1) of
    /// where it is now, the entity's own player too. True means queued, not
    /// heard; false for an entity outside the level in place.
    fn entity_play_bound_sound(
        &mut self,
        entity: EntityRef,
        sound: &str,
        source: SoundSource,
        volume: f32,
        pitch: f32,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_play_bound_sound",
            "entity_play_bound_sound",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
