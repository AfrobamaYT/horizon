//! A falling block's parts hear each tick it ends in the air (`falling_tick`), and not the
//! tick it lands.
use crate::game::{FallingBlock, native_entity_refs::tests::with_game};
use horizon_client::chunk::Chunk;
use horizon_core::{geometry::Vec3, position::BlockPos};
use mod_platform::__private::part_tests::{falling_tick_game_thread, falling_ticks_heard};
use std::sync::Arc;

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_falling_part_hears_each_airborne_tick_and_not_the_landing() {
    falling_tick_game_thread();
    with_game(|game| {
        game.world
            .chunks
            .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
        let registry = game.pack.assets.registry;
        let stone = registry.default_state("minecraft:stone").unwrap();
        game.world.set(0, 60, 0, stone).unwrap();
        let sand = registry.default_state("minecraft:sand").unwrap();
        let at = Vec3::new(0.5, 70.0, 0.5);
        game.falling.push(FallingBlock {
            on_ground: false,
            attachment_lifetime: Default::default(),
            rotation: [0.0; 2],
            join: Default::default(),
            id: 71,
            state: sand,
            start_pos: BlockPos::new(0, 70, 0),
            position: at,
            previous: at,
            velocity: Vec3::ZERO,
            age: 0,
            no_gravity: false,
            portal: Default::default(),
            drop_item: false,
            fall_distance: 0.0,
            stuck: Vec3::ZERO,
            damage: Default::default(),
            saved: None,
        });
        let mut ticks = 0;
        while !game.falling.is_empty() {
            assert!(ticks < 200, "the sand never landed");
            game.tick_one_falling(0, &[]).unwrap();
            game.deliver_falling_ticks();
            ticks += 1;
        }
        // Every tick but the landing one.
        assert!(ticks > 1);
        assert_eq!(falling_ticks_heard(), ticks - 1);
    });
}
