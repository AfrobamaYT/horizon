//! #792 stage 4: the general rows' tick pass through the game fixture. The
//! fixture part advances a per-row counter attachment (#796: the sole
//! per-entity store) and records the callback trace; the tests pin the
//! acceptance matrix's ordering, residency and containment rows against the
//! pass's own contract, with the vanilla orderings cited where they hold.
use super::*;
use crate::game::native_entity_refs::tests::with_game;
use horizon_core::nbt::{Compound, Tag};
use mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, EntityPart, EntityTickListenedEvent,
    EntityTickPreEvent, Flow, GeneralEntityCategory, GeneralEntityTickEvent, GeneralSavePolicy,
    Holder, Hooks, Registrar, ServerHost,
};
use std::sync::{Arc, Mutex};

/// The fixture counter: an attachment with a codec, so a saved row resumes
/// its count and a quarantined record keeps it in its bytes.
static COUNTER: AttachmentType<i32> = AttachmentType {
    namespace: "mymod",
    name: "counter",
    default: || 0,
    codec: Some(AttachmentCodec {
        encode: |n| Ok([("n".into(), Tag::Int(*n))].into()),
        decode: |tag: &Compound| match tag.get(&"n".into()) {
            Some(Tag::Int(n)) => Ok(*n),
            _ => Err(AttachmentError::Decode),
        },
    }),
    copy_on_death: false,
    sync: false,
};

/// What the part saw, in call order: each row's id.
type Trace = Arc<Mutex<Vec<u32>>>;

/// The fixture part: `test:counter` of the acceptance matrix. It carries no
/// per-instance state — two rows of one type share this one part, and their
/// counters live apart in their attachments.
struct Counter {
    trace: Trace,
    /// Panics on this row's first tick only, after its write: the
    /// containment row's failing subscriber (0 arms nothing — a test sets
    /// its target once its rows have ids).
    panic_on: Arc<std::sync::atomic::AtomicU32>,
    fired: std::sync::atomic::AtomicBool,
}

#[mod_api::horizon_part]
impl EntityPart for Counter {
    fn general_entity_tick(
        &self,
        host: &mut dyn ServerHost,
        event: &mut GeneralEntityTickEvent<'_>,
    ) {
        let holder = Holder::Entity(event.entity.id());
        let value = COUNTER.get(host, holder).unwrap() + 1;
        COUNTER.set(host, holder, value).unwrap();
        if self.panicking_on() == Some(event.entity.id())
            && !self.fired.swap(true, std::sync::atomic::Ordering::Relaxed)
        {
            panic!("general entity tick boom");
        }
        self.trace.lock().unwrap().push(event.entity.id());
    }
}

impl Counter {
    fn panicking_on(&self) -> Option<u32> {
        let target = self.panic_on.load(std::sync::atomic::Ordering::Relaxed);
        (target != 0).then_some(target)
    }
}

/// Declares the two fixture types and attaches the counter part to
/// `mymod:robot`; `ghost` stays part-free for the zero-callback rows. The
/// same mod listens to the entity tick gates, and cancels the row named by
/// `cancel` when armed (the Pre row of the matrix).
struct Declare {
    trace: Trace,
    panic_on: Arc<std::sync::atomic::AtomicU32>,
    cancel: Arc<std::sync::atomic::AtomicBool>,
}

#[mod_api::__subscriptions]
impl Hooks for Declare {
    fn register(&self, r: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
        for (path, save) in [("robot", true), ("ghost", false)] {
            r.declare_general_entity(
                path,
                GeneralEntityCategory::Misc,
                0.8,
                1.9,
                if save {
                    GeneralSavePolicy::Save
                } else {
                    GeneralSavePolicy::NoSave
                },
                true,
                8,
                3,
                1,
                &[],
            )
            .unwrap();
        }
        r.entity_part(
            "mymod:robot",
            Counter {
                trace: self.trace.clone(),
                panic_on: self.panic_on.clone(),
                fired: std::sync::atomic::AtomicBool::new(false),
            },
        );
    }
    fn entity_tick_listened(&self, _: &mut dyn ServerHost, event: &mut EntityTickListenedEvent) {
        event.listened = true;
    }
    fn entity_tick_pre(&self, _: &mut dyn ServerHost, event: &mut EntityTickPreEvent<'_>) -> Flow {
        if self.cancel.load(std::sync::atomic::Ordering::Relaxed) && event.kind == "mymod:robot" {
            return Flow::Cancel;
        }
        Flow::Continue
    }
}

