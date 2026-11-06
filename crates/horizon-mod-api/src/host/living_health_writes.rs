//! The `living_health_writes` area: host methods the game implements (`game/host/living_health_writes.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `living_health_writes` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait LivingHealthWritesBackend {
    /// Side: server.
    /// `LivingEntity.getKillCredit`: the remembered player takes precedence,
    /// even if the player cannot be resolved; otherwise the last hurting mob.
    /// Unlike vanilla, a mob that changed dimension cannot be resolved.
    fn living_kill_credit(&self, living: LivingRef) -> Option<EntityRef> {
        crate::phase::unprovided(
            "Backend::living_kill_credit",
            "living_kill_credit",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LivingEntity.getLastHurtByMob`: the entity that last hurt this one,
    /// within a hundred ticks of the hurt (the window of
    /// `CombatTracker.getKiller`'s credit and a mob's hurt-by memory, as
    /// `armadillo.rs` reads it). None for a stale handle, a window that has
    /// passed, an attacker that is gone, an armor stand, the dragon or a
    /// client.
    fn living_last_attacker(&self, living: LivingRef) -> Option<EntityRef> {
        crate::phase::unprovided(
            "Backend::living_last_attacker",
            "living_last_attacker",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `living_health_writes` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait LivingHealthWritesBackendMut {
    /// Side: server.
    /// Sets the entity's health (`LivingEntity.setHealth`; Paper's
    /// `Damageable#setHealth`), cut to its maximum health. No heal or damage
    /// hook runs. False for a stale handle, a non-finite value, an armor stand
    /// or the dragon.
    fn living_set_health(&mut self, living: LivingRef, health: f32) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_set_health",
            "living_set_health",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Heals the entity by `amount` (`LivingEntity.heal`), through the
    /// `living_heal` hook, which may change or cancel it. A dead or dying
    /// entity isn't healed, and health stops at the maximum. False for a stale
    /// handle, a non-finite amount, a zero-health entity, an armor stand or the dragon.
    fn living_heal(&mut self, living: LivingRef, amount: f32) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_heal",
            "living_heal",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
