//! The one list of point listeners. A native mod's `#[horizon_mod] impl Points` and the wasm
//! runtime's proxies both [`register`] here while the game loads; the game then [`seal`]s it and
//! sets its slot table from [`listened_at`]. A firing slot walks [`listeners`]: the mods
//! subscribed to that place, in registration order (dispatch order), skipping any [`stop`]ped.
use crate::{POINT_COUNT, PointId, Points};
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

struct Listener {
    id: &'static str,
    points: &'static dyn Points,
    /// Set when the mod stopped (a trap in its wasm group, or the game giving up on it): its
    /// listeners are skipped from then on.
    stopped: AtomicBool,
}

struct Sealed {
    listeners: Box<[Listener]>,
    /// Each place's subscribed listeners, by index into `listeners`, in order.
    places: Box<[Box<[u32]>]>,
}

static OPEN: Mutex<Vec<Listener>> = Mutex::new(Vec::new());
static SEALED: OnceLock<Sealed> = OnceLock::new();

/// Why a listener wasn't registered.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RegisterError {
    /// The list was sealed: listeners register while the game loads, before its points are set.
    Sealed,
    /// The mod already registered its listeners.
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

/// Registers mod `id`'s listeners, `points`, at the places its [`Points::subscribed`] names. One
/// route for both callers: a native mod's generated export and the wasm runtime's proxy.
pub fn register(id: &'static str, points: &'static dyn Points) -> Result<(), RegisterError> {
    if SEALED.get().is_some() {
        return Err(RegisterError::Sealed);
    }
    let mut open = OPEN.lock().unwrap_or_else(|e| e.into_inner());
    if open.iter().any(|listener| listener.id == id) {
        return Err(RegisterError::Twice);
    }
    open.push(Listener {
        id,
        points,
        stopped: AtomicBool::new(false),
    });
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

/// Seals the list: no listener registers after it. The game calls it once, after the mods load
/// and before it reads [`listened_at`].
pub fn seal() {
    SEALED.get_or_init(|| {
        let listeners: Box<[Listener]> =
            std::mem::take(&mut *OPEN.lock().unwrap_or_else(|e| e.into_inner())).into();
        let mut places = vec![Vec::new(); POINT_COUNT];
        for (index, listener) in listeners.iter().enumerate() {
            for place in listener.points.subscribed().iter().flatten() {
                places[*place as usize].push(index as u32);
            }
        }
        Sealed {
            listeners,
            places: places.into_iter().map(Vec::into_boxed_slice).collect(),
        }
    });
}

/// Whether a registered mod listens at the place with this `PointId` index.
pub fn listened_at(index: usize) -> bool {
    SEALED.get().is_some_and(|sealed| {
        sealed
            .places
            .get(index)
            .is_some_and(|place| !place.is_empty())
    })
}

/// The listeners at `place`, in dispatch order, each marked as running `name` while it runs so a
/// panic in it names the mod; stopped mods are skipped.
pub fn listeners(place: PointId, name: &'static str) -> impl Iterator<Item = &'static dyn Points> {
    let sealed = SEALED.get();
    let indices = sealed.map_or(&[][..], |sealed| &sealed.places[place as usize][..]);
    let mut guard = None;
    indices.iter().filter_map(move |&index| {
        drop(guard.take());
        let listener = &sealed?.listeners[index as usize];
        if listener.stopped.load(Ordering::Acquire) {
            return None;
        }
        guard = Some(mod_api::__private::enter(listener.id, name));
        Some(listener.points)
    })
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

    #[test]
    fn one_list_registered_sealed_and_stopped() {
        assert_eq!(register("tick_mod", &Tick), Ok(()));
        assert_eq!(register("tick_mod", &Quiet), Err(RegisterError::Twice));
        assert_eq!(register("quiet_mod", &Quiet), Ok(()));
        assert!(
            !listened_at(PointId::Tick as usize),
            "nothing listens before the seal"
        );
        seal();
        assert_eq!(register("late", &Quiet), Err(RegisterError::Sealed));
        assert!(listened_at(PointId::Tick as usize) && listened_at(PointId::TickReturn as usize));
        assert!(!listened_at(PointId::Tick as usize + 2));
        assert_eq!(listeners(PointId::Tick, "tick").count(), 1);
        stop("tick_mod");
        assert_eq!(listeners(PointId::Tick, "tick").count(), 0);
    }
}
