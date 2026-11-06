//! The `projectile_refs` area: host methods the game implements (`game/host/projectile_refs.rs`).
#![allow(unused_variables)]
use crate::*;
/// The server host's methods of the `projectile_refs` area, implemented by the game and reached through [`ServerHost`].
pub trait ProjectileRefsServerHost {
    /// Side: server.
    /// Narrows an entity to a projectile: one in the projectile list, a trident
    /// a player threw or a firework rocket. None for another kind, a stale
    /// handle or a client.
    fn projectile(&self, entity: EntityRef) -> Option<ProjectileRef> {
        crate::phase::unprovided(
            "ServerHost::projectile",
            "projectile",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' reads of the `projectile_refs` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait ProjectileRefsBackend {
    /// Side: server.
    /// `Projectile.getOwner`: the owner as a resolved entity. None for no
    /// owner, one that is gone or in another level, or a stale handle.
    fn projectile_owner(&self, projectile: ProjectileRef) -> Option<EntityRef> {
        crate::phase::unprovided(
            "Backend::projectile_owner",
            "projectile_owner",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `AbstractArrow.pickup`, for an arrow or a mob's trident.
    fn projectile_pickup(&self, projectile: ProjectileRef) -> Option<NativeArrowPickup> {
        crate::phase::unprovided(
            "Backend::projectile_pickup",
            "projectile_pickup",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `AbstractArrow.getPierceLevel`: the signed byte stored by vanilla.
    fn projectile_pierce_level(&self, projectile: ProjectileRef) -> Result<i8, PierceLevelError> {
        crate::phase::unprovided(
            "Backend::projectile_pierce_level",
            "projectile_pierce_level",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `AbstractArrow.getBaseDamage`, for an arrow (a trident's is fixed).
    fn projectile_base_damage(&self, projectile: ProjectileRef) -> Option<f64> {
        crate::phase::unprovided(
            "Backend::projectile_base_damage",
            "projectile_base_damage",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `AbstractArrow.isCritArrow`, for an arrow.
    fn projectile_critical(&self, projectile: ProjectileRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::projectile_critical",
            "projectile_critical",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `projectile_refs` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait ProjectileRefsBackendMut {
    /// Side: server.
    /// `Projectile.setOwner`. False for a stale projectile, or an owner that is
    /// stale or neither a player nor a mob (a trident a player threw takes a
    /// player only). Does not touch `leftOwner`.
    fn projectile_set_owner(
        &mut self,
        projectile: ProjectileRef,
        owner: Option<EntityRef>,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::projectile_set_owner",
            "projectile_set_owner",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Projectile.deflect(deflection, by, newOwner, byAttack, power)`, with
    /// `power` per axis, through `Projectile::deflect` for the projectile list
    /// (the engine path gameplay uses). `REVERSE` draws one float from the
    /// projectile's own random; `AIM_DEFLECT` uses `by`'s look and
    /// `MOMENTUM_DEFLECT` its normalized known speed, and both change nothing
    /// without `by`. The new owner is set, then `onDeflection`: a hurting
    /// projectile's acceleration power is 0.1 by an attack, else halved. A
    /// wind charge refuses for its first five ticks, with no change and no
    /// draw. A trident a player threw and a rocket turn the same way (no
    /// acceleration power). False, and nothing changed, for a stale projectile,
    /// `by` or owner, a non-finite power or a refusal.
    fn projectile_deflect(
        &mut self,
        projectile: ProjectileRef,
        deflection: NativeDeflection,
        by: Option<EntityRef>,
        new_owner: Option<EntityRef>,
        by_attack: bool,
        power: [f64; 3],
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::projectile_deflect",
            "projectile_deflect",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets `AbstractArrow.pickup`. False for a stale handle or a projectile
    /// that is not an arrow or a mob's trident.
    fn projectile_set_pickup(
        &mut self,
        projectile: ProjectileRef,
        pickup: NativeArrowPickup,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::projectile_set_pickup",
            "projectile_set_pickup",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `AbstractArrow.setPierceLevel`: stores the signed byte without clamping.
    fn projectile_set_pierce_level(
        &mut self,
        projectile: ProjectileRef,
        level: i8,
    ) -> Result<(), PierceLevelError> {
        crate::phase::unprovided(
            "BackendMut::projectile_set_pierce_level",
            "projectile_set_pierce_level",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `AbstractArrow.setBaseDamage`. False for a stale handle, a projectile
    /// that is not an arrow, or a value that is not finite.
    fn projectile_set_base_damage(&mut self, projectile: ProjectileRef, damage: f64) -> bool {
        crate::phase::unprovided(
            "BackendMut::projectile_set_base_damage",
            "projectile_set_base_damage",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `AbstractArrow.setCritArrow`; clients see the flag at once through the
    /// entity data. False for a stale handle or a projectile that is not an arrow.
    fn projectile_set_critical(&mut self, projectile: ProjectileRef, critical: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::projectile_set_critical",
            "projectile_set_critical",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