/// A counter fixture game. The registration phase is scoped around the whole
/// fixture: the harness re-runs the mods' `register` while the game builds,
/// and a general type is only declared while that phase is current. The
/// runtime initializes before the scope: a scope entered before the platform
/// installs its instrumentation writes a fallback nobody reads, and the
/// declaration then refuses as frozen.
fn with_counter(
    test: impl FnOnce(
        &mut Game<'_>,
        Trace,
        Arc<std::sync::atomic::AtomicBool>,
        Arc<std::sync::atomic::AtomicU32>,
    ),
) {
    let trace: Trace = Arc::default();
    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let panic_on = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let declare = Declare {
        trace: trace.clone(),
        panic_on: panic_on.clone(),
        cancel: cancel.clone(),
    };
    mod_api::__private::initialize().expect("test registration requires the mod API runtime");
    let _phase = mod_api::__private::PhaseOwner::scope(mod_api::__private::ModPhase::Registration);
    let mods: mod_api::Mods = vec![mod_api::ModEntry::new("mymod", Box::new(declare))].into();
    mod_api::testing::with_mods(mods, || {
        with_game(|game| {
            // The fixture speaks as a server, as the vehicle boundary tests
            // make theirs: attachment writes and the entity tick gates answer
            // only an authoritative game.
            game.options.dedicated = Some(Vec::new());
            game.native_level_opened(crate::game::Dim::Overworld);
            game.build_registries();
            game.reset_entity_residency();
            // The registration phase ends before the test's own ticks: a
            // part's writes are storage writes, which only a world's phases
            // take, as the server's own tick runs in World.
            let _world = mod_api::__private::PhaseOwner::scope(mod_api::__private::ModPhase::World);
            test(game, trace, cancel, panic_on)
        })
    });
}

/// One record's save tag at `at`, the format `to_tag` writes.
fn record_at(type_key: &str, uuid: [i32; 4], at: [f64; 3], riders: Vec<Tag>) -> Tag {
    let mut tag = GeneralEntityRecord {
        position: at,
        velocity: [0.0; 3],
        rotation: [0.0, 90.0],
        uuid,
        envelope: GeneralTypeEnvelope {
            type_key: type_key.to_owned(),
            payload_version: 3,
            payload_bytes: Vec::new(),
        },
        passengers: Vec::new(),
    }
    .to_tag();
    if let (Tag::Compound(map), false) = (&mut tag, riders.is_empty()) {
        map.insert(format::PASSENGERS.into(), Tag::List(riders));
    }
    tag
}

/// Spawns one tree of `mymod:robot` records and answers its root's row id.
fn spawn_tree(game: &mut Game<'_>, root: [i32; 4], at: [f64; 3], riders: Vec<Tag>) -> u32 {
    game.load_listed(
        "mymod:robot",
        None,
        None,
        record_at("mymod:robot", root, at, riders),
    )
    .unwrap();
    game.resident_entities
        .general
        .rows
        .iter()
        .find(|row| row.uuid == root)
        .unwrap()
        .id
}

/// The rows' ids in storage order.
fn row_ids(game: &Game<'_>) -> Vec<u32> {
    game.resident_entities
        .general
        .rows
        .iter()
        .map(|row| row.id)
        .collect()
}

/// One entity phase over the current level: what `tick_world` does between
/// its ordered runs, driven by hand. The spawn chunks tick with nobody
/// watching, as vanilla's do (`tickets.update(None)`).
fn tick(game: &mut Game<'_>) {
    game.tickets.update(None);
    game.begin_ordered_ticks();
    game.tick_general().unwrap();
}

/// A row's counter, read through the same host surface the part writes.
fn counter(game: &mut Game<'_>, id: u32) -> i32 {
    COUNTER.get(game, Holder::Entity(id)).unwrap()
}

/// One robot row, its counter already at `value`.
fn seeded_row(game: &mut Game<'_>, uuid: [i32; 4], value: i32) -> u32 {
    let id = spawn_tree(game, uuid, [0.5, 64.0, 0.5], Vec::new());
    COUNTER.set(game, Holder::Entity(id), value).unwrap();
    id
}

