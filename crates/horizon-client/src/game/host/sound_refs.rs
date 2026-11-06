//! The game's side of the `sound_refs` area (`horizon-mod-api/src/host/sound_refs.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl SoundRefsBackendMut for Game<'_> {
    fn player_stop_sound(
        &mut self,
        player: PlayerRef,
        sound: Option<&str>,
        source: Option<SoundSource>,
    ) -> bool {
        mod_api::__private::check_access(
            "player_stop_sound",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_stop_sound(player, sound, source)
    }
    fn player_play_sound(
        &mut self,
        player: PlayerRef,
        sound: &str,
        source: SoundSource,
        volume: f32,
        pitch: f32,
    ) -> bool {
        mod_api::__private::check_access(
            "player_play_sound",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_play_sound(player, sound, source, volume, pitch)
    }
    fn entity_play_bound_sound(
        &mut self,
        entity: EntityRef,
        sound: &str,
        source: SoundSource,
        volume: f32,
        pitch: f32,
    ) -> bool {
        mod_api::__private::check_access(
            "entity_play_bound_sound",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_play_bound_sound(entity, sound, source, volume, pitch)
    }
}
