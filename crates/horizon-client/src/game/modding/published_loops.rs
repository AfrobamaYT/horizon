//! Published loops (`[[native.loop]]`): hot loops the game owns, with a slot a native mod's helper
//! fills. Like every `[[native.*]]` table this is game code by another route, pinned to the exact
//! build: a loop's slot trait and cursor live beside the loop (`block_entity_ticks.rs` for
//! `block_entities.tick`) and lend the slot the loop's own state, and nothing here is `mod_api`.
//!
//! A loop is written once as a function generic over its slot; its entry, marked
//! `#[horizon_loop]`, calls it with `()`, which compiles to the game's own loop. A mod build compiles
//! one copy of the entry that runs the loop with the installed slots, nested by `(priority, mod
//! id)` (`(A, (B, ()))`: the first answer that is not [`LoopFlow::Continue`] wins), and installs it
//! through the entry's replacement cell. Their state is one value per `Game` (this file), so a
//! single-player process's client and server games never share one.
#![allow(dead_code, reason = "a mod build's copy of a loop entry uses what the game's own loop does not")]
use super::Game;
use horizon_client::Result;
use std::any::Any;
use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

/// What a slot answers for the entry the loop is about to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LoopFlow {
    /// Run the entry, and ask the next mod's slot first.
    Continue,
    /// Leave this entry out of this run of the loop.
    Skip,
    /// End this run of the loop here; what the game does after the loop still runs.
    Stop,
}

/// One mod's row in a published loop, for the crash report: a mod build names each slot with one.
pub(crate) trait LoopMod {
    /// The call a panic inside this mod's slot is reported as.
    const CALL: mod_api::content::crash::ModCall;
}

/// A mod's slot `T`, each of whose calls is marked as inside the mod `N` names.
pub(crate) struct Attributed<N, T> {
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
    pub(crate) fn call<R>(&mut self, call: impl FnOnce(&mut T) -> R) -> R {
        let _inside = mod_api::content::crash::enter_native_mod(N::CALL);
        call(&mut self.slot)
    }
}

/// One published loop's slot state.
enum Cell {
    /// Between runs.
    Ready(Box<dyn Any>),
    /// Out for a run of the loop.
    Taken,
    /// A slot panicked: the state was dropped and the loop runs without its slots.
    Poisoned,
}

/// The cells of the loops this game has run with slots, by loop name, in first-run order.
#[derive(Default)]
pub(super) struct LoopStates {
    cells: Vec<LoopEntry>,
}

struct LoopEntry {
    name: &'static str,
    cell: Cell,
    /// Whether the run without slots was logged, for a nested run or after a panic.
    logged: bool,
}

impl LoopStates {
    fn entry(&mut self, name: &'static str) -> &mut LoopEntry {
        let index = match self.cells.iter().position(|entry| entry.name == name) {
            Some(index) => index,
            None => {
                self.cells.push(LoopEntry {
                    name,
                    cell: Cell::Ready(Box::new(())),
                    logged: false,
                });
                self.cells.len() - 1
            }
        };
        &mut self.cells[index]
    }

    /// The loop's state for a run, made with `Default` on the first; `None` while a run has it
    /// out or after a panic, logged once.
    fn take<T: Default + 'static>(&mut self, name: &'static str) -> Option<Box<T>> {
        let entry = self.entry(name);
        match std::mem::replace(&mut entry.cell, Cell::Taken) {
            Cell::Ready(state) => Some(state.downcast().unwrap_or_default()),
            other => {
                if !entry.logged {
                    entry.logged = true;
                    log::warn!(
                        "Published loop {name}: {}; it runs without its mods' slots",
                        match other {
                            Cell::Poisoned => "a mod's slot panicked",
                            _ => "it was entered again while it ran",
                        }
                    );
                }
                entry.cell = other;
                None
            }
        }
    }

    fn put(&mut self, name: &'static str, cell: Cell) {
        self.entry(name).cell = cell;
    }
}

impl Game<'_> {
    /// Runs the published loop `name` with the slots `T`: `run` with this game's state for it, or
    /// `vanilla` (the loop with no slots) while the state is out for an enclosing run or a slot
    /// has panicked. A panic in `run` drops the state, marks the loop poisoned and goes on
    /// unwinding to the crash path, which names the mod.
    pub(crate) fn run_published_loop<T: Default + 'static>(
        &mut self,
        name: &'static str,
        run: impl FnOnce(&mut Self, &mut T) -> Result<()>,
        vanilla: impl FnOnce(&mut Self) -> Result<()>,
    ) -> Result<()> {
        let Some(mut state) = self.loop_states.take::<T>(name) else {
            return vanilla(self);
        };
        match catch_unwind(AssertUnwindSafe(|| run(self, &mut state))) {
            Ok(result) => {
                self.loop_states.put(name, Cell::Ready(state));
                result
            }
            Err(panic) => {
                drop(state);
                self.loop_states.put(name, Cell::Poisoned);
                resume_unwind(panic)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Count(u32);

    #[test]
    fn the_state_is_kept_between_runs_and_made_once() {
        let mut states = LoopStates::default();
        let mut first = states.take::<Count>("l").unwrap();
        first.0 = 7;
        states.put("l", Cell::Ready(first));
        assert_eq!(states.take::<Count>("l").unwrap().0, 7);
    }

    #[test]
    fn a_nested_run_gets_no_state_and_logs_once() {
        let mut states = LoopStates::default();
        let outer = states.take::<Count>("l").unwrap();
        assert!(states.take::<Count>("l").is_none());
        assert!(states.entry("l").logged);
        assert!(states.take::<Count>("l").is_none());
        // The outer run puts it back; the next run has it.
        states.put("l", Cell::Ready(outer));
        assert!(states.take::<Count>("l").is_some());
    }

    #[test]
    fn a_poisoned_loop_stays_without_state() {
        let mut states = LoopStates::default();
        drop(states.take::<Count>("l"));
        states.put("l", Cell::Poisoned);
        assert!(states.take::<Count>("l").is_none());
        assert!(states.take::<Count>("l").is_none());
        assert!(matches!(states.entry("l").cell, Cell::Poisoned));
        // Another loop is not touched.
        assert!(states.take::<Count>("other").is_some());
    }
}
