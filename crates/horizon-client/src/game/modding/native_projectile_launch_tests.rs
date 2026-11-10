//! Owner launch boundary regression with vanilla's deterministic sequence.
use super::super::native_entity_refs::tests::with_game;
use horizon_client::{items::ItemStack, mobs::Mob};
use horizon_core::{geometry::Vec3, random::RandomSource};
use mod_api::Identifier;

#[test]
#[ignore = "needs prepared resources and HORIZON_TEST_SEEDS=1"]
fn owner_launch_sets_owner_before_insertion_and_uses_projectile_random() {
    with_game(|game| {
        let kind = game
            .res
            .mob_types()
            .get("minecraft:zombie")
            .unwrap()
            .clone();
        let mut mob = Mob::new(21, kind, Vec3::new(0.0, 64.0, 0.0), 0.0);
        mob.previous = Vec3::new(-0.2, 63.7, 0.4);
        mob.on_ground = false;
        game.mobs.push(mob);
        let source = game.entity(21).unwrap();
        let owner = game.living(source).unwrap();
        game.entity_uuid(source).unwrap();
        let mut level_before = game.world.random.clone();
        let stack = ItemStack::new("minecraft:snowball", 1);
        let entity_type = Identifier::parse("minecraft:snowball").unwrap();
        // This thread's next unique seed is 1234 (odd multiplier inverse),
        // independent of world RNG and any draws during resource loading.
        horizon_client::random_source::set_thread_uniquifier(2709830126447559882).unwrap();
        let shot = game
            .spawn_projectile_from_rotation(owner, &entity_type, &stack, 0.0, 1.5, 1.0)
            .unwrap();
        assert_eq!(game.projectile_owner(shot), Some(source));
        let projectile = game
            .creatures
            .projectiles
            .iter()
            .find(|p| p.id == shot.entity().id())
            .unwrap();
        assert!((projectile.velocity.x - 0.19212421880232217).abs() < 1e-12);
        assert!((projectile.velocity.y - 0.3103198363925326).abs() < 1e-12);
        assert!((projectile.velocity.z - 1.1034130508130342).abs() < 1e-12);
        let mut random = projectile.random.0.clone();
        assert_eq!(random.next_double(), 0.34690742873967684);
        assert_eq!(game.world.random.next_long(), level_before.next_long());
        let count = game.creatures.projectiles.len();
        assert!(
            game.spawn_projectile_from_rotation(owner, &entity_type, &stack, 0.0, f32::NAN, 1.0)
                .is_none()
        );
        let unknown = Identifier::parse("test:unknown").unwrap();
        assert!(
            game.spawn_projectile_from_rotation(owner, &unknown, &stack, 0.0, 1.5, 1.0)
                .is_none()
        );
        assert_eq!(game.creatures.projectiles.len(), count);
    });
}
