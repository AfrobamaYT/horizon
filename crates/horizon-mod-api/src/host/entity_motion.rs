//! The `entity_motion` area: host methods the game implements (`game/host/entity_motion.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `entity_motion` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait EntityMotionBackend {
    /// Side: both.
    /// The entity's velocity in blocks a tick (`Entity.getDeltaMovement`). A
    /// player's is the move its client last reported (`getKnownMovement`); an
    /// entity Horizon doesn't simulate (a display, a painting) has none, so
    /// zero. On a client, another entity's is the motion the server last
    /// sent it (`Entity.lerpMotion`). API 379.
    fn entity_velocity(&self, entity: EntityRef) -> Option<[f64; 3]> {
        crate::phase::unprovided(
            "Backend::entity_velocity",
            "entity_velocity",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether the entity stands on a block (`Entity.onGround`).
    /// Whether the entity stands on a block (`Entity.onGround`). API
    /// 379.
    fn entity_on_ground(&self, entity: EntityRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::entity_on_ground",
            "entity_on_ground",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The entity's box (`Entity.getBoundingBox`): its lowest and highest
    /// corner. None for the kinds whose box Horizon doesn't keep (displays,
    /// markers, End crystals, the dragon). API 379.
    fn entity_bounding_box(&self, entity: EntityRef) -> Option<[[f64; 3]; 2]> {
        crate::phase::unprovided(
            "Backend::entity_bounding_box",
            "entity_bounding_box",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The height of the entity's eyes above its feet (`Entity.getEyeHeight`).
    /// API 379.
    fn entity_eye_height(&self, entity: EntityRef) -> Option<f64> {
        crate::phase::unprovided(
            "Backend::entity_eye_height",
            "entity_eye_height",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Where the entity's eyes are (`Entity.getEyePosition`). API
    /// 379.
    fn entity_eye_position(&self, entity: EntityRef) -> Option<[f64; 3]> {
        crate::phase::unprovided(
            "Backend::entity_eye_position",
            "entity_eye_position",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The unit vector the entity looks along (`Entity.getViewVector`, from
    /// its pitch and yaw). API 379.
    fn entity_view_vector(&self, entity: EntityRef) -> Option<[f64; 3]> {
        crate::phase::unprovided(
            "Backend::entity_view_vector",
            "entity_view_vector",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The square of the distance from the entity's feet to `pos`
    /// (`Entity.distanceToSqr`). API 379.
    fn entity_distance_to_sqr(&self, entity: EntityRef, pos: [f64; 3]) -> Option<f64> {
        crate::phase::unprovided(
            "Backend::entity_distance_to_sqr",
            "entity_distance_to_sqr",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `entity_motion` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait EntityMotionBackendMut {
    /// Side: server.
    /// Sets the entity's velocity (`Entity.setDeltaMovement`); clients see it
    /// as they see the entity move. False for a player (its client moves
    /// it), an entity Horizon doesn't simulate, a value that isn't finite,
    /// or on a client. API 379.
    fn entity_set_velocity(&mut self, entity: EntityRef, velocity: [f64; 3]) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_set_velocity",
            "entity_set_velocity",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
