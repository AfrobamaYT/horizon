//! `#[point]` at run time: functions with the attribute, and the slot table, records and listener
//! lists the expansion finds in `mod_points`, declared here in place of the crate's. The raw
//! records and listener types are written out as the staging pass declares them beside each
//! function.
use horizon_mod_api_macros::point;
use mod_api::ServerHost;
use std::sync::atomic::Ordering;

pub struct Counter {
    total: u32,
}

impl mod_api::Host for Counter {}
impl ServerHost for Counter {}

/// A type off `Game`: its points have no host.
pub struct Pen {
    tags: Vec<String>,
}

mod mod_points {
    use mod_api::ServerHost;
    use std::sync::Mutex;
    use std::sync::atomic::AtomicBool;

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

    pub struct SlotArgs {}

    impl SlotArgs {
        pub fn __new() -> Self {
            Self {}
        }
    }

    pub struct PenLabelArgs<'a> {
        pub name: std::borrow::Cow<'a, str>,
    }

    impl<'a> PenLabelArgs<'a> {
        pub fn __new(name: &'a str) -> Self {
            Self {
                name: std::borrow::Cow::Borrowed(name),
            }
        }
    }

    pub struct DoubleArgs {
        pub x: i64,
        pub result: Option<i64>,
    }

    impl DoubleArgs {
        pub fn __new(x: i64) -> Self {
            Self { x, result: None }
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum PointId {
        Add,
        AddReturn,
        Reset,
        ResetReturn,
        Slot,
        SlotReturn,
        PenLabel,
        PenLabelReturn,
        Double,
        DoubleReturn,
    }

    /// The listener the test installs: its own `Points`.
    pub trait Points: Sync {
        fn add(&self, _: &mut dyn ServerHost, _: &mut AddArgs) {}
        fn add_return(&self, _: &mut dyn ServerHost, _: &mut AddArgs) {}
        fn reset(&self, _: &mut dyn ServerHost, _: &mut ResetArgs) {}
        fn reset_return(&self, _: &mut dyn ServerHost, _: &mut ResetArgs) {}
        fn slot(&self, _: &mut dyn ServerHost, _: &mut SlotArgs) {}
        fn pen_label(&self, _: &mut PenLabelArgs<'_>) {}
        fn pen_label_return(&self, _: &mut PenLabelArgs<'_>) {}
        fn double(&self, _: &mut DoubleArgs) {}
        fn double_return(&self, _: &mut DoubleArgs) {}
    }

    pub static LISTENER: Mutex<Option<&'static dyn Points>> = Mutex::new(None);
    pub static RAW: Mutex<Vec<(PointId, usize)>> = Mutex::new(Vec::new());
    pub static HEARD: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

    pub mod __private {
        use super::*;

        pub static POINTS: [AtomicBool; 10] = [const { AtomicBool::new(false) }; 10];

        pub fn fire(_: PointId, name: &'static str, call: &mut dyn FnMut(&'static dyn Points)) {
            HEARD.lock().unwrap().push(name);
            let listener = *LISTENER.lock().unwrap();
            listener.into_iter().for_each(call);
        }

        pub fn fire_raw(place: PointId, name: &'static str, call: &mut dyn FnMut(*const ())) {
            let functions: Vec<_> = RAW
                .lock()
                .unwrap()
                .iter()
                .filter(|(at, _)| *at == place)
                .map(|(_, function)| *function)
                .collect();
            if !functions.is_empty() {
                HEARD.lock().unwrap().push(name);
            }
            for function in functions {
                call(function as *const ());
            }
        }
    }
}

impl Counter {
    #[point(side = "server", raw)]
    fn add(&mut self, step: u32) -> u32 {
        self.total += step;
        self.total
    }

    #[point(side = "server")]
    fn reset(&mut self) {
        self.total = 0;
    }

    /// Its value borrows the counter: a head, and a raw-only return that gets the value.
    #[point(side = "server", raw)]
    fn slot(&mut self) -> &mut u32 {
        &mut self.total
    }
}

// What the staging pass declares after `impl Counter`.
pub struct __AddRaw<'r, T0, R> {
    pub step: &'r mut T0,
    pub result: &'r mut Option<R>,
}
pub type __AddListener = for<'r> fn(&'r mut Counter, &'r mut __AddRaw<'r, u32, u32>);
pub struct __SlotRaw {}
pub type __AddReturnListener = for<'r> fn(&'r mut Counter, &'r mut u32);
pub type __SlotListener = for<'r> fn(&'r mut Counter, &'r mut __SlotRaw);
pub type __SlotReturnListener = for<'r> fn(&'r mut &mut u32);

impl Pen {
    #[point(name = "pen_label", raw)]
    fn label(&self, name: &str, mut tags: Vec<String>) -> Vec<String> {
        tags.push(name.to_owned());
        tags.extend(self.tags.iter().cloned());
        tags
    }
}

// What the staging pass declares after `impl Pen`.
pub struct __PenLabelRaw<'r, T0, T1, R> {
    pub name: &'r mut T0,
    pub tags: &'r mut T1,
    pub result: &'r mut Option<R>,
}
pub type __PenLabelListener = for<'r> fn(
    &'r Pen,
    &'r mut __PenLabelRaw<'r, std::borrow::Cow<'_, str>, Vec<String>, Vec<String>>,
);
pub type __PenLabelReturnListener = for<'r> fn(&'r Pen, &'r mut Vec<String>);

