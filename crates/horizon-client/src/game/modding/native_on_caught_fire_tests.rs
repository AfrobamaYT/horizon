//! `on_caught_fire` heard by a block Part (#3185): the listener gets the burned block's old state
//! and the face toward the fire, and what it writes through its level is in the world.
use super::native_entity_refs::tests::with_game;
use horizon_core::{geometry::Direction, position::BlockPos};
use horizon_world::level::LevelAccess;
use mod_api::{
    self, BlockPart, BlockUpdateFlags, Hooks, LevelWriteHost, OnCaughtFireEvent, Registrar,
};

/// Wool that, burned from its west face, leaves its old state on the cell above.
struct Ember;

#[mod_api::horizon_part]
impl BlockPart for Ember {
    fn on_caught_fire(&self, level: &mut dyn LevelWriteHost, event: &mut OnCaughtFireEvent) {
        if event.face == Direction::West {
            let above = [event.pos.x, event.pos.y + 1, event.pos.z];
            level.set_block(above, event.state, BlockUpdateFlags::NONE);
        }
    }
}

struct EmberMod;

#[mod_api::__subscriptions]
impl Hooks for EmberMod {
    fn register(&self, r: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
        let wool = r.block_key("minecraft:white_wool");
        r.block_part(wool, Ember);
    }
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_block_part_hears_it_burned_and_writes_the_level() {
    use horizon_client::chunk::Chunk;
    use std::sync::Arc;

    let mods: mod_api::Mods = vec![mod_api::ModEntry::new("ember", Box::new(EmberMod))].into();
    crate::game::api_values::install_parts(&mods).unwrap();
    mod_api::testing::with_mods(mods, || {
        with_game(|game| {
            game.options.dedicated = Some(Vec::new());
            game.world
                .chunks
                .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
            game.block_ticks
                .add_empty_chunk(horizon_core::position::ChunkPos::new(0, 0))
                .unwrap();
            let registry = game.pack.assets.registry;
            let wool = registry.default_state("minecraft:white_wool").unwrap();
            let air = registry.default_state("minecraft:air").unwrap();
            // The wool at (1, 64, 0) has burned to air, heard from `face`.
            let burn = |game: &mut super::Game<'_>, face: Direction| {
                game.with_level(|_, level, ticks| {
                    level.on_caught_fire(ticks, wool, BlockPos::new(1, 64, 0), face);
                });
                game.world.block(1, 65, 0)
            };
            assert_eq!(burn(game, Direction::North), air, "the Part writes nothing");
            assert_eq!(
                burn(game, Direction::West),
                wool,
                "its write is in the level"
            );
        })
    });
}
