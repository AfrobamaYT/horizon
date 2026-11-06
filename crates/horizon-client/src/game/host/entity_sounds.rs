//! The game's side of the `entity_sounds` area (`horizon-mod-api/src/host/entity_sounds.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl EntitySoundsBackend for Game<'_> {
    fn entity_sound_source(&self, entity: EntityRef) -> Option<SoundSource> {
        mod_api::__private::check_access(
            "entity_sound_source",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_sound_source(entity)
    }
    fn entity_spectator(&self, entity: EntityRef) -> Option<bool> {
        mod_api::__private::check_access(
            "entity_spectator",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_spectator(entity)
    }
}
impl EntitySoundsBackendMut for Game<'_> {
    fn entity_play_sound(
        &mut self,
        entity: EntityRef,
        sound: &str,
        volume: f32,
        pitch: f32,
    ) -> bool {
        mod_api::__private::check_access(
            "entity_play_sound",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_play_sound(entity, sound, volume, pitch)
    }
    fn entity_game_event(&mut self, entity: EntityRef, event: &str) -> bool {
        mod_api::__private::check_access(
            "entity_game_event",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_game_event(entity, event)
    }
    fn entity_broadcast_event(&mut self, entity: EntityRef, id: u8) -> bool {
        mod_api::__private::check_access(
            "entity_broadcast_event",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_broadcast_event(entity, id)
    }
}
