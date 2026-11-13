//! A mod's world pickup and placement of fluid in a real game: decided at the call, its block
//! kept by the transaction, its updates, drops and effects at the outermost commit, nothing of
//! it after an abort.
use crate::game::{Game, dispense_fixture, native_entity_refs::tests::with_game};
use horizon_client::{chunk::Chunk, containers::BlockEntity};
use horizon_world::registry::BlockStateId;
use mod_api::{FluidStack, HostTransaction, LevelRef, ServerHost};
use std::sync::Arc;

const AT: [i32; 3] = [2, 65, 2];

/// A game with one empty chunk, a dirt floor at y 64 and the level's handle.
fn with_level(test: impl FnOnce(&mut Game<'_>, LevelRef)) {
    with_game(|game| {
        game.world
            .chunks
            .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
        game.native_lifecycle
            .begin_open(mod_api::Authority::IntegratedServer);
        let dirt = state(game, "minecraft:dirt");
        for x in 0..5 {
            for z in 0..5 {
                game.world.set(x, 64, z, dirt).unwrap();
            }
        }
        let level = game.level("minecraft:overworld").unwrap();
        test(game, level);
    });
}

fn state(game: &Game<'_>, name: &str) -> BlockStateId {
    game.pack.assets.registry.default_state(name).unwrap()
}

fn key(id: &str) -> mod_api::FluidKey {
    mod_api::__private::fluid_key(id).unwrap()
}

fn block(game: &Game<'_>) -> BlockStateId {
    game.world.block(AT[0], AT[1], AT[2])
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_second_pickup_of_one_source_in_a_transaction_gets_nothing() {
    with_level(|game, level| {
        let water = state(game, "minecraft:water");
        game.world.set(AT[0], AT[1], AT[2], water).unwrap();
        let host: &mut dyn ServerHost = game;
        let picked: Result<_, ()> = host.transaction(|host, tx| {
            Ok((
                level.pickup_fluid(host, AT, None, tx),
                level.pickup_fluid(host, AT, None, tx),
            ))
        });
        let one = FluidStack::new(key("minecraft:water"), 1000);
        assert_eq!(picked, Ok((Some(one), None)));
        assert_eq!(block(game), state(game, "minecraft:air"));
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn an_aborted_placement_over_a_skull_leaves_it_and_drops_nothing() {
    with_level(|game, level| {
        let skull = state(game, "minecraft:skeleton_skull");
        game.world.set(AT[0], AT[1], AT[2], skull).unwrap();
        let pos = (AT[0], AT[1], AT[2]);
        game.world
            .block_entities
            .insert(pos, BlockEntity::Skull(Default::default()));
        let kept = format!("{:?}", game.world.block_entities.get(&pos));
        assert!(game.world.block_entities.get(&pos).is_some(), "the skull has its block entity");
        let items = game.dropped.len();
        dispense_fixture::start_recording();
        let root = game.transaction_open();
        let water = key("minecraft:water");
        assert!(game.level_place_fluid(level, AT, water, None, true, root.transaction()));
        // Inside the transaction the skull is gone and the water is there.
        assert_eq!(block(game), state(game, "minecraft:water"));
        assert!(game.world.block_entities.get(&pos).is_none());
        game.transaction_close(&root, false);
        assert_eq!(block(game), skull);
        assert_eq!(format!("{:?}", game.world.block_entities.get(&pos)), kept);
        assert_eq!(game.dropped.len(), items);
        assert_eq!(dispense_fixture::stop_recording(), Vec::<String>::new());
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_committed_pour_breaks_what_it_replaces_then_sends_its_events_in_order() {
    with_level(|game, level| {
        let poppy = state(game, "minecraft:poppy");
        game.world.set(AT[0], AT[1], AT[2], poppy).unwrap();
        let items = game.dropped.len();
        dispense_fixture::start_recording();
        let host: &mut dyn ServerHost = game;
        let placed: Result<bool, ()> = host.transaction(|host, tx| {
            Ok(level.place_fluid(host, AT, key("minecraft:lava"), None, false, tx))
        });
        assert_eq!(placed, Ok(true));
        let lines = dispense_fixture::stop_recording();
        assert_eq!(block(game), state(game, "minecraft:lava"));
        // The poppy's drop, made at the call, spawned at the commit.
        assert_eq!(game.dropped.len(), items + 1);
        assert_eq!(&*game.dropped[items].stack.item, "minecraft:poppy");
        assert_eq!(
            lines,
            [
                format!("L 2001 2,65,2 {}", poppy.0),
                "G 2,65,2 minecraft:block_destroy".to_owned(),
                "G 2,65,2 minecraft:fluid_place".to_owned(),
            ]
        );
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn the_call_time_answer_stands_when_the_block_changes_before_the_commit() {
    with_level(|game, level| {
        let poppy = state(game, "minecraft:poppy");
        game.world.set(AT[0], AT[1], AT[2], poppy).unwrap();
        let stone = state(game, "minecraft:stone");
        let items = game.dropped.len();
        dispense_fixture::start_recording();
        let root = game.transaction_open();
        let water = key("minecraft:water");
        let placed = game.level_place_fluid(level, AT, water, None, false, root.transaction());
        // A later write in the same transaction: the commit doesn't decide again.
        game.journal_set_block(AT, stone);
        game.transaction_close(&root, true);
        assert!(placed);
        let lines = dispense_fixture::stop_recording();
        assert_eq!(block(game), stone);
        assert_eq!(game.dropped.len(), items + 1);
        assert_eq!(
            lines,
            [
                format!("L 2001 2,65,2 {}", poppy.0),
                "G 2,65,2 minecraft:block_destroy".to_owned(),
                "G 2,65,2 minecraft:fluid_place".to_owned(),
            ]
        );
    });
}
