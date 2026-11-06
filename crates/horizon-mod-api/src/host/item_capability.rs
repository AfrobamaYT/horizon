//! The `item_capability` area: host methods the game implements (`game/host/item_capability.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `item_capability` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait ItemCapabilityBackend {
    /// Side: server.
    /// How many slots of the block entity at `at` its side reaches (none: every slot), by its
    /// `WorldlyContainer` rules: a furnace's top reaches its input only. None where no block
    /// entity has slots, in a level not in place, and on a client's copy of the world.
    fn container_size(&self, at: BlockSide) -> Option<usize> {
        crate::phase::unprovided(
            "Backend::container_size",
            "container_size",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The stack in slot `slot` of those `at`'s side reaches (`native_container_size`).
    fn container_stack(&self, at: BlockSide, slot: usize) -> Option<ItemStack> {
        crate::phase::unprovided(
            "Backend::container_stack",
            "container_stack",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// How many of `stack` slot `slot` of those `at`'s side reaches holds when empty, as
    /// NeoForge's `isValid` and `getCapacityAsLong` answer: none where it may not go in (a
    /// furnace's output, `canPlaceItem`), else the slot's limit for it.
    fn container_capacity(&self, at: BlockSide, slot: usize, stack: &ItemStack) -> u32 {
        crate::phase::unprovided(
            "Backend::container_capacity",
            "container_capacity",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `item_capability` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait ItemCapabilityBackendMut {
    /// Side: server.
    /// Puts as much of `stack` as slot `slot` of those `at`'s side reaches takes, as a hopper
    /// would (`canPlaceItem`, the slot's limit, the same item and components), in transaction
    /// `tx`; what didn't fit.
    fn container_insert(
        &mut self,
        at: BlockSide,
        slot: usize,
        stack: ItemStack,
        tx: &Transaction,
    ) -> Option<ItemStack> {
        crate::phase::unprovided(
            "BackendMut::container_insert",
            "container_insert",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Takes up to `max` out of slot `slot` of those `at`'s side reaches, in transaction `tx`;
    /// through a side, only what that side may take (`canTakeItemThroughFace`). What came out.
    fn container_extract(
        &mut self,
        at: BlockSide,
        slot: usize,
        max: u32,
        tx: &Transaction,
    ) -> Option<ItemStack> {
        crate::phase::unprovided(
            "BackendMut::container_extract",
            "container_extract",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