#[point(name = "double", raw, free)]
fn double(x: i64) -> i64 {
    x * 2
}

// What the staging pass declares after `fn double`.
pub struct __DoubleRaw<'r, T0, R> {
    pub x: &'r mut T0,
    pub result: &'r mut Option<R>,
}
pub type __DoubleListener = for<'r> fn(&'r mut __DoubleRaw<'r, i64, i64>);
pub type __DoubleReturnListener = for<'r> fn(&'r mut i64);

struct Listener;

impl mod_points::Points for Listener {
    // Doubles the step, or answers 7 for a step of 100.
    fn add(&self, _: &mut dyn ServerHost, args: &mut mod_points::AddArgs) {
        match args.step {
            100 => args.result = Some(7),
            _ => args.step *= 2,
        }
    }
    fn add_return(&self, _: &mut dyn ServerHost, args: &mut mod_points::AddArgs) {
        args.result = args.result.map(|value| value + 1000);
    }
    fn reset(&self, _: &mut dyn ServerHost, args: &mut mod_points::ResetArgs) {
        args.cancel = true;
    }
    fn pen_label(&self, args: &mut mod_points::PenLabelArgs<'_>) {
        args.name = format!("{}!", args.name).into();
    }
}

/// A raw listener after the API one: it sees the API's change and reads the receiver.
fn raw_add(counter: &mut Counter, raw: &mut __AddRaw<'_, u32, u32>) {
    *raw.step += counter.total;
}

/// The borrowed value itself: the listener writes through it.
fn raw_slot_return(slot: &mut &mut u32) {
    **slot += 50;
}

/// An internal-typed argument changed at the head.
fn raw_label(
    pen: &Pen,
    raw: &mut __PenLabelRaw<'_, std::borrow::Cow<'_, str>, Vec<String>, Vec<String>>,
) {
    raw.tags.push(format!("{} tags", pen.tags.len()));
}

/// A value the API can't carry, changed at the return.
fn raw_label_return(_: &Pen, value: &mut Vec<String>) {
    value.push("returned".into());
}

/// A free function's head: answers at once for a negative argument.
fn raw_double(raw: &mut __DoubleRaw<'_, i64, i64>) {
    if *raw.x < 0 {
        *raw.result = Some(0);
    }
}

#[test]
fn slots_change_arguments_stop_the_function_and_change_its_value() {
    use mod_points::__private::POINTS;
    use mod_points::{HEARD, LISTENER, PointId, RAW};
    let mut counter = Counter { total: 0 };
    let pen = Pen {
        tags: vec!["own".into()],
    };
    // Empty slots: the function runs as written, and no listener list is asked for.
    assert_eq!(counter.add(1), 1);
    counter.reset();
    *counter.slot() = 0;
    assert_eq!(pen.label("a", Vec::new()), ["a", "own"]);
    assert_eq!(double(4), 8);
    assert!(HEARD.lock().unwrap().is_empty());

    *LISTENER.lock().unwrap() = Some(&Listener);
    // As the generated `main` registers raw listeners: a typed `let`, then erased.
    let add: __AddListener = raw_add;
    let slot: __SlotReturnListener = raw_slot_return;
    let label: __PenLabelListener = raw_label;
    let label_return: __PenLabelReturnListener = raw_label_return;
    let head: __DoubleListener = raw_double;
    RAW.lock().unwrap().extend([
        (PointId::Add, add as usize),
        (PointId::SlotReturn, slot as usize),
        (PointId::PenLabel, label as usize),
        (PointId::PenLabelReturn, label_return as usize),
        (PointId::Double, head as usize),
    ]);
    for id in [
        PointId::Add,
        PointId::AddReturn,
        PointId::Reset,
        PointId::Slot,
        PointId::SlotReturn,
        PointId::PenLabel,
        PointId::PenLabelReturn,
        PointId::Double,
    ] {
        POINTS[id as usize].store(true, Ordering::Release);
    }
    // The API listener doubles the step to 6, the raw one adds the total (0): the body adds 6,
    // and the API return listener changes the value.
    assert_eq!(counter.add(3), 1006);
    assert_eq!(counter.total, 6);
    // A head result returns at once: no body, no return place.
    assert_eq!(counter.add(100), 7);
    assert_eq!(counter.total, 6);
    // A head cancel skips the body; its unset return slot isn't asked.
    counter.reset();
    assert_eq!(counter.total, 6);
    // The borrowed return reaches its raw listener as itself.
    *counter.slot() += 1;
    assert_eq!(counter.total, 57);
    // Off `Game`: the API listener changes the text, the raw one an internal argument at the
    // head and the value the API can't carry at the return.
    assert_eq!(
        pen.label("b", Vec::new()),
        ["1 tags", "b!", "own", "returned"]
    );
    // A free function: a raw head result skips the body.
    assert_eq!(double(-5), 0);
    assert_eq!(double(5), 10);
    assert_eq!(
        *HEARD.lock().unwrap(),
        [
            "add",
            "add",
            "add_return",
            "add",
            "add",
            "reset",
            "slot",
            "slot_return",
            "pen_label",
            "pen_label",
            "pen_label_return",
            "pen_label_return",
            "double",
            "double",
            "double",
            "double",
        ]
    );
}
