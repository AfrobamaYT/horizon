//! The state of the `[[native.loop]]` slots this game runs (`mod_api::loops`): one cell per
//! published loop, owned by the `Game`, so a single-player process's client and server games never
//! share one. The game never names a mod type: a mod build's copy of the loop's entry calls
//! [`Game::run_published_loop`] with its slots' type, and the state lives here between runs.
use super::Game;
use horizon_client::Result;
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

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
    #[allow(dead_code, reason = "only a mod build's copy of a loop entry calls it")]
    pub fn run_published_loop<T: Default + 'static>(
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
