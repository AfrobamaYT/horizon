//! The registry's lifecycle, registering, sealing and stopping, on the real points: an integration
//! test, so the generated points compile once for the tests.
use mod_points::__private::{POINTS, register_raw, seal};
use mod_points::{PointId, Points, RegisterError, register, stop};
use std::sync::atomic::Ordering;

struct Tick;

impl Points for Tick {
    fn subscribed(&self) -> &'static [Option<PointId>] {
        &[mod_points::ids::tick, mod_points::ids::tick_return]
    }
}

struct Quiet;

impl Points for Quiet {
    fn subscribed(&self) -> &'static [Option<PointId>] {
        &[]
    }
}

fn raw_listener() {}

/// Stops its own mod, and tries to register another, from inside the seal.
struct SelfStop;

impl Points for SelfStop {
    fn subscribed(&self) -> &'static [Option<PointId>] {
        stop("self_stopping");
        assert_eq!(register("during", &Quiet), Err(RegisterError::Sealed));
        &[mod_points::ids::tick]
    }
}

#[test]
fn one_list_registered_sealed_and_stopped() {
    assert_eq!(register("tick_mod", &Tick), Ok(()));
    assert_eq!(register("tick_mod", &Quiet), Err(RegisterError::Twice));
    assert_eq!(register("quiet_mod", &Quiet), Ok(()));
    // A stop before the seal holds after it.
    assert_eq!(register("stopped_early", &Tick), Ok(()));
    stop("stopped_early");
    assert_eq!(register("self_stopping", &SelfStop), Ok(()));
    let raw = PointId::Tick as usize + 2;
    let place = PointId::Tick;
    assert_eq!(
        register_raw("raw_mod", place, raw_listener as *const ()),
        Ok(())
    );
    assert!(
        !POINTS[PointId::Tick as usize].load(Ordering::Acquire),
        "nothing listens before the seal"
    );
    seal();
    assert_eq!(register("late", &Quiet), Err(RegisterError::Sealed));
    assert_eq!(
        register_raw("late", place, raw_listener as *const ()),
        Err(RegisterError::Sealed)
    );
    assert!(POINTS[PointId::Tick as usize].load(Ordering::Acquire));
    assert!(POINTS[PointId::TickReturn as usize].load(Ordering::Acquire));
    assert!(!POINTS[raw].load(Ordering::Acquire));
    let (mut heard, mut raw_heard) = (0, 0);
    mod_points::__private::fire(PointId::Tick, "tick", &mut |_| heard += 1);
    mod_points::__private::fire_raw(PointId::Tick, "tick", &mut |function| {
        assert_eq!(function, raw_listener as *const ());
        raw_heard += 1;
    });
    assert_eq!((heard, raw_heard), (1, 1));
    stop("tick_mod");
    stop("raw_mod");
    mod_points::__private::fire(PointId::Tick, "tick", &mut |_| heard += 1);
    mod_points::__private::fire_raw(PointId::Tick, "tick", &mut |_| raw_heard += 1);
    assert_eq!((heard, raw_heard), (1, 1), "stopped mods are skipped");
}

#[test]
fn lists_finds_a_name_at_compile_time() {
    use mod_points::__private::lists;
    const LIST: &[&str] = &["tick", "tick_return"];
    const _: () = assert!(lists(LIST, "tick_return"));
    assert!(!lists(LIST, "tic") && !lists(LIST, "ticks") && !lists(&[], "tick"));
}
