//! The `entity_fall_and_push` area: host methods the game implements (`game/host/entity_fall_and_push.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `entity_fall_and_push` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait EntityFallAndPushBackend {
    /// Side: server.
    /// A furnace minecart's push (`MinecartFurnace.push`): the way its fuel drives it on rails,
    /// x and z; its y part stays between ticks and sets the push's length. None for another
    /// kind of cart.
    fn minecart_push(&self, cart: MinecartRef) -> Option<[f64; 3]> {
        crate::phase::unprovided(
            "Backend::minecart_push",
            "minecart_push",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `entity_fall_and_push` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait EntityFallAndPushBackendMut {
    /// Side: server.
    /// `Entity.resetFallDistance`: how far the entity has fallen is forgotten, so its next
    /// landing hurts it as a step does (a fan's or a launcher's push, a mod's soft block).
    /// Players too, the acting one and guests. False for a stale handle or an entity not in
    /// the level in place; true for a kind that keeps no fall distance, which has none to forget.
    fn entity_reset_fall_distance(&mut self, entity: EntityRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_reset_fall_distance",
            "entity_reset_fall_distance",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets a furnace minecart's push, as a player's click with fuel sets it toward them
    /// (`MinecartFurnace.interact`): a cart a mod drives runs that way while it has fuel. False
    /// for another kind of cart or a push that isn't finite.
    fn minecart_set_push(&mut self, cart: MinecartRef, push: [f64; 3]) -> bool {
        crate::phase::unprovided(
            "BackendMut::minecart_set_push",
            "minecart_set_push",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
