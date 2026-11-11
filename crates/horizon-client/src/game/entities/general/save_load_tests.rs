//! #792 stage 3: the general record's live load and save, through the game
//! fixture. The record codec's own tests (`general.rs`) cover the format;
//! these cover what the game does with it: publish a tree through the same
//! join boundary a fresh spawn takes, remember its identities, save one
//! record per root (never for `noSave`), and refuse a whole tree — codec
//! fault, structural fault or a UUID a live entity already owns — with zero
//! published rows.
use super::*;
use crate::game::native_entity_refs::tests::with_game;
use std::collections::HashMap;

/// Declares the fixture's two types: `mymod:robot`, which saves, and
/// `mymod:ghost`, which never does.
struct Declare;

#[mod_api::__subscriptions]
impl mod_api::Hooks for Declare {
    fn register(&self, r: &mut dyn mod_api::Registrar, _: &mod_api::RegistrationContext) {
        for (path, save) in [("robot", true), ("ghost", false)] {
            r.declare_general_entity(
                path,
                mod_api::GeneralEntityCategory::Misc,
                0.8,
                1.9,
                if save {
                    mod_api::GeneralSavePolicy::Save
                } else {
                    mod_api::GeneralSavePolicy::NoSave
                },
                true,
                8,
                3,
                1,
                &[],
            )
            .unwrap();
        }
    }
}

/// A fixture game whose frozen registry holds the two declared types. The
/// registration phase is scoped around the whole fixture: the game harness
/// re-runs the installed mods' `register`, and a general type is only
/// declared while that phase is current.
fn with_types(test: impl FnOnce(&mut Game<'_>)) {
    let _phase = mod_api::__private::PhaseOwner::scope(mod_api::__private::ModPhase::Registration);
    let mods: mod_api::Mods = vec![mod_api::ModEntry::new("mymod", Box::new(Declare))].into();
    mod_api::testing::with_mods(mods, || with_game(test));
}

/// One record's save tag, the format `to_tag` writes.
fn record_tag(type_key: &str, uuid: [i32; 4], payload: &[u8], riders: Vec<Tag>) -> Tag {
    let mut tag = GeneralEntityRecord {
        position: [0.5, 64.0, 0.5],
        velocity: [0.0; 3],
        rotation: [0.0, 90.0],
        uuid,
        envelope: GeneralTypeEnvelope {
            type_key: type_key.to_owned(),
            payload_version: 3,
            payload_bytes: payload.to_vec(),
        },
        passengers: Vec::new(),
    }
    .to_tag();
    if let (Tag::Compound(map), false) = (&mut tag, riders.is_empty()) {
        map.insert(format::PASSENGERS.into(), Tag::List(riders));
    }
    tag
}

/// The saved rows of one chunk, through the same walk the save uses.
fn saved_chunk(game: &mut Game<'_>, key: ChunkKey) -> Vec<Tag> {
    let mut out: HashMap<ChunkKey, Vec<(u32, horizon_client::join_seq::JoinSeq, Tag)>> =
        HashMap::new();
    let mut keys = HashSet::new();
    keys.insert(key);
    game.save_every_entity(&keys, &mut out);
    out.remove(&key)
        .unwrap_or_default()
        .into_iter()
        .map(|(_, _, tag)| tag)
        .collect()
}

/// A loaded tree publishes its rows through the same join boundary a fresh
/// spawn takes: the root and each rider are rows of one list, the riders
/// mounted, and every identity remembered. Saving the chunk writes one
/// record per root — the riders inside it — with the payload bytes it
/// loaded.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn general_tree_loads_joins_and_saves_one_record() {
    with_types(|game| {
        let rider = record_tag("mymod:robot", [9, 9, 9, 9], b"rider", Vec::new());
        let root = record_tag("mymod:robot", [1, 2, 3, 4], b"payload", vec![rider]);
        game.load_listed("mymod:robot", None, None, root).unwrap();
        let rows = &game.resident_entities.general.rows;
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].uuid, [1, 2, 3, 4]);
        assert_eq!(rows[1].mount, Some(rows[0].id));
        assert_eq!(rows[0].payload_bytes, b"payload");
        assert_eq!(rows[0].payload_version, 3);
        assert!(rows[0].join.is_joined() && rows[1].join.is_joined());
        assert_eq!(game.persist.uuids.get(&rows[0].id), Some(&[1, 2, 3, 4]));
        assert_eq!(game.persist.uuids.get(&rows[1].id), Some(&[9, 9, 9, 9]));
        // One record per root, the rider inside it, the payload as loaded.
        let saved = saved_chunk(game, (0, 0));
        assert_eq!(saved.len(), 1);
        let Tag::List(riders) = saved[0].get(format::PASSENGERS).unwrap() else {
            panic!("the rider saves inside its root");
        };
        assert_eq!(riders.len(), 1);
        assert_eq!(
            saved[0].get(format::PAYLOAD),
            Some(&Tag::ByteArray(b"payload".to_vec()))
        );
        assert_eq!(
            saved[0].get(format::UUID),
            Some(&Tag::IntArray(vec![1, 2, 3, 4]))
        );
    });
}

