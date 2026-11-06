//! The `level_signals` area: host methods the game implements (`game/host/level_signals.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `level_signals` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait LevelSignalsBackend {
    /// Side: server.
    /// How strongly the block at `pos` powers the block on its `direction` side, power it
    /// conducts included (`SignalGetter.getSignal(pos, direction)`, as a block reads its
    /// neighbour `pos` that lies `direction` from it), 0 to 15. None for another level, a stale
    /// handle or a client. Takes a shared host and opens no transaction.
    fn level_signal(
        &self,
        level: LevelRef,
        pos: [i32; 3],
        direction: Direction,
    ) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::level_signal",
            "level_signal",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The strong power alone that the block at `pos` gives that way
    /// (`SignalGetter.getDirectSignal(pos, direction)`), 0 to 15. None as for `signal`.
    fn level_direct_signal(
        &self,
        level: LevelRef,
        pos: [i32; 3],
        direction: Direction,
    ) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::level_direct_signal",
            "level_direct_signal",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The strongest strong power reaching the block at `pos` from its six neighbours
    /// (`SignalGetter.getDirectSignalTo`), 0 to 15. None as for `signal`.
    fn level_direct_signal_to(&self, level: LevelRef, pos: [i32; 3]) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::level_direct_signal_to",
            "level_direct_signal_to",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The strongest power reaching the block at `pos` from its six neighbours
    /// (`SignalGetter.getBestNeighborSignal`, what a dispenser or a piston reads), 0 to 15.
    /// None as for `signal`.
    fn level_best_neighbor_signal(&self, level: LevelRef, pos: [i32; 3]) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::level_best_neighbor_signal",
            "level_best_neighbor_signal",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether any neighbour powers the block at `pos` (`SignalGetter.hasNeighborSignal`),
    /// asking no side past the first that does. False where `signal` is None.
    fn level_has_neighbor_signal(&self, level: LevelRef, pos: [i32; 3]) -> bool {
        crate::phase::unprovided(
            "Backend::level_has_neighbor_signal",
            "level_has_neighbor_signal",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