/// One tick of `test:counter` (the matrix's single row): 40 becomes 41.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn counter_advances_one_per_tick() {
    with_counter(|game, trace, _, _| {
        let id = seeded_row(game, [1, 2, 3, 4], 40);
        assert_eq!(counter(game, id), 40);
        tick(game);
        assert_eq!(counter(game, id), 41);
        assert_eq!(*trace.lock().unwrap(), [id]);
    });
}

/// Two instances of one type: the part is shared, their counters are not.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn two_rows_share_the_part_but_not_their_counters() {
    with_counter(|game, trace, _, _| {
        let first = seeded_row(game, [1, 2, 3, 4], 40);
        let second = seeded_row(game, [5, 6, 7, 8], 7);
        tick(game);
        assert_eq!((counter(game, first), counter(game, second)), (41, 8));
        assert_eq!(*trace.lock().unwrap(), [first, second]);
    });
}

/// A branching tree three generations deep (root, two children, a
/// grandchild under the first): one phase gives each row its 1, in
/// `tickNonPassenger`'s order — the root, then its rider subtree depth-first
/// (`tickPassenger` recurses before the next rider), a rider never a root.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn parent_ticks_before_its_depth_first_rider_subtree() {
    with_counter(|game, trace, _, _| {
        let grandchild = record_at("mymod:robot", [8, 8, 8, 8], [0.5, 64.0, 0.5], Vec::new());
        let rider = record_at(
            "mymod:robot",
            [9, 9, 9, 9],
            [0.5, 64.0, 0.5],
            vec![grandchild],
        );
        let sibling = record_at("mymod:robot", [7, 7, 7, 7], [0.5, 64.0, 0.5], Vec::new());
        let root = spawn_tree(game, [1, 2, 3, 4], [0.5, 64.0, 0.5], vec![rider, sibling]);
        let ids = row_ids(game);
        let (rider_id, grandchild_id, sibling_id) = (ids[1], ids[2], ids[3]);
        tick(game);
        assert_eq!(
            *trace.lock().unwrap(),
            [root, rider_id, grandchild_id, sibling_id]
        );
        for id in ids {
            assert_eq!(counter(game, id), 1);
        }
    });
}

/// The join-cutoff row: a row that joins during the tick's span (here at or
/// above `begin_ordered_ticks`' `born` cutoff — a mod's own spawn path,
/// which stage 4 does not ship, joins exactly there) first ticks the next
/// phase, the vanilla tick list's passive swap.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_row_joined_during_the_tick_span_ticks_the_next_phase() {
    with_counter(|game, trace, _, _| {
        let first = seeded_row(game, [1, 2, 3, 4], 40);
        game.tickets.update(None);
        game.begin_ordered_ticks();
        let late = spawn_tree(game, [5, 6, 7, 8], [0.5, 64.0, 0.5], Vec::new());
        game.tick_general().unwrap();
        assert_eq!(*trace.lock().unwrap(), [first]);
        assert_eq!(counter(game, late), 0);
        trace.lock().unwrap().clear();
        tick(game);
        assert_eq!(*trace.lock().unwrap(), [first, late]);
        assert_eq!(counter(game, late), 1);
    });
}

/// The re-entry oracle: vanilla's `EntityTickList.add` appends to the active
/// map's end, and a level's entities re-enter ticking through the section
/// callbacks when their chunk loads again, so a row that left and returned
/// ticks after a row that joined later and never left. Horizon's join order
/// is that insertion order (Ari's #792 ruling); the row's fresh admission on
/// reload is its re-entry.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_reloaded_row_reenters_ticking_behind_later_joins() {
    with_counter(|game, trace, _, _| {
        let early = spawn_tree(game, [1, 2, 3, 4], [0.5, 64.0, 0.5], Vec::new());
        let late = spawn_tree(game, [5, 6, 7, 8], [1024.5, 64.0, 0.5], Vec::new());
        tick(game);
        assert_eq!(*trace.lock().unwrap(), [early, late]);
        // The early row's chunk unloads (the later row's stays loaded).
        let mut chunks = game.entity_chunks(&HashSet::from([(0, 0)]), true);
        let (_, mut records) = chunks.remove(0);
        assert_eq!(records.len(), 1);
        trace.lock().unwrap().clear();
        // It loads back into a fresh join, behind the row that never left.
        game.load_listed("mymod:robot", None, None, records.remove(0))
            .unwrap();
        let rows = &game.resident_entities.general.rows;
        let reloaded = rows.iter().find(|row| row.uuid == [1, 2, 3, 4]).unwrap();
        let (early_id, early_join) = (reloaded.id, reloaded.join);
        let late_join = rows
            .iter()
            .find(|row| row.uuid == [5, 6, 7, 8])
            .unwrap()
            .join;
        assert!(
            late_join < early_join,
            "the reloaded row rejoined at the end"
        );
        tick(game);
        assert_eq!(*trace.lock().unwrap(), [late, early_id]);
    });
}

