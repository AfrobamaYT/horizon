//! The one list of point listeners, and the slot table the points read. A native mod's
//! `#[horizon_mod] impl Points` and the wasm runtime's proxies both [`register`] here while the game
//! loads, and a native mod's raw listeners [`register_raw`]; the game then [`seal`]s the list, which
//! sets [`POINTS`]. A firing slot walks the mods subscribed to that place, API listeners first and
//! then raw ones, each in registration (dispatch) order, skipping any [`stop`]ped mod.
use crate::{POINT_COUNT, PointId, Points};
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// Each place's slot: set once, at the seal, where a mod listens. A point that nothing listens at
/// costs one load of it.
pub static POINTS: [AtomicBool; POINT_COUNT] = [const { AtomicBool::new(false) }; POINT_COUNT];

struct Listener {
    id: &'static str,
    /// The mod's `impl Points`, when it has one.
    points: Option<&'static dyn Points>,
    /// Set when the mod stopped (a trap in its wasm group, or the game giving up on it): its
    /// listeners are skipped from then on.
    stopped: AtomicBool,
}

/// A raw listener: a plain function of the mod's, whose type the generated `main` checked against
/// the place's listener type before erasing it.
#[derive(Clone, Copy)]
struct Raw {
    listener: u32,
    function: *const (),
}

// A function pointer: no data behind it, so it moves between threads.
unsafe impl Send for Raw {}
unsafe impl Sync for Raw {}

#[derive(Default)]
struct Open {
    listeners: Vec<Listener>,
    raw: Vec<(PointId, Raw)>,
}

struct Sealed {
    listeners: Box<[Listener]>,
    /// Each place's subscribed `impl Points`, by index into `listeners`, in order.
    places: Box<[Box<[u32]>]>,
    /// Each place's raw listeners, in order.
    raw: Box<[Box<[Raw]>]>,
}

static OPEN: Mutex<Open> = Mutex::new(Open {
    listeners: Vec::new(),
    raw: Vec::new(),
});
static SEALED: OnceLock<Sealed> = OnceLock::new();

/// Why a listener wasn't registered.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RegisterError {
    /// The list was sealed: listeners register while the game loads, before its points are set.
    Sealed,
    /// The mod already registered its `impl Points`.
    Twice,
}

impl std::fmt::Display for RegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Sealed => "the game's points were already set",
            Self::Twice => "the mod already listens at points",
        })
    }
}

impl std::error::Error for RegisterError {}

/// The open list, while listeners may still register.
fn open() -> Result<std::sync::MutexGuard<'static, Open>, RegisterError> {
    if SEALED.get().is_some() {
        return Err(RegisterError::Sealed);
    }
    Ok(OPEN.lock().unwrap_or_else(|e| e.into_inner()))
}

/// Mod `id`'s entry in the open list, added on its first registration.
fn listener(open: &mut Open, id: &'static str) -> usize {
    match open.listeners.iter().position(|listener| listener.id == id) {
        Some(index) => index,
        None => {
            open.listeners.push(Listener {
                id,
                points: None,
                stopped: AtomicBool::new(false),
            });
            open.listeners.len() - 1
        }
    }
}

/// Registers mod `id`'s listeners, `points`, at the places its [`Points::subscribed`] names. One
/// route for both callers: a native mod's generated export and the wasm runtime's proxy.
pub fn register(id: &'static str, points: &'static dyn Points) -> Result<(), RegisterError> {
    let mut open = open()?;
    let index = listener(&mut open, id);
    let entry = &mut open.listeners[index].points;
    if entry.is_some() {
        return Err(RegisterError::Twice);
    }
    *entry = Some(points);
    Ok(())
}

/// Registers one of mod `id`'s raw listeners at `place`: a function of the place's listener type,
/// erased. Only the launcher's generated `main` calls it, after a typed `let` checked the type; the
/// place's slot casts it back to that type.
pub fn register_raw(
    id: &'static str,
    place: PointId,
    function: *const (),
) -> Result<(), RegisterError> {
    let mut open = open()?;
    let listener = listener(&mut open, id) as u32;
    open.raw.push((place, Raw { listener, function }));
    Ok(())
}

