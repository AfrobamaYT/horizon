//! The `entity_invulnerable` area: host methods the game implements (`game/host/entity_invulnerable.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `entity_invulnerable` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait EntityInvulnerableBackend {
    /// Side: server.
    /// `Entity.isPermanentlyInvulnerable`: the `Invulnerable` flag, which
    /// only damage that bypasses invulnerability (the void, `/kill`) or a
    /// creative player gets past. Mobs keep it; Horizon keeps no such flag
    /// for players or other kinds yet, so they read None.
    fn entity_permanently_invulnerable(&self, entity: EntityRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::entity_permanently_invulnerable",
            "entity_permanently_invulnerable",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LivingEntity.damageCooldownTime`: 20 after a hit, counting down;
    /// while it is above 10 a hit does only what it deals beyond the last
    /// one. Mobs and players.
    fn entity_damage_cooldown_time(&self, entity: EntityRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::entity_damage_cooldown_time",
            "entity_damage_cooldown_time",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `entity_invulnerable` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait EntityInvulnerableBackendMut {
    /// Side: server.
    /// `Entity.setInvulnerable`: sets a mob's `Invulnerable` flag, saved
    /// with it. False for the other kinds.
    fn entity_set_invulnerable(&mut self, entity: EntityRef, invulnerable: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_set_invulnerable",
            "entity_set_invulnerable",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets `LivingEntity.damageCooldownTime`, as mods do to let rapid hits
    /// through (0). Mobs and players.
    fn entity_set_damage_cooldown_time(&mut self, entity: EntityRef, ticks: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_set_damage_cooldown_time",
            "entity_set_damage_cooldown_time",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
