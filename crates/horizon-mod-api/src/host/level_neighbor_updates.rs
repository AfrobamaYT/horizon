//! The `level_neighbor_updates` area: host methods the game implements (`game/host/level_neighbor_updates.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' server writes of the `level_neighbor_updates` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait LevelNeighborUpdatesBackendMut {
    /// Side: server.
    /// Tell the neighbours of `pos` that a block changed there, as vanilla's own writes do:
    /// `Neighbours` (`Level.updateNeighborsAt`: a lever's or a button's own block, and the one
    /// it is attached to, when the signal it gives changes), `NeighboursExcept(direction)`
    /// (`updateNeighborsAtExceptFromFacing`: a diode's front) or `OutputSignal`
    /// (`updateNeighbourForOutputSignal`: the comparators beside it, when its comparator output
    /// changes without its state changing). `source` is the state whose block vanilla passes as
    /// the one that changed. False, with nothing done, off the level in place, for a position
    /// that isn't loaded, or for a `source` that is no block state.
    fn level_update_neighbors(
        &mut self,
        level: LevelRef,
        pos: [i32; 3],
        source: BlockStateId,
        update: NeighbourUpdate,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::level_update_neighbors",
            "level_update_neighbors",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Tell the block at `pos` that `source`'s block beside it changed (`Level.neighborChanged`),
    /// as a diode tells the block in front of it before updating that block's neighbours. False,
    /// with nothing done, as for `update_neighbors`: off the level in place, for a position that
    /// isn't loaded, or for a `source` that is no block state.
    fn level_neighbor_changed(
        &mut self,
        level: LevelRef,
        pos: [i32; 3],
        source: BlockStateId,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::level_neighbor_changed",
            "level_neighbor_changed",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
