//! The `entity_remove` area: host methods the game implements (`game/host/entity_remove.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' server writes of the `entity_remove` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait EntityRemoveBackendMut {
    /// Side: server.
    /// Removes the entity with no death and no loot (`Entity.discard`): a
    /// chest boat or a minecart with a chest still drops what it holds, and
    /// a boat its lead, as vanilla's `remove` does. False for a player, or a
    /// stale handle.
    fn entity_discard(&mut self, entity: EntityRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_discard",
            "entity_discard",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Removes the entity for `reason` (`Entity.remove`): `Discarded`, as
    /// `discard`, or `Killed`, which also runs a mob's effects that act on
    /// removal (Oozing's slimes, Weaving's cobwebs, Wind Charged's burst),
    /// still with no death and no loot. False for a player, or a stale
    /// handle.
    fn entity_remove(&mut self, entity: EntityRef, reason: NativeRemovalReason) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_remove",
            "entity_remove",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Drops `stack` at the entity's feet, `y_offset` above
    /// (`Entity.spawnAtLocation`), or 0.15 out along a frame or painting's
    /// horizontal facing (`HangingEntity.spawnAtLocation`), moving as a new item does, with the
    /// default pickup delay: the item entity. None for an empty stack or an
    /// unknown item, a stale handle, a player in another level, or an item a
    /// mod kept out.
    fn entity_spawn_at_location(
        &mut self,
        entity: EntityRef,
        stack: ItemStack,
        y_offset: f32,
    ) -> Option<EntityRef> {
        crate::phase::unprovided(
            "BackendMut::entity_spawn_at_location",
            "entity_spawn_at_location",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
