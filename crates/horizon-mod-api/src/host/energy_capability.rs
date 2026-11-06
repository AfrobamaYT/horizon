//! The `energy_capability` area: host methods the game implements (`game/host/energy_capability.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `energy_capability` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait EnergyCapabilityBackend {
    /// Side: server.
    /// The energy the buffer of the mod block at `at` in the level in place holds, for the
    /// game's own energy capability: none where no buffer is, in another level, a chunk that
    /// isn't loaded, or the block entity ticking now, or where `at`'s side reaches neither a
    /// fill nor a drain.
    fn energy_amount(&self, at: BlockSide) -> Option<u32> {
        crate::phase::unprovided(
            "Backend::energy_amount",
            "energy_amount",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The capacity of the buffer at `at` (`native_energy_amount`), 0 where none is.
    fn energy_capacity(&self, at: BlockSide) -> u32 {
        crate::phase::unprovided(
            "Backend::energy_capacity",
            "energy_capacity",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `energy_capability` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait EnergyCapabilityBackendMut {
    /// Side: server.
    /// `SimpleEnergyHandler.insert`: puts up to `amount` into the buffer at `at`
    /// (`native_energy_amount`) through its side, no more than its room and its limit per call,
    /// kept by transaction `tx`. How much went in.
    fn energy_insert(&mut self, at: BlockSide, amount: u32, tx: &Transaction) -> u32 {
        crate::phase::unprovided(
            "BackendMut::energy_insert",
            "energy_insert",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `SimpleEnergyHandler.extract`: takes up to `amount` out of the buffer at `at`
    /// (`native_energy_amount`) by its side, no more than it holds and its limit per call, kept
    /// by transaction `tx`. How much came out.
    fn energy_extract(&mut self, at: BlockSide, amount: u32, tx: &Transaction) -> u32 {
        crate::phase::unprovided(
            "BackendMut::energy_extract",
            "energy_extract",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
