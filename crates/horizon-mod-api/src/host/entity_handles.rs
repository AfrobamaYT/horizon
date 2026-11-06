//! The `entity_handles` area: host methods the game implements (`game/host/entity_handles.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `entity_handles` area, implemented by the game and reached through [`Host`].
pub trait EntityHandlesHost {
    /// Side: both.
    /// The entity `id` (the id events carry) as a handle for the `entity_*`
    /// methods: None with no world open, or for no such entity in the level
    /// in place or among the server's players. On a client (API
    /// @API:client_entity_refs@), any entity the server shows this window,
    /// this window's own player included. API 373.
    fn entity(&self, id: u32) -> Option<EntityRef> {
        crate::phase::unprovided(
            "Host::entity",
            "entity",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The level `dimension` (`minecraft:the_nether`) as a handle: None on a
    /// client, with no world open, or for a dimension the server hasn't
    /// loaded. API 373.
    fn level(&self, dimension: &str) -> Option<LevelRef> {
        crate::phase::unprovided(
            "Host::level",
            "level",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' reads of the `entity_handles` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait EntityHandlesBackend {
    /// Side: both.
    /// The entity's position (`Entity.position`): its feet, the middle of
    /// its box for a painting or an item frame.
    fn entity_position(&self, entity: EntityRef) -> Option<[f64; 3]> {
        crate::phase::unprovided(
            "Backend::entity_position",
            "entity_position",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The block the entity's position is in (`Entity.blockPosition`). API
    /// 373.
    fn entity_block_position(&self, entity: EntityRef) -> Option<[i32; 3]> {
        crate::phase::unprovided(
            "Backend::entity_block_position",
            "entity_block_position",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The entity's yaw and pitch in degrees (`Entity.getYRot`, `getXRot`); 0
    /// for the kinds that don't turn. API 373.
    fn entity_rotation(&self, entity: EntityRef) -> Option<[f32; 2]> {
        crate::phase::unprovided(
            "Backend::entity_rotation",
            "entity_rotation",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The entity's type (`Entity.getType`): `minecraft:zombie`. API
    /// 373.
    fn entity_kind(&self, entity: EntityRef) -> Option<String> {
        crate::phase::unprovided(
            "Backend::entity_kind",
            "entity_kind",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The level the entity is in (`Entity.level`): on a client, the
    /// client's level. API 373.
    fn entity_level(&self, entity: EntityRef) -> Option<LevelRef> {
        crate::phase::unprovided(
            "Backend::entity_level",
            "entity_level",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `entity_handles` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait EntityHandlesBackendMut {
    /// Side: server.
    /// The entity's UUID (`Entity.getUUID`), the one it is saved with; an
    /// entity that has none yet gets it now, as a new vanilla entity does.
    /// On a client, the one the server added it with (a player's is its
    /// profile's). API 373.
    fn entity_uuid(&mut self, entity: EntityRef) -> Option<u128> {
        crate::phase::unprovided(
            "BackendMut::entity_uuid",
            "entity_uuid",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