/// The save row: a row saved at 41 loads back with its counter and ticks 42
/// from the next phase; its old handle — the id is spent, never reused
/// (`next_entity` only rises) — no longer reaches the row.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_reloaded_row_resumes_the_counter_and_refuses_its_old_handle() {
    with_counter(|game, _, _, _| {
        let old = seeded_row(game, [1, 2, 3, 4], 41);
        let mut chunks = game.entity_chunks(&HashSet::from([(0, 0)]), true);
        let (_, mut records) = chunks.remove(0);
        game.load_listed("mymod:robot", None, None, records.remove(0))
            .unwrap();
        let rows = &game.resident_entities.general.rows;
        let fresh = rows[0].id;
        assert_ne!(fresh, old);
        assert_eq!(counter(game, fresh), 41);
        assert!(COUNTER.get(game, Holder::Entity(old)).is_err());
        tick(game);
        assert_eq!(counter(game, fresh), 42);
    });
}

/// The parking row: a parked level's rows persist untouched — the pass only
/// runs for the level in place — and the first tick after it returns gives
/// the next count.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_parked_level_holds_the_counter_until_it_returns() {
    with_counter(|game, trace, _, _| {
        let id = seeded_row(game, [1, 2, 3, 4], 41);
        game.enter_level(crate::game::Dim::Nether).unwrap();
        for _ in 0..10 {
            tick(game);
        }
        assert!(trace.lock().unwrap().is_empty(), "nothing ticked");
        // The row's counter reads through its own level's routing, so read it
        // once its level is back in place: ten parked ticks left it, and the
        // first tick after the return takes it.
        game.enter_level(crate::game::Dim::Overworld).unwrap();
        assert_eq!(counter(game, id), 41, "the parked level's rows do not tick");
        tick(game);
        assert_eq!(counter(game, id), 42);
    });
}

/// The containment row: a native panic in a row's tick rolls the row's
/// writes back (the counter keeps its value), the later row ticks on, and
/// the panicking row's own next tick proceeds. A subscriber is not
/// auto-disabled.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_panicking_row_rolls_back_and_the_next_tick_proceeds() {
    with_counter(|game, trace, _, panic_on| {
        let first = seeded_row(game, [1, 2, 3, 4], 41);
        let second = seeded_row(game, [5, 6, 7, 8], 7);
        panic_on.store(first, std::sync::atomic::Ordering::Relaxed);
        tick(game);
        assert_eq!(
            *trace.lock().unwrap(),
            [second],
            "only the later row ticked"
        );
        assert_eq!(counter(game, first), 41, "the panic rolled the write back");
        assert_eq!(counter(game, second), 8);
        tick(game);
        assert_eq!(*trace.lock().unwrap(), [second, first, second]);
        assert_eq!(counter(game, first), 42);
    });
}

/// The Pre row: a canceled `entity_tick_pre` skips the row's behavior and
/// its whole rider subtree this tick (the vanilla event wraps
/// `tickNonPassenger`); nothing is written and no callback runs.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_canceled_pre_skips_the_row_and_its_subtree() {
    with_counter(|game, trace, cancel, _| {
        let rider = record_at("mymod:robot", [9, 9, 9, 9], [0.5, 64.0, 0.5], Vec::new());
        let root = spawn_tree(game, [1, 2, 3, 4], [0.5, 64.0, 0.5], vec![rider]);
        tick(game);
        assert_eq!(trace.lock().unwrap().len(), 2);
        trace.lock().unwrap().clear();
        cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        tick(game);
        assert!(trace.lock().unwrap().is_empty());
        assert_eq!(counter(game, root), 1);
        cancel.store(false, std::sync::atomic::Ordering::Relaxed);
        tick(game);
        assert_eq!(trace.lock().unwrap().len(), 2);
    });
}

