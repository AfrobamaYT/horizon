//! The `fluid_capability` area: host methods the game implements (`game/host/fluid_capability.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `fluid_capability` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait FluidCapabilityBackend {
    /// Side: server.
    /// How many tanks of the block at `at` in the level in place its side reaches
    /// (none: every tank), for the game's own fluid capability: a mod block's by its table, a
    /// cauldron's one. None where no tanks are.
    fn fluid_tank_count(&self, at: BlockSide) -> Option<usize> {
        crate::phase::unprovided(
            "Backend::fluid_tank_count",
            "fluid_tank_count",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// What tank `tank` of those `at`'s side reaches holds (`native_fluid_tank_count`).
    fn fluid_tank(&self, at: BlockSide, tank: usize) -> Option<FluidStack> {
        crate::phase::unprovided(
            "Backend::fluid_tank",
            "fluid_tank",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// How much of `fluid` tank `tank` of those `at`'s side reaches holds when empty: a mod
    /// block's fillable tank its capacity, a cauldron a bucket; none for another tank, a
    /// drain-only one, or a fluid the tanks there don't take: other than water and lava, or
    /// with components into a mod block's tanks.
    fn fluid_tank_capacity(&self, at: BlockSide, tank: usize, fluid: &FluidStack) -> u32 {
        crate::phase::unprovided(
            "Backend::fluid_tank_capacity",
            "fluid_tank_capacity",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `fluid_capability` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait FluidCapabilityBackendMut {
    /// Side: server.
    /// Puts as much of `fluid` as tank `tank` of those `at`'s side reaches takes: a mod
    /// block's fillable tank, empty or holding that fluid, up to its capacity; a cauldron by
    /// whole thirds, lava only whole into an empty one, discarding the fluid's components as
    /// NeoForge's `CauldronFluidContent` does. What didn't fit. The write is kept by
    /// transaction `tx`.
    fn fluid_tank_insert(
        &mut self,
        at: BlockSide,
        tank: usize,
        fluid: FluidStack,
        tx: &Transaction,
    ) -> Option<FluidStack> {
        crate::phase::unprovided(
            "BackendMut::fluid_tank_insert",
            "fluid_tank_insert",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Takes up to `max` out of tank `tank` of those `at`'s side reaches: a mod block's
    /// drainable tank, a cauldron by whole thirds and lava only whole. What came out. The
    /// write is kept by transaction `tx`.
    fn fluid_tank_extract(
        &mut self,
        at: BlockSide,
        tank: usize,
        max: u32,
        tx: &Transaction,
    ) -> Option<FluidStack> {
        crate::phase::unprovided(
            "BackendMut::fluid_tank_extract",
            "fluid_tank_extract",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
