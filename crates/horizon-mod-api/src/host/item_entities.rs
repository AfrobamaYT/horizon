//! The `item_entities` area: host methods the game implements (`game/host/item_entities.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `item_entities` area, implemented by the game and reached through [`Host`].
pub trait ItemEntitiesHost {
    /// Side: both.
    /// Narrows an entity to a dropped item, or None for another kind, a
    /// stale handle, a client, or an item temporarily taken out for ticking.
    fn item_entity(&self, entity: EntityRef) -> Option<ItemEntityRef> {
        crate::phase::unprovided(
            "Host::item_entity",
            "item_entity",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' reads of the `item_entities` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait ItemEntitiesBackend {
    /// Side: server.
    /// `ItemEntity.getItem`: the complete stack, or None if unavailable.
    fn item_entity_stack(&self, item: ItemEntityRef) -> Option<ItemStack> {
        crate::phase::unprovided(
            "Backend::item_entity_stack",
            "item_entity_stack",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The signed pickup delay; the legacy infinite sentinel reads as vanilla's
    /// 32767. None also covers a stored value outside i32.
    fn item_entity_pickup_delay(&self, item: ItemEntityRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::item_entity_pickup_delay",
            "item_entity_pickup_delay",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemEntity.getAge`, including negative lifetime sentinels.
    fn item_entity_age(&self, item: ItemEntityRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::item_entity_age",
            "item_entity_age",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The UUID set by `ItemEntity.setTarget`, limiting pickup to that player;
    /// None also means an unavailable item. This is not the thrower.
    fn item_entity_target(&self, item: ItemEntityRef) -> Option<u128> {
        crate::phase::unprovided(
            "Backend::item_entity_target",
            "item_entity_target",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemEntity.getOwner`: the thrower as a resolved entity, not the pickup
    /// target. None for no thrower, an unloaded or different-level thrower,
    /// an unavailable handle, or one Horizon cannot resolve.
    fn item_entity_owner(&self, item: ItemEntityRef) -> Option<EntityRef> {
        crate::phase::unprovided(
            "Backend::item_entity_owner",
            "item_entity_owner",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' writes of the `item_entities` area, implemented by the game and reached through [`BackendMut`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait ItemEntitiesBackendMut {
    /// Side: server.
    /// Writes a nonempty complete stack using `ItemEntity.setItem`. Empty
    /// stacks are refused: vanilla discards the item on its next tick. To
    /// remove an item entity, use the removal handle. False also for an
    /// unavailable item or outside a server.
    fn item_entity_set_stack(&mut self, item: ItemEntityRef, stack: ItemStack) -> bool {
        crate::phase::unprovided(
            "BackendMut::item_entity_set_stack",
            "item_entity_set_stack",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemEntity.setPickUpDelay`.
    fn item_entity_set_pickup_delay(&mut self, item: ItemEntityRef, ticks: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::item_entity_set_pickup_delay",
            "item_entity_set_pickup_delay",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemEntity.setDefaultPickUpDelay`: ten ticks.
    fn item_entity_set_default_pickup_delay(&mut self, item: ItemEntityRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::item_entity_set_default_pickup_delay",
            "item_entity_set_default_pickup_delay",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemEntity.setNoPickUpDelay`.
    fn item_entity_set_no_pickup_delay(&mut self, item: ItemEntityRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::item_entity_set_no_pickup_delay",
            "item_entity_set_no_pickup_delay",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemEntity.setNeverPickUp`: the infinite-delay sentinel 32767.
    fn item_entity_set_never_pickup(&mut self, item: ItemEntityRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::item_entity_set_never_pickup",
            "item_entity_set_never_pickup",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemEntity.setTarget`; None allows any player to pick it up.
    fn item_entity_set_target(&mut self, item: ItemEntityRef, target: Option<u128>) -> bool {
        crate::phase::unprovided(
            "BackendMut::item_entity_set_target",
            "item_entity_set_target",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
