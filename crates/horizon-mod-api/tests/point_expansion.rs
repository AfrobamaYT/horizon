//! `#[point]` at run time: a method with the attribute, and the slots and records the generator
//! would write for it, declared here where the expansion looks for them (`crate::game::points`).
use horizon_mod_api_macros::point;
use mod_api::__private::point::Attach;
use mod_api::ServerHost;

struct Counter {
    total: u32,
}

impl mod_api::Host for Counter {}
impl ServerHost for Counter {}

mod game {
    pub mod points {
        use mod_api::__private::point::{Place, Point};
        use mod_api::ServerHost;

        pub struct AddArgs {
            pub step: u32,
            pub result: Option<u32>,
        }

        impl AddArgs {
            pub fn __new(step: u32) -> Self {
                Self { step, result: None }
            }
        }

        pub struct ResetArgs {
            pub cancel: bool,
        }

        impl ResetArgs {
            pub fn __new() -> Self {
                Self { cancel: false }
            }
        }

        type Listener<A> = fn(&mut dyn ServerHost, &mut A);
        pub(crate) static ADD: Point<Listener<AddArgs>> = Point::new("Counter::add", Place::Head);
        pub(crate) static ADD_RETURN: Point<Listener<AddArgs>> =
            Point::new("Counter::add", Place::Return);
        pub(crate) static RESET: Point<Listener<ResetArgs>> =
            Point::new("Counter::reset", Place::Head);
        pub(crate) static RESET_RETURN: Point<Listener<ResetArgs>> =
            Point::new("Counter::reset", Place::Return);
    }
}

impl Counter {
    #[point(side = "server")]
    fn add(&mut self, step: u32) -> u32 {
        self.total += step;
        self.total
    }

    #[point(side = "server")]
    fn reset(&mut self) {
        self.total = 0;
    }
}

type Add = fn(&mut dyn ServerHost, &mut game::points::AddArgs);
type Reset = fn(&mut dyn ServerHost, &mut game::points::ResetArgs);

#[test]
fn slots_change_arguments_stop_the_function_and_change_its_value() {
    let mut counter = Counter { total: 0 };
    // Empty slots: the function runs as written.
    assert_eq!(counter.add(1), 1);
    counter.reset();
    assert_eq!(counter.total, 0);

    // A head listener doubles the step, or answers 7 for a step of 100; a return listener adds
    // 1000 to what the function returns.
    let head: Add = |_, args| match args.step {
        100 => args.result = Some(7),
        _ => args.step *= 2,
    };
    let returned: Add = |_, args| args.result = args.result.map(|value| value + 1000);
    game::points::ADD.attach(Box::new(vec![head])).unwrap();
    game::points::ADD_RETURN
        .attach(Box::new(vec![returned]))
        .unwrap();
    // The doubled step reaches the body, and the return listener changes the value.
    assert_eq!(counter.add(3), 1006);
    assert_eq!(counter.total, 6);
    // A head result returns at once: no body, no return slot.
    assert_eq!(counter.add(100), 7);
    assert_eq!(counter.total, 6);

    // A head cancel skips the body.
    let cancel: Reset = |_, args| args.cancel = true;
    game::points::RESET.attach(Box::new(vec![cancel])).unwrap();
    counter.reset();
    assert_eq!(counter.total, 6);
    assert!(game::points::RESET_RETURN.get().is_none());
}
