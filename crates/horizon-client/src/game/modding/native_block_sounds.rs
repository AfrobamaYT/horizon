// game-module-visibility: pub(crate)
//! The `sound_type` Part method (`IBlockExtension.getSoundType`). The client library asks through
//! `native_client_hooks`; only a state whose block has a subscribing part is converted, and only
//! then is its own sound type kept to fall back on.
use super::api_values::{TryIntoApi, block_state};
use super::{Game, GameLevel};
use horizon_core::{identifier::Identifier, position::BlockPos};
use horizon_world::{
    level::data::Sounds, native_load_context::LoopContext, registry::BlockStateId,
};
use mod_api::{BlockSoundCause, BlockSoundType};
use std::sync::Arc;

/// The game's side of `horizon_client::block_sounds::native_sound_type`.
pub(crate) fn sound_type(
    context: &LoopContext,
    pos: BlockPos,
    state: BlockStateId,
    entity: Option<(u32, &str)>,
    cause: BlockSoundCause,
    sound: &mut BlockSoundType,
) {
    if !block_state::block_listens(mod_api::HookId::SoundType, state) {
        return;
    }
    let Ok(api_state) = state.try_into_api() else {
        return;
    };
    let own = sound.clone();
    mod_api::dispatch_sound_type(
        &horizon_client_render::api_context::LoopReader(context).context(),
        pos,
        api_state,
        entity,
        cause,
        sound,
    );
    if !playable(sound) {
        *sound = own;
    }
}

/// Whether a part's answer can be played: a finite volume and pitch of at least 0, and every
/// event a `namespace:path` id (a mod's own included; one the resources lack plays nothing).
fn playable(sound: &BlockSoundType) -> bool {
    let level = |value: f32| value.is_finite() && value >= 0.0;
    let events = [
        &sound.break_sound,
        &sound.step,
        &sound.place,
        &sound.hit,
        &sound.fall,
    ];
    level(sound.volume)
        && level(sound.pitch)
        && events
            .iter()
            .all(|event| Identifier::parse_namespaced(event).is_some())
}

impl Game<'_> {
    /// The sound type of `state` at `pos` for `cause` (`IBlockExtension.getSoundType`), as the
    /// block's parts answer on this side; `entity` is the id and type of the entity stepping,
    /// landing or placing.
    pub(in crate::game) fn block_sound_type(
        &self,
        [x, y, z]: [i32; 3],
        state: BlockStateId,
        entity: Option<(u32, &str)>,
        cause: BlockSoundCause,
    ) -> BlockSoundType {
        horizon_client::block_sounds::sound_type(
            &self.loop_context,
            self.res.facts(),
            BlockPos::new(x, y, z),
            state,
            entity,
            cause,
        )
    }
    /// [`Game::block_sound_type`] for this window's own player, as its client asks for the
    /// sounds it plays itself: its steps and the blocks it places.
    pub(in crate::game) fn own_sound_type(
        &self,
        pos: [i32; 3],
        state: BlockStateId,
        cause: BlockSoundCause,
    ) -> BlockSoundType {
        let player = (self.own_entity_id() as u32, "minecraft:player");
        self.block_sound_type(pos, state, Some(player), cause)
    }
    /// `LevelEventHandler`'s break sound (level event 2001, which a client's predicted break
    /// also plays): the block's break sound at its centre, at `(volume + 1) / 2` and pitch × 0.8.
    /// None for air.
    pub(in crate::game) fn play_break_sound(&mut self, pos: [i32; 3], state: BlockStateId) {
        if self.res.facts().is_air(state) {
            return;
        }
        let sound = self.block_sound_type(pos, state, None, BlockSoundCause::Break);
        self.audio.play(
            &sound.break_sound,
            Some(pos.map(|v| f64::from(v) + 0.5)),
            (sound.volume + 1.0) / 2.0,
            sound.pitch * 0.8,
        );
    }
}

