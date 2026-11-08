#![cfg(feature = "wasm-guest")]

//! A real catalog free-function point with a `callback` argument, `parse(depth: u32, load:
//! &dyn Fn(&str) -> Option<u32>) -> Option<u32>`. The closure is an
//! opaque leaf: the listener sees an empty typed header, no edit reaches it, and the native
//! body still calls the real closure.
mod common;
use common::Endpoint;
use mod_api::RowWindow;
use mod_points::{__private::SlotListener, PointId, endpoint_policy::EndpointPolicy};
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

include!(concat!(env!("OUT_DIR"), "/native_raw_callback.rs"));

static MODE: AtomicUsize = AtomicUsize::new(0);
static HEADS: AtomicUsize = AtomicUsize::new(0);
static REFUSED: AtomicBool = AtomicBool::new(false);
static HEAD: OnceLock<Endpoint> = OnceLock::new();

fn head(snapshot: mod_points::snapshot::Snapshot<'_>, edits: &mut mod_api::wire::RecordPatch) {
    HEADS.fetch_add(1, Ordering::Relaxed);
    // Roots: depth, load, result.
    let depth = snapshot.field(0, 0).unwrap();
    let load = snapshot.field(0, 1).unwrap();
    assert!(snapshot.field(0, 3).is_none());
    assert_eq!(snapshot.read::<u32>(depth), Ok(4));
    let rows = snapshot
        .window(RowWindow {
            node: load,
            start: 0,
            rows: 8,
            depth: 3,
        })
        .unwrap();
    let owned = mod_points::snapshot::Snapshot::new(&rows);
    assert!(
        owned.field(load, 0).is_none(),
        "an opaque leaf has no children"
    );
    assert!(snapshot.ty(load).is_some());
    match MODE.load(Ordering::Relaxed) {
        1 => edits.set(depth, &10u32).unwrap(),
        2 => edits.set(load, &0u32).unwrap(),
        _ => {}
    }
}

#[test]
fn callback_argument_is_an_opaque_leaf_the_native_body_still_calls() {
    let policy = Arc::new(EndpointPolicy::build(&catalog(), &[], &[HEAD_CALL]).unwrap());
    assert!(
        HEAD.set(Endpoint {
            listener: 0,
            policy,
            refused: &REFUSED,
            retained: Default::default(),
        })
        .is_ok()
    );
    mod_points::guest::register_raw(&mod_points::raw_listeners::game_test_environments_parse(
        "callback", head,
    ))
    .unwrap();
    mod_points::register(
        &SlotListener::snapshot(
            "callback-fixture",
            PointId::GameTestEnvironmentsParse,
            HEAD.get().unwrap(),
            0xC000_0000 + PointId::GameTestEnvironmentsParse as u32,
        )
        .unwrap(),
    )
    .unwrap();
    mod_points::__private::seal();
    let calls = AtomicUsize::new(0);
    let load = |name: &str| {
        calls.fetch_add(1, Ordering::Relaxed);
        (name == "minecraft:overworld").then_some(100)
    };
    assert_eq!(parse(4, &load), Some(104));
    // The scalar next to the closure is editable; the body still calls the real closure.
    MODE.store(1, Ordering::Relaxed);
    assert_eq!(parse(4, &load), Some(110));
    // An edit of the leaf refuses the whole patch: the scalar edit is not applied either.
    MODE.store(2, Ordering::Relaxed);
    REFUSED.store(true, Ordering::Relaxed);
    assert_eq!(parse(4, &load), Some(104));
    assert_eq!(
        (HEADS.load(Ordering::Relaxed), calls.load(Ordering::Relaxed)),
        (3, 3)
    );
}
