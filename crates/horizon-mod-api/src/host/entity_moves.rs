//! The `entity_moves` area: host methods the game implements (`game/host/entity_moves.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `entity_moves` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait EntityMovesBackend {
    /// Side: server.
    /// The yaw of the entity's head (`Entity.getYHeadRot`): a mob's or a
    /// player's own, zero for the kinds without a head.
    fn entity_head_yaw(&self, entity: EntityRef) -> Option<f32> {
        crate::phase::unprovided(
            "Backend::entity_head_yaw",
            "entity_head_yaw",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The horizontal direction the entity faces (`Entity.getDirection`):
    /// `north`, `south`, `east` or `west`.
    fn entity_direction(&self, entity: EntityRef) -> Option<&'static str> {
        crate::phase::unprovided(
            "Backend::entity_direction",
            "entity_direction",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The entity's width and height (`Entity.getBbWidth`, `getBbHeight`):
    /// its box's, so a baby's or a scaled entity's own.
    fn entity_dimensions(&self, entity: EntityRef) -> Option<[f64; 2]> {
        crate::phase::unprovided(
            "Backend::entity_dimensions",
            "entity_dimensions",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `entity_moves` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait EntityMovesBackendMut {
    /// Side: server.
    /// Moves the entity to `pos` in its level (`Entity.teleportTo`), as
    /// `/tp` does: a player is placed there and told, a rider gets off. With
    /// `rotation` (yaw, pitch in degrees) it also turns. False for a position
    /// outside the world's bounds, for wall-mounted entities, or when the
    /// requested rotation cannot be stored.
    fn entity_teleport_to(
        &mut self,
        entity: EntityRef,
        pos: [f64; 3],
        rotation: Option<[f32; 2]>,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_teleport_to",
            "entity_teleport_to",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Turns the entity to `yaw` and `pitch` in degrees (`Entity.setYRot`,
    /// `setXRot`): the pitch wraps and is clamped to -90..90. False for kinds
    /// whose rotation Horizon does not fully store, or an unavailable player.
    /// Does not teleport or dismount.
    fn entity_set_rotation(&mut self, entity: EntityRef, yaw: f32, pitch: f32) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_set_rotation",
            "entity_set_rotation",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Turns a mob's head to `yaw` (`Entity.setYHeadRot`); its body follows
    /// as its AI turns it. Players' heads can turn too; false for other kinds
    /// or an unavailable player.
    fn entity_set_head_yaw(&mut self, entity: EntityRef, yaw: f32) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_set_head_yaw",
            "entity_set_head_yaw",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Adds `delta` to the entity's velocity (`Entity.push`,
    /// `addDeltaMovement`). False for an unavailable player, an entity
    /// Horizon doesn't simulate, or a value that isn't finite.
    fn entity_push(&mut self, entity: EntityRef, delta: [f64; 3]) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_push",
            "entity_push",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
