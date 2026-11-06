//! The `living_health` area: host methods the game implements (`game/host/living_health.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `living_health` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait LivingHealthBackend {
    /// Side: both.
    /// The entity's health (`LivingEntity.getHealth`; Paper's
    /// `Damageable#getHealth`). During a player's turn only that player
    /// resolves: the hub is out for their action, and other players'
    /// handles read `None` until it is back.
    fn living_health(&self, living: LivingRef) -> Option<f32> {
        crate::phase::unprovided(
            "Backend::living_health",
            "living_health",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The entity's maximum health (`LivingEntity.getMaxHealth`; Paper's
    /// `Damageable#getMaxHealth`). Players and guests read their attribute
    /// map; a mob, an armor stand and the dragon their own fields (the
    /// armor stand the fixed 20 the game seeds it with, the dragon its
    /// 200) until live attribute maps cover them.
    fn living_max_health(&self, living: LivingRef) -> Option<f32> {
        crate::phase::unprovided(
            "Backend::living_max_health",
            "living_max_health",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether the entity is alive (`LivingEntity.isAlive`; Paper's
    /// `Entity#isDead` is its inverse). A dying mob, mid death
    /// animation, is not.
    fn living_alive(&self, living: LivingRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::living_alive",
            "living_alive",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
