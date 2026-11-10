//! A block's parts decide what a tool makes of it (`tool_modified_state`) and whether a player
//! breaking it destroyed it (`on_destroyed_by_player`).
use crate::game::{Game, Hit, native_entity_refs::tests::with_game, native_guest_turn_tests};
use horizon_client::{chunk::Chunk, items::ItemStack, motion::PlayerMotion};
use horizon_core::{geometry::Vec3, position::ChunkPos};
use mod_api::{BlockHit, Hand};
use std::sync::Arc;

fn hit_on(pos: [i32; 3]) -> Hit {
    Hit {
        pos,
        face: [0, 1, 0],
        point: [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 1.0, 0.5],
        inside: false,
    }
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_block_part_allows_replaces_or_denies_a_tool_action_and_a_preview_changes_nothing() {
    use crate::game::api_values::TryIntoApi;

    // The fixture's parts: an axe on an oak log makes a stripped spruce log, a birch log and a
    // campfire are left alone, and glass strips to a stripped oak log.
    mod_api::__private::part_tests::tool_modified_state_game_thread();
    native_guest_turn_tests::with_guests(|game| {
        game.world
            .chunks
            .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
        let registry = game.pack.assets.registry;
        let state = |name: &str| registry.default_state(name).unwrap();
        let axe = ItemStack::new("minecraft:iron_axe", 1);
        let value = Some(axe.clone());
        game.lan
            .as_mut()
            .unwrap()
            .hub
            .guests
            .update(0, |g| g.inventory[0] = value);
        let player = game.player(12).unwrap();
        let level = game.level(game.dim.id()).unwrap();
        // (block, what the axe makes of it, None for nothing, and whether a part answers)
        let cases = [
            // Replaced: vanilla strips an oak log to a stripped oak log; the part says spruce.
            (
                "minecraft:oak_log",
                Some("minecraft:stripped_spruce_log"),
                true,
            ),
            // Denied: vanilla strips a birch log; the part refuses.
            ("minecraft:birch_log", None, true),
            // Allowed: vanilla does nothing to glass; the part lets the axe strip it.
            ("minecraft:glass", Some("minecraft:stripped_oak_log"), true),
            // No part: vanilla's own table, both ways.
            (
                "minecraft:acacia_log",
                Some("minecraft:stripped_acacia_log"),
                false,
            ),
            ("minecraft:stone", None, false),
        ];
        for (index, (block, made, part)) in cases.into_iter().enumerate() {
            let pos = [2 * index as i32, 64, 0];
            game.world
                .set(pos[0], pos[1], pos[2], state(block))
                .unwrap();
            let at = |game: &Game<'_>| game.world.block(pos[0], pos[1], pos[2]);
            // The preview answers as the action will, and changes nothing.
            let native = BlockHit::new(hit_on(pos).point, [0, 1, 0], false);
            let preview = level.block_transform(
                game,
                player,
                Hand::Main,
                &axe.try_into_api().unwrap(),
                pos,
                native,
            );
            assert_eq!(
                preview.map(|p| p.state),
                made.map(|name| state(name).try_into_api().unwrap()),
                "{block}: preview"
            );
            assert_eq!(at(game), state(block), "{block}: the preview wrote");
            assert_eq!(
                mod_api::__private::part_tests::take_tool_simulations(),
                if part { vec![true] } else { vec![] },
                "{block}: the part hears the simulation"
            );
            // The action does it.
            game.as_guest(12, |game| {
                assert_eq!(
                    game.use_block_transformer(&axe, &hit_on(pos)).unwrap(),
                    made.is_some(),
                    "{block}: acted"
                );
            })
            .unwrap();
            assert_eq!(at(game), state(made.unwrap_or(block)), "{block}: result");
            assert_eq!(
                mod_api::__private::part_tests::take_tool_simulations(),
                if part { vec![false] } else { vec![] },
                "{block}: the part hears the real action"
            );
        }
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_block_part_can_deny_a_shovel_putting_out_a_campfire() {
    // The fixture's parts: a campfire is left alone by every tool.
    mod_api::__private::part_tests::tool_modified_state_game_thread();
    native_guest_turn_tests::with_guests(|game| {
        game.world
            .chunks
            .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
        let registry = game.pack.assets.registry;
        let state = |name: &str| registry.default_state(name).unwrap();
        let shovel = ItemStack::new("minecraft:iron_shovel", 1);
        let is_lit = |game: &Game<'_>, pos: [i32; 3], block: &str| {
            let id = game.world.block(pos[0], pos[1], pos[2]);
            let state = game.pack.assets.registry.state(id).unwrap();
            assert_eq!(&*state.name, block);
            state.properties.get("lit").map(String::as_str) == Some("true")
        };
        // Denied: the part's campfire stays lit. No part: the soul campfire goes out.
        for (block, doused) in [
            ("minecraft:campfire", false),
            ("minecraft:soul_campfire", true),
        ] {
            let pos = [if doused { 2 } else { 0 }, 64, 0];
            game.world
                .set(pos[0], pos[1], pos[2], state(block))
                .unwrap();
            assert!(is_lit(game, pos, block), "{block} starts lit");
            assert_eq!(
                game.use_campfire_douse(&hit_on(pos), Some(&shovel))
                    .unwrap(),
                doused,
                "{block}"
            );
            assert_eq!(is_lit(game, pos, block), !doused, "{block}: lit after");
        }
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_block_part_decides_whether_a_player_breaking_a_block_destroyed_it() {
    // The fixture's parts: snow loses one layer per break and stays, down to the last, which
    // vanilla removes; glass is removed by its part, which answers that it was destroyed.
    mod_api::__private::part_tests::destroyed_by_player_game_thread();
    with_game(|game| {
        // The parts' writes need a server.
        game.options.dedicated = Some(Vec::new());
        let scratch = std::env::temp_dir().join(format!(
            "horizon-destroyed-part-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&scratch).unwrap();
        game.world_root = scratch.clone();
        game.player = Some(PlayerMotion::new(game.motion, Vec3::new(4.5, 62.0, 4.5)).unwrap());
        game.player.as_mut().unwrap().ground.on_ground = true;
        game.creative = false;
        game.build_registries();
        game.set_allow_commands(true);
        assert!(game.run_command("gamemode survival"));
        game.world
            .chunks
            .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
        game.block_ticks
            .add_empty_chunk(ChunkPos::new(0, 0))
            .unwrap();
        let registry = game.pack.assets.registry;
        let pos = [5, 63, 4];
        let snow = |layers: u32| {
            registry
                .resolve(&horizon_world::chunk::BlockState {
                    name: "minecraft:snow".into(),
                    properties: [("layers".to_string(), layers.to_string())].into(),
                })
                .unwrap()
        };
        let air = registry.default_state("minecraft:air").unwrap();
        let count = |game: &Game<'_>, item: &str| {
            game.dropped
                .iter()
                .filter(|dropped| &*dropped.stack.item == item)
                .map(|dropped| dropped.stack.count)
                .sum::<u32>()
        };

        // Three layers, broken by a bare hand: each break takes a layer, drops its snowball and
        // leaves the block; the last layer is vanilla's removal.
        game.world.set(pos[0], pos[1], pos[2], snow(3)).unwrap();
        game.dropped.clear();
        for (layers, snowballs) in [(2, 1), (1, 2)] {
            game.break_block(pos).unwrap();
            assert_eq!(
                game.world.block(pos[0], pos[1], pos[2]),
                snow(layers),
                "{layers} layers are left"
            );
            assert_eq!(count(game, "minecraft:snowball"), snowballs);
        }
        game.break_block(pos).unwrap();
        assert_eq!(game.world.block(pos[0], pos[1], pos[2]), air);
        // The bare hand never harvests snow, and the stack it holds is empty: the part's own
        // snowballs are the only ones.
        assert_eq!(count(game, "minecraft:snowball"), 2);
        assert_eq!(
            mod_api::__private::part_tests::take_destroys(),
            vec![(false, 0); 3]
        );

        // The part removes glass itself and answers that it did: a destroyed block.
        game.world
            .set(
                pos[0],
                pos[1],
                pos[2],
                registry.default_state("minecraft:glass").unwrap(),
            )
            .unwrap();
        game.break_block(pos).unwrap();
        assert_eq!(game.world.block(pos[0], pos[1], pos[2]), air);
        // Glass needs no tool, so a bare hand harvests it.
        assert_eq!(
            mod_api::__private::part_tests::take_destroys(),
            vec![(true, 0)]
        );

        // No part: vanilla's removal and drops.
        game.world
            .set(
                pos[0],
                pos[1],
                pos[2],
                registry.default_state("minecraft:dirt").unwrap(),
            )
            .unwrap();
        game.dropped.clear();
        game.break_block(pos).unwrap();
        assert_eq!(game.world.block(pos[0], pos[1], pos[2]), air);
        assert_eq!(count(game, "minecraft:dirt"), 1);
        assert_eq!(mod_api::__private::part_tests::take_destroys(), vec![]);
        std::fs::remove_dir_all(scratch).unwrap();
    });
}
