//! The `entity_sounds` area: host methods the game implements (`game/host/entity_sounds.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `entity_sounds` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait EntitySoundsBackend {
    /// Side: server.
    /// `Entity.getSoundSource`: the group an entity's sounds play under.
    /// Players are `Players`; `Monster`s, the dragon, hoglins, ghasts,
    /// phantoms, shulkers, slimes and a killer bunny are `Hostile`; dropped
    /// items and experience orbs `Ambient`; every other entity `Neutral`.
    fn entity_sound_source(&self, entity: EntityRef) -> Option<SoundSource> {
        crate::phase::unprovided(
            "Backend::entity_sound_source",
            "entity_sound_source",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Entity.isSpectator`: true for a player in spectator mode, false for
    /// every other entity.
    fn entity_spectator(&self, entity: EntityRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::entity_spectator",
            "entity_spectator",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `entity_sounds` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait EntitySoundsBackendMut {
    /// Side: server.
    /// `Entity.playSound(sound, volume, pitch)`: plays sound event `sound` at
    /// the entity under its sound source, for the players within range. A
    /// silent mob plays nothing, as vanilla's check is `isSilent`; a player
    /// (`Player.playSound`) doesn't hear it themself, their own client plays
    /// it. True means queued, not heard; false for an unknown `minecraft:`
    /// event or a volume or pitch that isn't a finite non-negative number.
    fn entity_play_sound(
        &mut self,
        entity: EntityRef,
        sound: &str,
        volume: f32,
        pitch: f32,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_play_sound",
            "entity_play_sound",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Entity.gameEvent(event)`: game event `event` (`minecraft:step`...)
    /// at the entity's position with the entity as its source, which sculk
    /// sensors, shriekers, wardens and allays in range hear. A player or mob
    /// is the event's source entity; other entities retain their own id.
    /// False for an event vanilla doesn't have, a dead guest and a guest while
    /// another one acts.
    fn entity_game_event(&mut self, entity: EntityRef, event: &str) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_game_event",
            "entity_game_event",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Level.broadcastEntityEvent(entity, id)`: sends entity event `id` (the
    /// byte `Entity.handleEntityEvent` reads: `2` is a hurt flash, `3` a
    /// death, `35` a totem...) to the players tracking the entity and to the
    /// entity itself. True means queued; false for an entity outside the
    /// level in place.
    fn entity_broadcast_event(&mut self, entity: EntityRef, id: u8) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_broadcast_event",
            "entity_broadcast_event",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
