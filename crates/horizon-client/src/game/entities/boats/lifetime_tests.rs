use super::*;
use horizon_mod_runtime::attachments::{
    AttachmentType, Attachments, HasAttachmentLifetime, Holder, index::AttachmentIndex,
};
use std::sync::Mutex;

static VALUE: AttachmentType<i32> = AttachmentType {
    namespace: "test",
    name: "boat_lifetime",
    default: || 0,
    codec: None,
    copy_on_death: false,
    sync: false,
};

#[test]
fn a_tick_clone_follows_its_join_but_cannot_replace_a_later_resident() {
    let mut boats = crate::join_seq::JoinList::<Boat, false>::default();
    for id in [1, 2] {
        boats.push(Boat::new(id, "oak".into(), Vec3::ZERO, 0.0));
    }
    let mut clone = boats[1].clone();
    clone.age = 7;
    boats.remove(0);
    assert!(boats.store_back(1, clone));
    assert_eq!(boats[0].id, 2);
    assert_eq!(boats[0].age, 7);
    let stale = boats.pop().unwrap().clone();
    boats.push(Boat::new(3, "oak".into(), Vec3::ZERO, 0.0));
    assert!(!boats.store_back(0, stale));
    assert_eq!(boats[0].id, 3);
    assert_eq!(boats[0].age, 0);
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_bound_boat_keeps_its_registration_through_ticks_until_discard() {
    super::super::native_entity_refs::tests::with_game(|game| {
        game.boats
            .push(Boat::new(71, "oak".into(), Vec3::new(0.5, 70.0, 0.5), 0.0));
        let holder = {
            let host: &dyn mod_api::ServerHost = game;
            host.entity(71).unwrap()
        };
        let index = Arc::new(Mutex::new(AttachmentIndex::default()));
        let mut values = Attachments::default();
        VALUE.set(&mut values, Holder::Entity(71), 10).unwrap();
        let original = game.boats.update(0, |boat| {
            boat.attachment_lifetime().bind(&mut values, &index, holder);
            boat.attachment_lifetime().registration().unwrap()
        });
        let key = ("test".into(), "boat_lifetime".into());
        let before = index.lock().unwrap().snapshot(&key);
        assert_eq!(before.len(), 1);
        for _ in 0..3 {
            let clone = game.tick_boat(game.boats[0].clone(), false);
            assert!(game.store_boat_back(0, clone));
            let current = game
                .boats
                .update(0, |boat| boat.attachment_lifetime().registration().unwrap());
            assert!(original.same_registration(&current));
            assert_eq!(index.lock().unwrap().snapshot(&key).len(), 1);
            assert_eq!(VALUE.get(&mut values, Holder::Entity(71)), Ok(10));
        }
        assert_eq!(game.boats[0].age, 3);
        let taken = game.take_entities();
        assert_eq!(index.lock().unwrap().snapshot(&key).len(), 1);
        game.restore_entities(taken);
        VALUE.set(&mut values, Holder::Entity(71), 20).unwrap();
        let after = index.lock().unwrap().snapshot(&key);
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].holder, holder);
        assert_eq!(after[0].entry.read(&VALUE), Ok(&20));
        game.persist
            .removals
            .push(super::super::commands::Target::Boat(71));
        let clone = game.boats[0].clone();
        assert!(!game.store_boat_back(0, clone));
        assert_eq!(index.lock().unwrap().snapshot(&key).len(), 1);
        game.native_removals_flush();
        assert!(game.boats.is_empty());
        assert!(index.lock().unwrap().snapshot(&key).is_empty());
        VALUE.set(&mut values, Holder::Entity(71), 30).unwrap();
        assert!(index.lock().unwrap().snapshot(&key).is_empty());
        assert_eq!(before[0].entry.read(&VALUE), Ok(&10));
        assert_eq!(after[0].entry.read(&VALUE), Ok(&20));
    });
}