/// Stops mod `id`'s listeners: none of them runs again. A trap that stops a wasm group and the
/// game giving up on a native mod both end here.
pub fn stop(id: &str) {
    if let Some(sealed) = SEALED.get() {
        for listener in sealed.listeners.iter().filter(|listener| listener.id == id) {
            listener.stopped.store(true, Ordering::Release);
        }
    }
}

/// Seals the list, so no listener registers after it, and sets each slot a mod listens at. The game
/// calls it once, after the mods load.
pub fn seal() {
    let sealed = SEALED.get_or_init(|| {
        let open = std::mem::take(&mut *OPEN.lock().unwrap_or_else(|e| e.into_inner()));
        let mut places = vec![Vec::new(); POINT_COUNT];
        for (index, listener) in open.listeners.iter().enumerate() {
            for place in listener
                .points
                .iter()
                .flat_map(|points| points.subscribed())
                .flatten()
            {
                places[*place as usize].push(index as u32);
            }
        }
        let mut raw = vec![Vec::new(); POINT_COUNT];
        for (place, function) in open.raw {
            raw[place as usize].push(function);
        }
        Sealed {
            listeners: open.listeners.into(),
            places: places.into_iter().map(Vec::into_boxed_slice).collect(),
            raw: raw.into_iter().map(Vec::into_boxed_slice).collect(),
        }
    });
    for (index, slot) in POINTS.iter().enumerate() {
        let listened = !sealed.places[index].is_empty() || !sealed.raw[index].is_empty();
        slot.store(listened, Ordering::Release);
    }
}

/// Runs `call` for the mod `listener` while it runs `name`, so a panic in it names the mod; a
/// stopped mod is skipped.
fn run(sealed: &Sealed, listener: u32, name: &'static str, call: impl FnOnce()) {
    let listener = &sealed.listeners[listener as usize];
    if !listener.stopped.load(Ordering::Acquire) {
        let _guard = mod_api::__private::enter(listener.id, name);
        call();
    }
}

/// Runs the `impl Points` listeners at `place` (the `Points` method `name`), each through `call`.
pub(crate) fn fire(place: PointId, name: &'static str, call: &mut dyn FnMut(&'static dyn Points)) {
    let Some(sealed) = SEALED.get() else { return };
    for &index in &sealed.places[place as usize] {
        let points = sealed.listeners[index as usize].points;
        run(sealed, index, name, || {
            call(points.expect("a subscribed mod has its Points"))
        });
    }
}

/// Runs the raw listeners at `place`, each through `call` with its erased function.
pub(crate) fn fire_raw(place: PointId, name: &'static str, call: &mut dyn FnMut(*const ())) {
    let Some(sealed) = SEALED.get() else { return };
    for raw in &sealed.raw[place as usize] {
        run(sealed, raw.listener, name, || call(raw.function));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tick;

    impl Points for Tick {
        fn subscribed(&self) -> &'static [Option<PointId>] {
            &[crate::ids::tick, crate::ids::tick_return]
        }
    }

    struct Quiet;

    impl Points for Quiet {
        fn subscribed(&self) -> &'static [Option<PointId>] {
            &[]
        }
    }

    fn raw_listener() {}

    #[test]
    fn one_list_registered_sealed_and_stopped() {
        assert_eq!(register("tick_mod", &Tick), Ok(()));
        assert_eq!(register("tick_mod", &Quiet), Err(RegisterError::Twice));
        assert_eq!(register("quiet_mod", &Quiet), Ok(()));
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
        fire(PointId::Tick, "tick", &mut |_| heard += 1);
        fire_raw(PointId::Tick, "tick", &mut |function| {
            assert_eq!(function, raw_listener as *const ());
            raw_heard += 1;
        });
        assert_eq!((heard, raw_heard), (1, 1));
        stop("tick_mod");
        stop("raw_mod");
        fire(PointId::Tick, "tick", &mut |_| heard += 1);
        fire_raw(PointId::Tick, "tick", &mut |_| raw_heard += 1);
        assert_eq!((heard, raw_heard), (1, 1), "stopped mods are skipped");
    }
}
