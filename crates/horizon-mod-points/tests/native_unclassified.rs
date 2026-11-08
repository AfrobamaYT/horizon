#![cfg(feature = "wasm-guest")]

//! A real catalog point with an `unclassified` argument, `Loading::menu_action(&mut self,
//! event_loop: &dyn EventLoopCtl, action: Action)`, on a stand-in receiver. The trait object is an
//! opaque leaf: the listener sees an empty typed header and no edit reaches it. The engine
//! object itself never leaves the call and stays usable by the native body.
mod common;
use common::Endpoint;
use mod_points::{__private::SlotListener, PointId, endpoint_policy::EndpointPolicy};
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use startup_menu::{Action, Loading};
pub mod startup_menu {
    #[derive(Clone, Copy)]
    pub enum Action {
        Quit,
        Options,
    }
    pub struct Loading {
        pub actions: u32,
    }
}
pub trait EventLoopCtl {
    fn exit(&self);
}
struct Ctl(AtomicUsize);
impl EventLoopCtl for Ctl {
    fn exit(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}
include!(concat!(env!("OUT_DIR"), "/native_raw_unclassified.rs"));

static MODE: AtomicUsize = AtomicUsize::new(0);
static HEADS: AtomicUsize = AtomicUsize::new(0);
static REFUSED: AtomicBool = AtomicBool::new(false);
static HEAD: OnceLock<Endpoint> = OnceLock::new();

fn head(snapshot: mod_points::snapshot::Snapshot<'_>, edits: &mut mod_api::wire::RecordPatch) {
    HEADS.fetch_add(1, Ordering::Relaxed);
    // Roots: receiver, event_loop, action, cancel.
    let ctl = snapshot.field(0, 1).unwrap();
    assert!(snapshot.field(0, 2).is_some() && snapshot.field(0, 3).is_some());
    assert!(
        snapshot.field(ctl, 0).is_none(),
        "an opaque leaf has no children"
    );
    assert!(snapshot.ty(ctl).is_some());
    if MODE.load(Ordering::Relaxed) == 1 {
        // A cancel edit next to the leaf, and an edit of the leaf itself.
        edits.set(snapshot.field(0, 3).unwrap(), &true).unwrap();
        edits.set(ctl, &0u32).unwrap();
    }
}

#[test]
fn unclassified_trait_object_argument_is_an_opaque_leaf() {
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
    mod_points::guest::register_raw(
        &mod_points::raw_listeners::game_startup_menu_loading_menu_action("opaque", head),
    )
    .unwrap();
    mod_points::register(
        &SlotListener::snapshot(
            "opaque-fixture",
            PointId::GameStartupMenuLoadingMenuAction,
            HEAD.get().unwrap(),
            0xC000_0000 + PointId::GameStartupMenuLoadingMenuAction as u32,
        )
        .unwrap(),
    )
    .unwrap();
    mod_points::__private::seal();
    let ctl = Ctl(AtomicUsize::new(0));
    let mut menu = Loading { actions: 0 };
    // The native body gets the real object, never a copy.
    menu.menu_action(&ctl, Action::Quit);
    menu.menu_action(&ctl, Action::Options);
    assert_eq!((menu.actions, ctl.0.load(Ordering::Relaxed)), (1, 1));
    // An edit of the leaf refuses the whole patch, so the cancel next to it is not applied.
    MODE.store(1, Ordering::Relaxed);
    REFUSED.store(true, Ordering::Relaxed);
    menu.menu_action(&ctl, Action::Quit);
    assert_eq!((menu.actions, ctl.0.load(Ordering::Relaxed)), (1, 2));
    assert_eq!(HEADS.load(Ordering::Relaxed), 3);
}
