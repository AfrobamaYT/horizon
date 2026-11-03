//! A mod crate's view of the mod API: `Hooks` is implemented with the names `mod_api` exports.
//! The test is that this file builds with the exported event types.
use horizon_client::mod_api::{Hooks, Host, InputKeyEvent, winit};

struct Keys;

impl Hooks for Keys {
    fn input_key(&self, _host: &mut dyn Host, event: &mut InputKeyEvent<'_>) {
        let _: &winit::event::KeyEvent = event.event;
    }
}

#[test]
fn a_mod_implements_the_key_hooks() {
    let _: Box<dyn Hooks> = Box::new(Keys);
}

#[cfg(feature = "testing")]
#[test]
fn scoped_mods_dispatch_and_restore_after_panic() {
    use horizon_client::mod_api::{TickEvent, dispatch_tick, mods, testing::with_mods};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct HostDouble;
    impl Host for HostDouble {}
    struct Counter(Arc<AtomicUsize>);
    impl Hooks for Counter {
        fn tick(&self, _: &mut dyn Host, _: &mut TickEvent) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    let outer = Arc::new(AtomicUsize::new(0));
    let inner = Arc::new(AtomicUsize::new(0));
    with_mods(
        vec![Box::new(Counter(outer.clone())) as Box<dyn Hooks>].into(),
        || {
            dispatch_tick(&mut HostDouble);
            let result = std::panic::catch_unwind(|| {
                with_mods(
                    vec![Box::new(Counter(inner.clone())) as Box<dyn Hooks>].into(),
                    || {
                        dispatch_tick(&mut HostDouble);
                        panic!("test unwind");
                    },
                );
            });
            assert!(result.is_err());
            dispatch_tick(&mut HostDouble);
        },
    );
    assert_eq!(outer.load(Ordering::Relaxed), 2);
    assert_eq!(inner.load(Ordering::Relaxed), 1);
    assert!(mods().is_empty());
}
