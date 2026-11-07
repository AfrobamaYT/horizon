use super::*;
use crate::game::native_entity_refs::tests::with_game;
use horizon_core::nbt::Tag;
use mod_api::{AttachmentQueries, AttachmentQueriesMut, AttachmentType};

static VALUE: AttachmentType<i32> = AttachmentType {
    namespace: "test",
    name: "mana",
    default: || 0,
    codec: None,
    copy_on_death: false,
    sync: false,
};

fn mob(uuid: [i32; 4]) -> Tag {
    Tag::compound([
        ("id", Tag::String("minecraft:cow".into())),
        ("UUID", Tag::IntArray(uuid.to_vec())),
        (
            "Pos",
            Tag::List(vec![Tag::Double(1.0), Tag::Double(70.0), Tag::Double(3.0)]),
        ),
        (
            horizon_mod_runtime::attachments::ATTACHMENTS_KEY,
            Tag::compound([(
                "test",
                Tag::compound([("mana", Tag::compound([("value", Tag::compound([]))]))]),
            )]),
        ),
    ])
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn saved_mob_binding_survives_store_back_and_retires_at_sweep() {
    with_game(|game| {
        game.options.dedicated = Some(Vec::new());
        game.native_level_opened(Dim::Overworld);
        let uuid = [17, 18, 19, 20];
        let id = game.load_mob(mob(uuid), None).unwrap();
        // Check residency before a typed read decodes the saved entry.
        let index = game.attachment_indexes[&Dim::Overworld].clone();
        let key = ("test".into(), "mana".into());
        assert_eq!(index.lock().unwrap().snapshot(&key).len(), 1);
        VALUE.set(game, Holder::Entity(id), 7).unwrap();
        let old = game.query_mut(&VALUE).unwrap();
        let row = old.iter().next().unwrap().unwrap();
        let registration = game.mobs[0].attachment_lifetime.registration().unwrap();
        let replacement = game.mobs[0].clone();
        game.mobs.store_back(0, replacement);
        assert!(game.mobs[0].attachment_lifetime.is_bound());
        assert_eq!(index.lock().unwrap().snapshot(&key).len(), 1);
        assert_eq!(VALUE.get(game, Holder::Entity(id)), Ok(7));
        assert_eq!(
            game.query(&VALUE)
                .unwrap()
                .iter()
                .next()
                .unwrap()
                .unwrap()
                .value,
            &7
        );
        drop(registration);
        let entity = game.entity(id).unwrap();
        assert!(game.entity_discard(entity));
        assert_eq!(index.lock().unwrap().snapshot(&key).len(), 1);
        game.native_removals_flush();
        assert!(game.mobs.is_empty());
        assert!(index.lock().unwrap().snapshot(&key).is_empty());
        assert_eq!(
            row.set(game, 9),
            Err(mod_api::AttachmentRowError::StaleHolder)
        );
        assert_eq!(*row.value(), 7);
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn saved_mob_binding_tracks_serialized_transfer_and_parked_levels() {
    with_game(|game| {
        game.options.dedicated = Some(Vec::new());
        game.native_level_opened(Dim::Overworld);
        let uuid = [21, 22, 23, 24];
        let id = game.load_mob(mob(uuid), None).unwrap();
        let source = game.attachment_indexes[&Dim::Overworld].clone();
        let key = ("test".into(), "mana".into());
        assert_eq!(source.lock().unwrap().snapshot(&key).len(), 1);
        VALUE.set(game, Holder::Entity(id), 11).unwrap();
        let batch = game.query_mut(&VALUE).unwrap();
        let row = batch.iter().next().unwrap().unwrap();
        game.enter_level(Dim::Nether).unwrap();
        assert_eq!(source.lock().unwrap().snapshot(&key).len(), 1);
        game.enter_level(Dim::Overworld).unwrap();
        assert_eq!(VALUE.get(game, Holder::Entity(id)), Ok(11));
        // Serialized portals discard the source owner before destination load.
        game.mobs.retain(|m| m.id != id);
        game.detach_entity(id);
        assert!(source.lock().unwrap().snapshot(&key).is_empty());
        game.enter_level(Dim::Nether).unwrap();
        let moved = game.load_mob(mob(uuid), None).unwrap();
        assert_ne!(moved, id); // This route allocates; retained-id vehicles have their own arrival seam.
        assert_eq!(
            game.attachment_indexes[&Dim::Nether]
                .lock()
                .unwrap()
                .snapshot(&key)
                .len(),
            1
        );
        assert_eq!(VALUE.get(game, Holder::Entity(moved)), Ok(11));
        assert_eq!(
            row.set(game, 12),
            Err(mod_api::AttachmentRowError::StaleHolder)
        );
        assert_eq!(*row.value(), 11);
        game.mobs.clear();
        assert!(
            game.attachment_indexes[&Dim::Nether]
                .lock()
                .unwrap()
                .snapshot(&key)
                .is_empty()
        );
    });
}
