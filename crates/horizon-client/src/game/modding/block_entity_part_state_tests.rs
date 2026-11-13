//! Part state (#781) on real block entities: one value each, kept with its block entity
//! through ticks, transactions, replacement, saving and unloading.
use std::{panic::AssertUnwindSafe, sync::Arc};

use horizon_client::{
    chunk::Chunk,
    containers::{BlockEntity, ModBlockEntity},
};
use horizon_core::nbt::{Compound, Tag};
use horizon_mod_runtime::attachments::{AttachmentCodec, AttachmentError, Attachments, Holder};
use mod_api::{
    AttachmentType, BlockEntityPart, BlockEntityPartState, BlockEntityRef, BlockEntityTickEvent,
    BlockPart, Hooks, HostTransaction, PartStateError, Registrar, ServerHost, edit_part_state,
    read_state,
};

use super::{Dim, Game};

static PROGRESS: AttachmentType<i32> = AttachmentType {
    namespace: "part_state",
    name: "progress",
    default: || 0,
    codec: Some(AttachmentCodec {
        encode: |n| Ok([("n".into(), Tag::Int(*n))].into()),
        decode: |saved: &Compound| match saved.get(&"n".into()) {
            Some(Tag::Int(n)) => Ok(*n),
            _ => Err(AttachmentError::Decode),
        },
    }),
    copy_on_death: false,
    sync: false,
};

/// Kept for the session only: no codec.
static SCRATCH: AttachmentType<i32> = AttachmentType {
    namespace: "part_state",
    name: "scratch",
    default: || 0,
    codec: None,
    copy_on_death: false,
    sync: false,
};

/// Counts its block entity's ticks in its state.
struct Press;
#[mod_api::horizon_part]
impl BlockPart for Press {}
#[mod_api::horizon_part]
impl BlockEntityPart for Press {
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let level = host.level(host.dimension()).unwrap();
        assert_eq!(host.block_entity(level, event.pos), None, "out for its tick");
        let entity = host.block_entity_identity(level, event.pos).unwrap();
        Press::edit_state(host, entity, |_, ticks| *ticks += 1).unwrap();
    }
}
impl BlockEntityPartState for Press {
    type Data = i32;
    const DATA: &'static AttachmentType<i32> = &PROGRESS;
}

struct StateMod;
#[mod_api::__subscriptions]
impl Hooks for StateMod {
    fn register(&self, r: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
        let kind = r
            .block_entity_type("press")
            .blocks(&["part_state:press"])
            .build();
        r.block_entity_part_with_state(&kind, Press).unwrap();
    }
    fn blocks(&self, _: &mod_api::RegistrationContext, e: &mut mod_api::BlocksEvent<'_>) {
        e.blocks
            .push(mod_api::Block::new("part_state:press", "minecraft:barrel").with_item(false));
    }
}

