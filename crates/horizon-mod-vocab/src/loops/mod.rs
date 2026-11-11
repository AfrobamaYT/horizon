//! Published loops (`[[native.loop]]`): hot loops the game owns, with a slot a native mod fills.
//!
//! A published loop is written once in the game as a function generic over its slot trait. With no
//! mod, the slot is `()` and the loop is the game's own. A native mod names a type of its crate
//! in a `[[native.loop]]` row; the launcher compiles one copy of the loop with the installed
//! mods' types in the slot, so their calls inline into it, and installs that copy through the
//! replacement cell of the loop's entry function. The game keeps the loop's order, what it
//! visits and where the slots sit; a mod answers through [`LoopFlow`] and the typed mutators of
//! the loop's view.
//!
//! Several mods on one loop nest by `(priority, mod id)`: `(A, (B, ()))`. The first answer that is
//! not [`LoopFlow::Continue`] wins, and later mods are not asked for that entry.
//!
//! A slot's state is one value per game instance, made with `Default` on its first run, kept
//! between ticks and shared by that instance's dimensions. A panic in a slot is the server's
//! crash, named for the mod; the slot is not run again.

use std::marker::PhantomData;

pub mod block_entity_tick;

/// What a slot answers for the entry the loop is about to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
// exhaustive: the loop matches every answer; a new one would change what every slot means
#[derive(horizon_mod_api_macros::Wire)]
pub enum LoopFlow {
    /// Run the entry, and ask the next mod's slot first.
    Continue,
    /// Leave this entry out of this run of the loop.
    Skip,
    /// End this run of the loop here; what the game does after the loop still runs.
    Stop,
}

/// One mod's row in a published loop, for the crash report: the launcher's build names each slot
/// type with one.
#[doc(hidden)]
pub trait LoopMod {
    /// The call a panic inside this mod's slot is reported as.
    const CALL: crate::content::crash::ModCall;
}

/// A mod's slot type `T`, each of whose calls is marked as inside the mod `N` names.
#[doc(hidden)]
// no wire: a compile-time wrapper of a mod's own slot state; it never crosses to wasm.
// exhaustive: built only by `Default`
pub struct Attributed<N, T> {
    slot: T,
    mod_row: PhantomData<fn() -> N>,
}

impl<N, T: Default> Default for Attributed<N, T> {
    fn default() -> Self {
        Self {
            slot: T::default(),
            mod_row: PhantomData,
        }
    }
}

impl<N: LoopMod, T> Attributed<N, T> {
    /// Runs `call` on the slot, marked as inside the mod.
    #[inline(always)]
    fn call<R>(&mut self, call: impl FnOnce(&mut T) -> R) -> R {
        let _inside = crate::content::crash::enter_native_mod(N::CALL);
        call(&mut self.slot)
    }
}
