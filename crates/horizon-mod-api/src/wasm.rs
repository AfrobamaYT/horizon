//! The mod's side of the WebAssembly interface, built only for `wasm32`: the generated
//! `Guest` reaches the game through the world's imports, and the generated exports call the
//! mod's [`Hooks`] and parts. A mod writes nothing for it: `#[mod_api::horizon_mod]` names
//! its hooks, and the same source builds native or to `mod.wasm`.
use crate::Hooks;
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};

/// A value from the game that the mod API can't hold (a stack that doesn't decode, a list
/// of the wrong length): the game broke the interface, so the call traps.
#[derive(Debug)]
pub(crate) struct Refused;

fn expect<T>(value: Result<T, Refused>) -> T {
    match value {
        Ok(value) => value,
        Err(Refused) => panic!("the game sent a value the mod API can't hold"),
    }
}

/// One call's storage for the values its borrowed arguments point into. The values may
/// borrow each other, so they are held type-erased and dropped newest first with the store.
#[derive(Default)]
pub(crate) struct Keep {
    held: RefCell<Vec<(*mut u8, unsafe fn(*mut u8))>>,
}

impl Keep {
    fn hold<T>(&self, value: T) -> *mut T {
        unsafe fn drop_box<T>(pointer: *mut u8) {
            // SAFETY: `pointer` came from `Box::into_raw` of a `Box<T>` in `hold`, once.
            drop(unsafe { Box::from_raw(pointer.cast::<T>()) });
        }
        let pointer = Box::into_raw(Box::new(value));
        self.held.borrow_mut().push((pointer.cast(), drop_box::<T>));
        pointer
    }

    pub(crate) fn str(&self, value: String) -> &str {
        // SAFETY: the box stays allocated, unmoved and handed out once, until `self` drops.
        unsafe { &*self.hold(value) }
    }

    pub(crate) fn slice<T>(&self, value: Vec<T>) -> &[T] {
        // SAFETY: as in `str`.
        unsafe { &*self.hold(value) }
    }

    pub(crate) fn val<T>(&self, value: T) -> &T {
        // SAFETY: as in `str`.
        unsafe { &*self.hold(value) }
    }

    #[allow(clippy::mut_from_ref)]
    pub(crate) fn val_mut<T>(&self, value: T) -> &mut T {
        // SAFETY: as in `str`.
        unsafe { &mut *self.hold(value) }
    }
}

impl Drop for Keep {
    fn drop(&mut self) {
        for (pointer, drop) in self.held.get_mut().drain(..).rev() {
            // SAFETY: each entry is a live box `hold` made, dropped once here.
            unsafe { drop(pointer) };
        }
    }
}

fn array<T, const N: usize>(items: Vec<T>) -> Result<[T; N], Refused> {
    items.try_into().map_err(|_| Refused)
}

/// The one `&'static str` for each distinct string: ids and dimensions the game names.
fn intern(value: String) -> &'static str {
    static INTERNED: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);
    let mut interned = INTERNED.lock().unwrap_or_else(|e| e.into_inner());
    let set = interned.get_or_insert_with(HashSet::new);
    match set.get(value.as_str()) {
        Some(existing) => existing,
        None => {
            let leaked: &'static str = Box::leak(value.into_boxed_str());
            set.insert(leaked);
            leaked
        }
    }
}

unsafe extern "Rust" {
    /// The mod's hooks, defined by `#[mod_api::horizon_mod]` in the mod's crate.
    safe fn __horizon_mod_hooks() -> Box<dyn Hooks>;
}

fn hooks() -> &'static dyn Hooks {
    static HOOKS: OnceLock<Box<dyn Hooks>> = OnceLock::new();
    &**HOOKS.get_or_init(__horizon_mod_hooks)
}

/// The parts and handlers the mod registered, by the index the game holds for each: every
/// trait the game may call it through (a block entity part is a block part too).
static HANDLERS: Mutex<Vec<Vec<Box<dyn Any + Send>>>> = Mutex::new(Vec::new());

/// Keeps a part or handler the mod hands the game, by its views, for the game's calls; gives
/// its index.
fn add_handler(views: Vec<Box<dyn Any + Send>>) -> u32 {
    let mut handlers = HANDLERS.lock().unwrap_or_else(|e| e.into_inner());
    handlers.push(views);
    (handlers.len() - 1) as u32
}

/// One view of a handler: it as the trait `T`. Handlers live as long as the mod.
struct Handler<T: ?Sized + 'static>(&'static T);
// SAFETY: wasm32 mods run on one thread.
unsafe impl<T: ?Sized> Send for Handler<T> {}

fn view<T: ?Sized + 'static>(handler: Arc<T>) -> Box<dyn Any + Send> {
    Box::new(Handler::<T>(&**Box::leak(Box::new(handler))))
}

fn handler<T: ?Sized + 'static>(index: u32) -> &'static T {
    let handlers = HANDLERS.lock().unwrap_or_else(|e| e.into_inner());
    match handlers
        .get(index as usize)
        .and_then(|views| views.iter().find_map(|v| v.downcast_ref::<Handler<T>>()))
    {
        Some(handler) => handler.0,
        None => panic!("the game called a handler the mod didn't register"),
    }
}

#[allow(
    clippy::all,
    unused_imports,
    unused_variables,
    unused_parens,
    dead_code,
    non_snake_case
)]
mod glue {
    use super::{array, expect, intern};
    include!(concat!(env!("OUT_DIR"), "/wasm_guest_generated.rs"));
}
