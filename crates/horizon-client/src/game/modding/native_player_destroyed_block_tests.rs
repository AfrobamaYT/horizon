//! `player_destroyed_block` heard through the game's own break path: a creative and a survival
//! break are heard once with the removed state; a break `break_block` cancels and a piston's
//! are not heard.
use super::native_entity_refs::tests::with_game;
use horizon_client::{command::args::GameMode, items::ItemStack, motion::PlayerMotion};
use horizon_core::{geometry::Vec3, position::BlockPos};

use mod_api::{BreakBlockEvent, Flow, Hooks, PlayerDestroyedBlockEvent, ServerHost};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

/// One heard break: the position, the state's block name and the block entity's saved data.
type Heard = ([i32; 3], String, Option<mod_api::Compound>);

struct Probe {
    heard: Arc<Mutex<Vec<Heard>>>,
    cancel_break: Arc<AtomicBool>,
}

#[mod_api::__subscriptions]
impl Hooks for Probe {
    fn break_block(&self, _: &mut dyn ServerHost, _: &mut BreakBlockEvent<'_>) -> Flow {
        if self.cancel_break.load(Ordering::Relaxed) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }
    fn player_destroyed_block(
        &self,
        host: &mut dyn ServerHost,
        event: &mut PlayerDestroyedBlockEvent,
    ) {
        // The state is the block's default state, which the test placed.
        let name = ["minecraft:stone", "minecraft:chest"]
            .into_iter()
            .find(|name| host.block_state_id(name) == Some(event.state))
            .expect("the broken state is a default state the test placed")
            .to_owned();
        // The level is already past the removal: the block is gone from it.
        let level = host.level("minecraft:overworld").expect("the overworld");
        assert_eq!(
            level.block_state(host, event.pos),
            host.block_state_id("minecraft:air"),
        );
        assert!(
            host.players().acting_player().is_some(),
            "the breaker is the acting player"
        );
        self.heard
            .lock()
            .unwrap()
            .push((event.pos, name, event.block_entity.clone()));
    }
}

fn place(game: &mut super::Game<'_>, pos: [i32; 3], block: &str) {
    let state = game.pack.assets.registry.default_state(block).unwrap();
    game.world.set(pos[0], pos[1], pos[2], state).unwrap();
}

fn block(game: &super::Game<'_>, pos: [i32; 3]) -> String {
    let state = game.world.block(pos[0], pos[1], pos[2]);
    game.res.facts().get(state).name.to_string()
}

/// A server's game with its own player standing in the origin chunk, as the native player-break
/// tests set one up: their UUID.
fn with_player(game: &mut super::Game<'_>, mode: GameMode, held: Option<ItemStack>) -> u128 {
    game.options.dedicated = Some(Vec::new());
    let _ = game
        .block_ticks
        .add_empty_chunk(horizon_core::position::ChunkPos::new(0, 0));
    game.world
        .chunks
        .insert((0, 0), Arc::new(horizon_client::chunk::Chunk::empty(0, 0)));
    game.player = Some(PlayerMotion::new(game.motion, Vec3::new(4.5, 64.0, 4.5)).unwrap());
    game.lan_acting_uuid = Some(123);
    game.lan_acting_entity = Some(20);
    game.lan_acting_name = Some("Breaker".into());
    game.lan_acting_permission = Some(0);
    game.set_mode(mode);
    game.inventory[game.selected] = held;
    game.native_acting_id()
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_players_completed_break_is_heard_once_with_the_removed_state() {
    let heard = Arc::new(Mutex::new(Vec::new()));
    let cancel_break = Arc::new(AtomicBool::new(false));
    let mods: mod_api::Mods = vec![mod_api::ModEntry::new(
        "destroyed",
        Box::new(Probe {
            heard: heard.clone(),
            cancel_break: cancel_break.clone(),
        }),
    )]
    .into();
    mod_api::testing::with_mods(mods, || {
        with_game(|game| {
            let take = || std::mem::take(&mut *heard.lock().unwrap());

            // Survival, with the right tool: stone drops, and the break is heard.
            let uuid = with_player(
                game,
                GameMode::Survival,
                Some(ItemStack::new("minecraft:diamond_pickaxe", 1)),
            );
            let stone = [6, 64, 4];
            place(game, stone, "minecraft:stone");
            assert!(game.native_player_break_block(uuid, stone, false));
            assert_eq!(block(game, stone), "minecraft:air");
            assert_eq!(take(), [(stone, "minecraft:stone".to_owned(), None)]);

            // Creative: no drops and an early return in vanilla, still heard once. A chest
            // carries its saved contents.
            let uuid = with_player(game, GameMode::Creative, None);
            let chest = [7, 64, 4];
            place(game, chest, "minecraft:chest");
            let mut contents = vec![None; 27];
            contents[0] = Some(ItemStack::new("minecraft:diamond", 3));
            game.world.block_entities.insert(
                (chest[0], chest[1], chest[2]),
                horizon_client::containers::BlockEntity::Chest(contents),
            );
            assert!(game.native_player_break_block(uuid, chest, false));
            assert_eq!(block(game, chest), "minecraft:air");
            let heard_now = take();
            assert_eq!(heard_now.len(), 1);
            assert_eq!(
                (heard_now[0].0, heard_now[0].1.as_str()),
                (chest, "minecraft:chest")
            );
            let data = heard_now[0]
                .2
                .as_ref()
                .expect("the removed chest's saved data");
            let Some(mod_api::Tag::List(items)) = data.get(&"Items".into()) else {
                panic!("a chest saves its Items");
            };
            assert_eq!(items.len(), 1, "the one stack the chest held");
            let mod_api::Tag::Compound(stack) = &items[0] else {
                panic!("a saved stack is a compound");
            };
            assert_eq!(stack.get(&"count".into()), Some(&mod_api::Tag::Int(3)));
            assert_eq!(
                stack.get(&"id".into()),
                Some(&mod_api::Tag::String("minecraft:diamond".into()))
            );

            // A break `break_block` cancels keeps the block and is not heard.
            let uuid = with_player(
                game,
                GameMode::Survival,
                Some(ItemStack::new("minecraft:diamond_pickaxe", 1)),
            );
            cancel_break.store(true, Ordering::Relaxed);
            let kept = [8, 64, 4];
            place(game, kept, "minecraft:stone");
            assert!(!game.native_player_break_block(uuid, kept, false));
            assert_eq!(block(game, kept), "minecraft:stone");
            assert!(take().is_empty());
            cancel_break.store(false, Ordering::Relaxed);

            // A piston pushing a torch breaks it from the level's side: not heard.
            let registry = game.pack.assets.registry;
            let piston = registry.default_state("minecraft:piston").unwrap();
            let piston = game.with_props(piston, &[("facing", "east")]).unwrap();
            let torch = registry.default_state("minecraft:torch").unwrap();
            let power = registry.default_state("minecraft:redstone_block").unwrap();
            game.world.set(10, 70, 4, piston).unwrap();
            game.world.set(11, 70, 4, torch).unwrap();
            game.with_level(|b, level, ticks| {
                b.set_block(level, ticks, BlockPos::new(9, 70, 4), power, 3);
                b.run_block_events(level, ticks);
            });
            assert_ne!(
                block(game, [11, 70, 4]),
                "minecraft:torch",
                "the torch was pushed away"
            );
            assert!(take().is_empty(), "a level-side break is not a player's");
        });
    });
}