impl GameLevel<'_> {
    /// `LevelAccess::place_sound_type`: the state's own `sounds` unless a part on its block
    /// answers another, asked with the acting player when `by_player`.
    pub(super) fn part_place_sound_type(
        &self,
        pos: BlockPos,
        state: BlockStateId,
        by_player: bool,
        sounds: Arc<Sounds>,
    ) -> Arc<Sounds> {
        if !block_state::block_listens(mod_api::HookId::SoundType, state) {
            return sounds;
        }
        let player = self.native_player_entity.filter(|_| by_player);
        let entity = player.map(|id| (id as u32, "minecraft:player"));
        let own = horizon_client::block_sounds::own(&self.facts.get(state).sounds);
        let mut answer = own.clone();
        horizon_client::block_sounds::native_sound_type(
            self.loop_context,
            pos,
            state,
            entity,
            BlockSoundCause::Place,
            &mut answer,
        );
        if answer == own {
            return sounds;
        }
        Arc::new(Sounds {
            volume: answer.volume,
            pitch: answer.pitch,
            place: answer.place,
            break_: answer.break_sound,
            fall: answer.fall,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::native_entity_refs::tests::with_game;
    use horizon_core::position::BlockPos;
    use horizon_world::level::LevelAccess;
    use mod_api::BlockSoundCause::{Break, Fall, Hit, Place, Step};
    use std::sync::Arc;

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_block_part_answers_the_sound_type_a_block_plays() {
        // The fixture's parts: glass answers `mymod:block.x.*` at volume 2.0 and pitch 1.5 to
        // every cause, white wool the same to a player only, and red and blue wool a NaN volume
        // and an event with no namespace.
        mod_api::__private::part_tests::sound_type_game_thread();
        // The client library asks the mods through the executable's callbacks.
        crate::entry::install_client_hooks();
        with_game(|game| {
            let registry = game.pack.assets.registry;
            let player = Some((7, "minecraft:player"));
            let creeper = Some((8, "minecraft:creeper"));
            let sound = |game: &crate::game::Game<'_>, block: &str, entity, cause| {
                let state = registry.default_state(block).unwrap();
                let sound = game.block_sound_type([5, 64, 5], state, entity, cause);
                let event = match cause {
                    Step => sound.step,
                    Fall => sound.fall,
                    Place => sound.place,
                    Break => sound.break_sound,
                    _ => sound.hit,
                };
                (event.to_string(), sound.volume, sound.pitch)
            };
            let stone = |event: &str| (format!("minecraft:block.stone.{event}"), 1.0, 1.0);
            let wool = |event: &str| (format!("minecraft:block.wool.{event}"), 1.0, 1.0);
            let answered = |event: &str| (format!("mymod:block.x.{event}"), 2.0, 1.5);
            // No part: the state's own.
            assert_eq!(sound(game, "minecraft:stone", creeper, Step), stone("step"));
            // A part answering every cause and entity, with the mod's own events.
            assert_eq!(
                sound(game, "minecraft:glass", creeper, Step),
                answered("step")
            );
            assert_eq!(
                sound(game, "minecraft:glass", creeper, Fall),
                answered("fall")
            );
            assert_eq!(
                sound(game, "minecraft:glass", player, Place),
                answered("place")
            );
            assert_eq!(
                sound(game, "minecraft:glass", None, Break),
                answered("break")
            );
            assert_eq!(sound(game, "minecraft:glass", None, Hit), answered("hit"));
            // A part answering for a player only.
            assert_eq!(
                sound(game, "minecraft:white_wool", player, Step),
                answered("step")
            );
            assert_eq!(
                sound(game, "minecraft:white_wool", creeper, Step),
                wool("step")
            );
            assert_eq!(
                sound(game, "minecraft:white_wool", None, Break),
                wool("break")
            );
            // Answers that can't be played are dropped whole for the state's own.
            assert_eq!(
                sound(game, "minecraft:red_wool", creeper, Step),
                wool("step")
            );
            assert_eq!(
                sound(game, "minecraft:blue_wool", creeper, Step),
                wool("step")
            );
            // The server's placement asks with the acting player (`BlockItem.place`), and
            // sculk's growth with none.
            let glass = registry.default_state("minecraft:glass").unwrap();
            let white_wool = registry.default_state("minecraft:white_wool").unwrap();
            game.lan_acting_entity = Some(7);
            let placed = game.with_level(|behaviours, level, _| {
                // None when the level's own sound type is kept.
                let place = |state, by_player| {
                    let own = Arc::clone(&behaviours.data(state).sounds);
                    let pos = BlockPos::new(5, 64, 5);
                    let sounds = level.place_sound_type(pos, state, by_player, Arc::clone(&own));
                    (!Arc::ptr_eq(&sounds, &own))
                        .then(|| (sounds.place.to_string(), sounds.volume, sounds.pitch))
                };
                [
                    place(glass, false),
                    place(white_wool, true),
                    place(white_wool, false),
                ]
            });
            let answered = Some(answered("place"));
            assert_eq!(placed, [answered.clone(), answered, None]);
        });
    }
}
