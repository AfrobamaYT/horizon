//! Production singleton installation/removal and id-allocation contracts.
use super::*;
use crate::game::native_entity_refs::tests::{with_client_game, with_game};

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn singleton_dragon_spawn_install_refusal_and_kill() {
    with_game(|game| {
        game.dragon.discard();
        let before = game.next_entity;
        let id = game
            .spawn_dragon_spawned([0.0, 80.0, 0.0], SpawnSite::Plain)
            .unwrap();
        assert_eq!(id, before + 1);
        assert_eq!(game.next_entity, before + 9);
        for part in id + 1..=id + 8 {
            assert!(game.dragon_part(part).is_some());
        }
        assert!(
            game.spawn_dragon_spawned([1.0, 80.0, 0.0], SpawnSite::Plain)
                .is_none()
        );
        assert_eq!(game.next_entity, before + 9);
        let mut replacement = Dragon::new(Vec3::ZERO, 0.0, 0);
        replacement.id = 900;
        let refused = game.dragon.install(replacement).err().unwrap();
        assert_eq!(refused.id, 900);
        assert_eq!(game.dragon.as_ref().unwrap().id, id);
        game.kill_dragon(id + 1);
        assert_eq!(game.dragon.as_ref().unwrap().id, id);
        game.kill_dragon(id);
        assert!(game.dragon.is_none());
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn singleton_dragon_load_keeps_state_and_reserves_nine_ids() {
    with_game(|game| {
        game.dragon.discard();
        let mut saved = Dragon::new(Vec3::new(2.0, 90.0, 3.0), 25.0, 0);
        saved.health = 73.0;
        let uuid = [17, 0, 0, 3];
        let tag = saved.to_nbt(uuid, None);
        let before = game.next_entity;
        game.load_dragon(tag.clone(), Some(uuid));
        let dragon = game.dragon.as_ref().unwrap();
        assert_eq!(dragon.id, before + 1);
        assert_eq!(dragon.position, saved.position);
        assert_eq!(dragon.health, 73.0);
        assert_eq!(game.next_entity, before + 9);
        assert_eq!(game.persist.uuids.get(&dragon.id), Some(&uuid));
        game.load_dragon(tag, Some(uuid));
        assert_eq!(game.next_entity, before + 9);
        assert_eq!(game.dragon.as_ref().unwrap().id, before + 1);
        let entity = game.entity(before + 1).unwrap();
        assert!(entity.discard(game));
        game.native_removals_flush();
        assert!(game.dragon.is_none());
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn singleton_fight_dragon_install_reserves_nine_ids() {
    with_game(|game| {
        game.dragon.discard();
        let before = game.next_entity;
        game.create_new_dragon();
        let dragon = game.dragon.as_ref().unwrap();
        assert_eq!(dragon.id, before + 1);
        assert_eq!(game.next_entity, before + 9);
        assert!(dragon.join.is_joined());
        assert_eq!(
            game.fight.dragon_uuid,
            game.persist.uuids.get(&dragon.id).copied()
        );
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn singleton_client_dragon_install_explicitly_replaces_network_owner() {
    with_client_game(|game| {
        game.dragon.discard();
        let before = game.next_entity;
        game.spawn_remote_dragon(420, [1.0, 80.0, 2.0], 45.0);
        assert_eq!(game.dragon.as_ref().unwrap().id, 420);
        game.spawn_remote_dragon(840, [3.0, 90.0, 4.0], 90.0);
        let dragon = game.dragon.as_ref().unwrap();
        assert_eq!(dragon.id, 840);
        assert_eq!(dragon.position, Vec3::new(3.0, 90.0, 4.0));
        assert_eq!(game.next_entity, before);
        game.remove_remote_dragon(420);
        assert_eq!(game.dragon.as_ref().unwrap().id, 840);
        game.remove_remote_dragon(840);
        assert!(game.dragon.is_none());
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn singleton_late_dragon_id_reserves_all_eight_parts_once() {
    with_game(|game| {
        game.dragon.discard();
        assert!(game.dragon.install(Dragon::new(Vec3::ZERO, 0.0, 0)).is_ok());
        let before = game.next_entity;
        game.assign_dragon_and_crystal_ids();
        assert_eq!(game.dragon.as_ref().unwrap().id, before + 1);
        assert_eq!(game.next_entity, before + 9);
        game.assign_dragon_and_crystal_ids();
        assert_eq!(game.next_entity, before + 9);
    });
}
