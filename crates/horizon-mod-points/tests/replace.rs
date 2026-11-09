//! `replace` listeners (#2368) at real points, through `mod_points::replace!`, the expression the
//! launcher and `cargo horizon-mod build --wasm` both emit: the body never runs, the listener's
//! value reaches the caller, and a second replace at the same place is refused.
// Generated fixtures exercise every shape; TODO(#2273): make the generator emit clippy-clean code.
#![allow(clippy::all, unused_comparisons, unreachable_patterns, dead_code)]
use mod_points::{RegisterError, register};
use std::sync::atomic::{AtomicUsize, Ordering};

static SHUTDOWN_BODY: AtomicUsize = AtomicUsize::new(0);
static AVERAGE_BODY: AtomicUsize = AtomicUsize::new(0);
static REPLACED: AtomicUsize = AtomicUsize::new(0);

/// `horizon_client::game::server_lifecycle::remember_shutdown`, as the staging pass marks it: a
/// unit-returning point, so its record has `cancel`.
#[horizon_mod_api_macros::point(name = "game_server_lifecycle_remember_shutdown", free)]
fn remember_shutdown(failure: &mut Option<String>, message: String) {
    SHUTDOWN_BODY.fetch_add(1, Ordering::Relaxed);
    if failure.is_none() {
        *failure = Some(message);
    }
}

/// `horizon_client::game::blocks::beacon::average`, as the staging pass marks it: a point that
/// returns an API value, so its record has `result`.
#[horizon_mod_api_macros::point(name = "game_beacon_average", free)]
fn average(a: u32, b: u32) -> u32 {
    AVERAGE_BODY.fetch_add(1, Ordering::Relaxed);
    (a + b) / 2
}

/// Leaves `cancel` alone: the replace contract, not the listener, keeps the body from running.
fn own_shutdown(_: &mut mod_points::GameServerLifecycleRememberShutdownArgs) {
    REPLACED.fetch_add(1, Ordering::Relaxed);
}

/// Produces the function's value from its inputs.
fn own_average(args: &mut mod_points::GameBeaconAverageArgs) -> u32 {
    args.a.max(args.b) * 10
}

fn other_average(_: &mut mod_points::GameBeaconAverageArgs) -> u32 {
    0
}

/// An ordinary head listener: it may share the place with a replace.
fn watch_average(_: &mut mod_points::GameBeaconAverageArgs) {}

#[test]
fn a_replace_owns_the_function_and_only_one_may() {
    register(&mod_points::replace!(
        game_server_lifecycle_remember_shutdown,
        cancel,
        none,
        "owner",
        0,
        "own_shutdown",
        own_shutdown
    ))
    .unwrap();
    register(&mod_points::replace!(
        game_beacon_average,
        result,
        none,
        "owner",
        0,
        "own_average",
        own_average
    ))
    .unwrap();
    assert_eq!(
        register(&mod_points::replace!(
            game_beacon_average,
            result,
            none,
            "rival",
            5,
            "other_average",
            other_average
        )),
        Err(RegisterError::Replaced {
            first: ("owner", "own_average"),
            second: ("rival", "other_average"),
        })
    );
    assert_eq!(
        RegisterError::Replaced {
            first: ("owner", "own_average"),
            second: ("rival", "other_average"),
        }
        .to_string(),
        "rival:other_average replaces a function that owner:own_average already replaces; \
         only one replace listener can own it"
    );
    register(&mod_points::listeners::game_beacon_average(
        "watcher",
        0,
        "watch",
        watch_average,
    ))
    .unwrap();
    mod_points::__private::seal();

    let mut failure = None;
    remember_shutdown(&mut failure, "disk full".into());
    assert_eq!(REPLACED.load(Ordering::Relaxed), 1);
    assert_eq!(
        SHUTDOWN_BODY.load(Ordering::Relaxed),
        0,
        "the replaced body ran"
    );
    assert_eq!(failure, None);

    assert_eq!(average(3, 7), 70);
    assert_eq!(
        AVERAGE_BODY.load(Ordering::Relaxed),
        0,
        "the replaced body ran"
    );
}