/// Run the test `name` with `StateMod` installed, in a fresh process: its content root
/// freezes the process's block entity types, which another test may already have frozen.
fn with_state_mod(name: &str, test: impl FnOnce(&mut Game<'_>)) {
    if super::native_entity_refs::tests::run_fresh_cases(
        name,
        "HORIZON_PART_STATE_GAME_CHILD",
        &["valid"],
    ) {
        return;
    }
    let mods: mod_api::Mods =
        vec![mod_api::ModEntry::new("part_state", Box::new(StateMod))].into();
    mod_platform::testing::with_mods(mods.clone(), || {
        crate::game::api_values::install_parts(&mods).unwrap();
        mod_api::__private::mark_game_thread();
        let base = std::path::PathBuf::from(std::env::var_os("HORIZON_TEST_RESOURCES").unwrap());
        let root = super::mods::content_root(&base, &Default::default()).unwrap();
        super::native_entity_refs::tests::with_game_at(root.prepared().to_owned(), |game| {
            game.options.dedicated = Some(Vec::new());
            game.native_level_opened(Dim::Overworld);
            game.world
                .chunks
                .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
            test(game);
        });
    });
}

/// A fresh press at `pos` in the level the game is in, and its handle.
fn press(game: &mut Game<'_>, pos: (i32, i32, i32)) -> BlockEntityRef {
    let block = game
        .pack
        .assets
        .registry
        .default_state("part_state:press")
        .unwrap();
    game.world.set(pos.0, pos.1, pos.2, block).unwrap();
    let entity = ModBlockEntity::new("part_state:press".into(), Default::default());
    game.world
        .block_entities
        .insert(pos, BlockEntity::Mod(Box::new(entity)));
    let level = game.level(game.dim.id()).unwrap();
    game.block_entity(level, [pos.0, pos.1, pos.2]).unwrap()
}

fn stored(game: &mut Game<'_>, entity: BlockEntityRef) -> Option<i32> {
    Press::state_existing(game, entity).unwrap()
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES; runs in an isolated scratch process"]
fn part_state_is_per_block_entity_through_ticks_transactions_and_replacement() {
    const NAME: &str = "game::block_entity_part_state_tests::part_state_is_per_block_entity_through_ticks_transactions_and_replacement";
    with_state_mod(NAME, |game| {
        let (a, b) = (press(game, (1, 64, 1)), press(game, (2, 64, 1)));
        // Reads insert nothing.
        assert_eq!(Press::state(game, a), Ok(0));
        assert_eq!(stored(game, a), None);
        assert_eq!(
            PROGRESS.has(game, Holder::BlockEntity([1, 64, 1])),
            Ok(false)
        );
        assert_eq!(Press::edit_state(game, a, |_, n| *n = 7), Ok(()));
        assert_eq!((stored(game, a), stored(game, b)), (Some(7), None));

        // Its own tick reaches it while it is out of the world, under the same handle.
        game.tick_mod_block_entity((1, 64, 1));
        game.tick_mod_block_entity((1, 64, 1));
        assert_eq!(stored(game, a), Some(9));
        assert_eq!(Press::state(game, b), Ok(0));

        // An inner edit commits into the outer transaction, which then aborts: the value and
        // the chunk's unsaved mark go back, so a chunk that was clean is clean again.
        game.world.modified.clear();
        let aborted: Result<(), ()> = (game as &mut dyn ServerHost).transaction(|host, _| {
            assert_eq!(Press::edit_state(host, a, |_, n| *n = 11), Ok(()));
            assert_eq!(Press::state(host, a), Ok(11), "later steps see it");
            assert!(
                !horizon_client::join_seq::visibility::passes_settled(),
                "no save while it is tentative"
            );
            Err(())
        });
        assert_eq!(aborted, Err(()));
        assert_eq!(stored(game, a), Some(9));
        assert!(!game.world.modified.contains(&(0, 0)), "clean after the abort");

        // A panic inside an edit leaves no default behind.
        let host: &mut dyn ServerHost = game;
        let panicked = std::panic::catch_unwind(AssertUnwindSafe(|| {
            Press::edit_state(host, b, |_, n| {
                *n = 5;
                panic!("edit fails");
            })
        }));
        assert!(panicked.is_err());
        assert!(game.transactions.is_empty());
        assert_eq!(stored(game, b), None);
        assert_eq!(Press::edit_state(game, b, |_, n| *n = 8), Ok(()));
        assert_eq!(stored(game, b), Some(8));

        // The same state can't be edited from inside its own edit; another key can.
        let inner = Press::edit_state(game, a, |host, n| {
            *n = 10;
            (
                Press::edit_state(host, a, |_, n| *n = 99),
                edit_part_state(host, a, &SCRATCH, |_, n| *n = 1),
            )
        });
        assert_eq!(inner, Ok((Err(PartStateError::Reentrant), Ok(()))));
        assert_eq!(stored(game, a), Some(10));
        assert_eq!(read_state(game, a, &SCRATCH), Ok(Some(1)));

        // A value replaced while the closure ran refuses the edit and keeps the old one.
        let changed = Press::edit_state(game, a, |host, n| {
            *n = 12;
            PROGRESS
                .set(host.attachments(), Holder::BlockEntity([1, 64, 1]), 50)
                .unwrap();
        });
        assert_eq!(changed, Err(PartStateError::Changed));
        assert_eq!(stored(game, a), Some(10));

        // An undecodable saved value refuses before the closure runs and keeps its bytes.
        let mut bad = Compound::new();
        bad.insert("n".into(), Tag::String("ten".into()));
        let mut saved = Attachments::default();
        PROGRESS.set(&mut saved, Holder::Level, 0).unwrap();
        let mut raw = saved.to_nbt();
        let Some(Tag::Compound(names)) = raw.get_mut(&"part_state".into()) else {
            unreachable!()
        };
        let Some(Tag::Compound(entry)) = names.get_mut(&"progress".into()) else {
            unreachable!()
        };
        entry.insert("value".into(), Tag::Compound(bad));
        game.world
            .block_entities
            .restore_attachments((2, 64, 1), Attachments::from_nbt(raw.clone()));
        let decode = PartStateError::Attachment {
            namespace: "part_state",
            name: "progress",
            error: AttachmentError::Decode,
        };
        assert_eq!(Press::state(game, b), Err(decode));
        let mut ran = false;
        game.world.modified.clear();
        assert_eq!(Press::edit_state(game, b, |_, _| ran = true), Err(decode));
        assert!(!ran);
        assert!(!game.world.modified.contains(&(0, 0)), "a refused edit marks nothing");
        assert_eq!(
            game.world
                .block_entities
                .attachments((2, 64, 1))
                .unwrap()
                .to_nbt(),
            raw
        );

        // A block entity of the same place in another level has its own value, and a
        // handle answers only in its own level.
        game.enter_level(Dim::Nether).unwrap();
        assert_eq!(Press::state(game, a), Err(PartStateError::OtherLevel));
        game.world
            .chunks
            .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
        let nether = press(game, (1, 64, 1));
        assert_eq!(Press::state(game, nether), Ok(0));
        assert_eq!(Press::edit_state(game, nether, |_, n| *n = 3), Ok(()));
        game.enter_level(Dim::Overworld).unwrap();
        assert_eq!(stored(game, a), Some(10));

        // A replacement at the same place is a new block entity with no value.
        game.world.block_entities.remove(&(1, 64, 1));
        let replacement = press(game, (1, 64, 1));
        assert_ne!(replacement, a);
        assert_eq!(Press::state(game, a), Err(PartStateError::Stale));
        assert_eq!(
            Press::edit_state(game, a, |_, n| *n = 1),
            Err(PartStateError::Stale)
        );
        assert_eq!(stored(game, replacement), None);
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES; runs in an isolated scratch process"]
fn part_state_saves_with_its_chunk_and_reloads_under_a_fresh_handle() {
    const NAME: &str = "game::block_entity_part_state_tests::part_state_saves_with_its_chunk_and_reloads_under_a_fresh_handle";
    with_state_mod(NAME, |game| {
        // A column far from the player, kept by a forced ticket while it is edited.
        let key = (80, 80);
        let pos = (1283, 100, 1283);
        // The test game starts with streaming paused, which never unloads.
        game.streaming.paused = false;
        assert!(game.streaming.set_forced(key.0, key.1, true));
        game.world
            .chunks
            .insert(key, Arc::new(Chunk::empty(key.0, key.1)));
        game.update_world();
        let entity = press(game, pos);
        game.world.modified.clear();
        assert_eq!(Press::edit_state(game, entity, |_, n| *n = 7), Ok(()));
        assert!(
            game.world.modified.contains(&key),
            "an edit marks its chunk for the unload's save"
        );
        assert_eq!(edit_part_state(game, entity, &SCRATCH, |_, n| *n = 4), Ok(()));

        // The ticket goes and the residency owner (`update_world`) unloads the column: it
        // saves it, then drops its block entities and their live bindings. Its sweep runs every
        // eighth update.
        assert!(game.streaming.set_forced(key.0, key.1, false));
        for _ in 0..9 {
            game.update_world();
        }
        assert!(!game.world.chunks.contains_key(&key), "unloaded");
        assert!(game.world.block_entities.attachments(pos).is_none());
        assert_eq!(Press::state(game, entity), Err(PartStateError::Stale));

        // Loaded back from the region file it was saved to, as streaming installs a column.
        game.save_level(true).unwrap();
        let mut chunk = game
            .save
            .load(key.0, key.1, game.pack.assets.registry, world_biomes!(game))
            .unwrap();
        game.world.install_chunk_block_entities(&mut chunk);
        game.world.chunks.insert(key, Arc::new(chunk));
        let level = game.level(game.dim.id()).unwrap();
        let reloaded = game.block_entity(level, [pos.0, pos.1, pos.2]).unwrap();
        assert_ne!(reloaded, entity);
        assert_eq!(Press::state(game, entity), Err(PartStateError::Stale));
        assert_eq!(stored(game, reloaded), Some(7));
        assert_eq!(read_state(game, reloaded, &SCRATCH), Ok(None), "session only");
    });
}

/// What the attachment index answers for `entity`'s `PROGRESS`, without the handle check.
fn indexed(game: &Game<'_>, entity: BlockEntityRef) -> Option<i32> {
    use mod_api::AttachmentQueries;
    let host: &dyn mod_api::Host = game;
    host.query(&PROGRESS)
        .unwrap()
        .get(mod_api::AttachmentHolder::BlockEntity(entity))
        .ok()
        .flatten()
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES; runs in an isolated scratch process"]
fn a_removal_retires_its_state_only_at_the_root_commit() {
    const NAME: &str = "game::block_entity_part_state_tests::a_removal_retires_its_state_only_at_the_root_commit";
    with_state_mod(NAME, |game| {
        let pos = (1, 64, 1);
        let a = press(game, pos);
        assert_eq!(Press::edit_state(game, a, |_, n| *n = 7), Ok(()));

        // Removed and replaced inside a transaction that then aborts: the handle is stale
        // while it is open, but the index keeps the map bound; the abort brings the very
        // binding back, and the tentative block entity's handle stays stale.
        let seal = game.transaction_open();
        game.journal_remove_block_entity(pos);
        assert_eq!(Press::state(game, a), Err(PartStateError::Stale));
        assert_eq!(indexed(game, a), Some(7), "still bound until the root commits");
        let tentative = press(game, pos);
        assert_eq!(Press::edit_state(game, tentative, |_, n| *n = 5), Ok(()));
        game.transaction_close(&seal, false);
        assert_eq!(Press::state(game, a), Ok(7));
        assert_eq!(indexed(game, a), Some(7));
        assert_eq!(Press::state(game, tentative), Err(PartStateError::Stale));
        assert_eq!(indexed(game, tentative), None);

        // A nested removal committed into its parent is still not published.
        let outer = game.transaction_open();
        let inner = game.transaction_open();
        game.journal_remove_block_entity(pos);
        game.transaction_close(&inner, true);
        assert_eq!(indexed(game, a), Some(7));
        // The root commit retires it.
        game.transaction_close(&outer, true);
        assert_eq!(Press::state(game, a), Err(PartStateError::Stale));
        assert_eq!(indexed(game, a), None);
        assert!(game.world.block_entities.attachments(pos).is_none());
    });
}

#[path = "../../../../horizon-mod-wasm/tests/fixture/src/part_state.rs"]
mod fixture;

struct NativeFixture;
#[mod_api::__subscriptions]
impl Hooks for NativeFixture {
    fn register(&self, r: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
        fixture::register(r);
    }
}

/// The fixture's Part counts its ticks natively and compiled to wasm alike: each tick's aborted
/// edit leaves nothing, its recursive edit is refused, and its own edit commits. The game reads
/// the count from the saved form, which a wasm value has on the host.
#[test]
#[ignore = "needs prepared resources and compiled HORIZON_WASM_FIXTURE/HORIZON_WASM_STD"]
fn native_and_compiled_wasm_part_state_count_ticks() {
    const NAME: &str =
        "game::block_entity_part_state_tests::native_and_compiled_wasm_part_state_count_ticks";
    if super::native_entity_refs::tests::run_fresh_cases(
        NAME,
        "PART_STATE_TARGET",
        &["native", "wasm"],
    ) {
        return;
    }
    horizon_guest_stack::run_on_guest_stack(|| {
        let mods: mod_api::Mods = if std::env::var("PART_STATE_TARGET").unwrap() == "wasm" {
            super::block_entity_tick_reply_tests::wasm_mods()
        } else {
            vec![mod_api::ModEntry::new("wasm_fixture", Box::new(NativeFixture))]
        }
        .into();
        mod_platform::testing::with_mods(mods.clone(), || {
            crate::game::api_values::install_parts(&mods).unwrap();
            mod_api::__private::mark_game_thread();
            let base =
                std::path::PathBuf::from(std::env::var_os("HORIZON_TEST_RESOURCES").unwrap());
            let root = super::mods::content_root(&base, &Default::default()).unwrap();
            super::native_entity_refs::tests::with_game_at(root.prepared().to_owned(), |game| {
                game.options.dedicated = Some(Vec::new());
                game.world
                    .chunks
                    .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
                let stone = game
                    .pack
                    .assets
                    .registry
                    .default_state("minecraft:stone")
                    .unwrap();
                game.world.set(1, 64, 1, stone).unwrap();
                let entity =
                    ModBlockEntity::new("wasm_fixture:part_state".into(), Default::default());
                game.world
                    .block_entities
                    .insert((1, 64, 1), BlockEntity::Mod(Box::new(entity)));
                for _ in 0..3 {
                    game.tick_mod_block_entity((1, 64, 1));
                }
                assert!(game.transactions.is_empty());
                let saved = game
                    .world
                    .block_entities
                    .attachments((1, 64, 1))
                    .unwrap()
                    .to_nbt();
                let count = saved
                    .get(&"wasm_fixture".into())
                    .and_then(|names| match names {
                        Tag::Compound(names) => names.get(&"part_state_count".into()),
                        _ => None,
                    })
                    .and_then(|entry| match entry {
                        Tag::Compound(entry) => entry.get(&"value".into()),
                        _ => None,
                    });
                assert_eq!(
                    count,
                    Some(&Tag::Compound([("n".into(), Tag::Int(3))].into()))
                );
            });
        });
    });
}