/// `noSave`: instances load and live like any other, but produce zero
/// persisted records.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn no_save_type_writes_zero_records() {
    with_types(|game| {
        game.load_listed(
            "mymod:ghost",
            None,
            None,
            record_tag("mymod:ghost", [5, 6, 7, 8], b"", Vec::new()),
        )
        .unwrap();
        assert_eq!(game.resident_entities.general.rows.len(), 1);
        assert!(saved_chunk(game, (0, 0)).is_empty());
    });
}

/// A UUID a live entity of the level already owns refuses the whole tree:
/// zero rows publish, rider included, and the refusing record names it.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn duplicate_uuid_with_a_live_entity_refuses_the_whole_tree() {
    with_types(|game| {
        game.load_listed(
            "mymod:robot",
            None,
            None,
            record_tag("mymod:robot", [1, 2, 3, 4], b"", Vec::new()),
        )
        .unwrap();
        assert_eq!(game.resident_entities.general.rows.len(), 1);
        // A different root whose rider carries the live row's UUID.
        let rider = record_tag("mymod:robot", [1, 2, 3, 4], b"", Vec::new());
        let root = record_tag("mymod:robot", [8, 8, 8, 8], b"", vec![rider]);
        game.load_listed("mymod:robot", None, None, root).unwrap();
        assert_eq!(
            game.resident_entities.general.rows.len(),
            1,
            "the whole tree refuses, not only the duplicate rider"
        );
        assert_eq!(game.resident_entities.general.rows[0].uuid, [1, 2, 3, 4]);
    });
}

/// A codec refusal (an envelope field of the wrong shape) publishes zero
/// rows and leaves nothing behind: the next valid record loads cleanly.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn codec_refusal_publishes_nothing_and_poisons_nothing() {
    with_types(|game| {
        let mut broken = record_tag("mymod:robot", [1, 2, 3, 4], b"", Vec::new());
        if let Tag::Compound(map) = &mut broken {
            map.insert(format::PAYLOAD.into(), Tag::Int(1));
        }
        game.load_listed("mymod:robot", None, None, broken).unwrap();
        assert!(game.resident_entities.general.rows.is_empty());
        // The next valid record through the same entry point stands.
        game.load_listed(
            "mymod:robot",
            None,
            None,
            record_tag("mymod:robot", [1, 2, 3, 4], b"", Vec::new()),
        )
        .unwrap();
        assert_eq!(game.resident_entities.general.rows.len(), 1);
    });
}

/// A structural refusal (a NaN coordinate) publishes zero rows of the tree.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn structural_refusal_publishes_nothing() {
    with_types(|game| {
        let mut nan = record_tag("mymod:robot", [1, 2, 3, 4], b"", Vec::new());
        if let Tag::Compound(map) = &mut nan {
            map.insert(
                format::POSITION.into(),
                Tag::List(vec![
                    Tag::Double(f64::NAN.to_bits()),
                    Tag::Double(64.0f64.to_bits()),
                    Tag::Double(0.5f64.to_bits()),
                ]),
            );
        }
        game.load_listed("mymod:robot", None, None, nan).unwrap();
        assert!(game.resident_entities.general.rows.is_empty());
    });
}

/// A structurally sound tree with a type this load cannot resolve hands the
/// compound back (`Err`), keeping it for the missing mod's path instead of
/// publishing or dropping it.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn unknown_rider_type_hands_the_tree_back() {
    with_types(|game| {
        let rider = record_tag("gone:drone", [9, 9, 9, 9], b"", Vec::new());
        let root = record_tag("mymod:robot", [1, 2, 3, 4], b"", vec![rider]);
        let handed = root.clone();
        assert_eq!(
            game.load_listed("mymod:robot", None, None, root),
            Err(handed)
        );
        assert!(game.resident_entities.general.rows.is_empty());
    });
}