/// The zero-callback row: a thousand rows of a type no part subscribes to
/// cost the pass one walk and no callback — no call, no event, no type
/// lookup (the slot resolved without subscribers before the walk began).
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_thousand_part_free_rows_call_nothing() {
    with_counter(|game, trace, _, _| {
        for index in 0..1000 {
            game.load_listed(
                "mymod:ghost",
                None,
                None,
                record_at(
                    "mymod:ghost",
                    [index, 0, 0, 0],
                    [0.5, 64.0, 0.5],
                    Vec::new(),
                ),
            )
            .unwrap();
        }
        assert_eq!(game.resident_entities.general.rows.len(), 1000);
        tick(game);
        assert!(trace.lock().unwrap().is_empty());
        assert_eq!(game.resident_entities.general.rows.len(), 1000);
    });
}

/// #3458's first uncovered test, on the tick's own fixture: a row of a
/// missing mod quarantines byte for byte, survives a second save unchanged,
/// and reinstating the mod restores the row with its counter — 41 resumed,
/// 42 on the next tick.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn quarantine_then_reinstall_restores_the_counter() {
    // With the mod: spawn at 41 and save the record out.
    let mut saved = None;
    with_counter(|game, _, _, _| {
        let id = seeded_row(game, [1, 2, 3, 4], 41);
        tick(game);
        assert_eq!(counter(game, id), 42);
        let mut chunks = game.entity_chunks(&HashSet::from([(0, 0)]), true);
        let (_, mut records) = chunks.remove(0);
        saved = Some(records.remove(0));
    });
    let saved = saved.unwrap();
    // Without it: the record quarantines (its namespace belongs to no
    // loaded package), and a save of that chunk writes the very same bytes
    // back.
    with_game(|game| {
        game.others.clear();
        game.spawn_loaded_entity((0, 0), saved.clone(), None);
        assert_eq!(game.others.len(), 1);
        let mut chunks = game.entity_chunks(&HashSet::from([(0, 0)]), false);
        let (_, mut records) = chunks.remove(0);
        assert_eq!(records.remove(0), saved, "the second save keeps the bytes");
    });
    // With the mod again: the row returns as itself, counter and all.
    with_counter(|game, trace, _, _| {
        game.load_listed("mymod:robot", None, None, saved).unwrap();
        let rows = &game.resident_entities.general.rows;
        let id = rows[0].id;
        assert_eq!(rows[0].uuid, [1, 2, 3, 4]);
        assert_eq!(counter(game, id), 42);
        tick(game);
        assert_eq!(*trace.lock().unwrap(), [id]);
        assert_eq!(counter(game, id), 43);
    });
}

/// #3458's second uncovered test: a summon's insertion of a general type —
/// the path `/summon` drives, `spawn_entity_from_nbt` — answers a target
/// whose network id and position read back (a general row's network id is
/// its own id, until stage 5's tracking gives it a connection-local one).
/// The command's own argument parser still names only vanilla entity types;
/// feeding it the declared general types is the negotiation's follow-up.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn summon_answers_a_target_with_a_network_id() {
    with_counter(|game, _, _, _| {
        let mut tag = record_at("mymod:robot", [1, 2, 3, 4], [0.5, 64.0, 0.5], Vec::new());
        if let Tag::Compound(map) = &mut tag {
            map.insert("id".into(), Tag::String("mymod:robot".into()));
        }
        let target = game.spawn_entity_from_nbt(tag).unwrap();
        let rows = &game.resident_entities.general.rows;
        assert_eq!(rows.len(), 1);
        let id = rows[0].id;
        assert_eq!(target, crate::game::commands::Target::General(id));
        assert_eq!(
            game.network_entity_id(crate::game::commands::Target::General(id)),
            Some(id as i32)
        );
        assert_eq!(
            game.target_position(crate::game::commands::Target::General(id)),
            Some([0.5, 64.0, 0.5])
        );
    });
}
