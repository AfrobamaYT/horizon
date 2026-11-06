//! The `entity_riding` area: host methods the game implements (`game/host/entity_riding.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `entity_riding` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait EntityRidingBackend {
    /// Side: server.
    /// What the entity rides (`Entity.getVehicle`): `None` when it rides
    /// nothing, or for a stale handle.
    fn entity_vehicle(&self, entity: EntityRef) -> Option<EntityRef> {
        crate::phase::unprovided(
            "Backend::entity_vehicle",
            "entity_vehicle",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether the entity rides another (`Entity.isPassenger`).
    fn entity_is_passenger(&self, entity: EntityRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::entity_is_passenger",
            "entity_is_passenger",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The lowest vehicle under the entity (`Entity.getRootVehicle`): the
    /// entity itself when it rides nothing.
    fn entity_root_vehicle(&self, entity: EntityRef) -> Option<EntityRef> {
        crate::phase::unprovided(
            "Backend::entity_root_vehicle",
            "entity_root_vehicle",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The entities riding this one (`Entity.getPassengers`): its players
    /// in the order they got in, then its mobs.
    fn entity_passengers(&self, entity: EntityRef) -> Option<Vec<EntityRef>> {
        crate::phase::unprovided(
            "Backend::entity_passengers",
            "entity_passengers",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether anything rides the entity (`Entity.isVehicle`).
    fn entity_is_vehicle(&self, entity: EntityRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::entity_is_vehicle",
            "entity_is_vehicle",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `entity_riding` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait EntityRidingBackendMut {
    /// Side: server.
    /// Attempts to board a vehicle through the shared server mount path.
    ///
    /// Players and mobs can ride a mob, boat or minecart. Returns false for
    /// unsupported entity pairs, stale or held-out handles, a full stored
    /// vehicle, a cycle, or a canceled mount or dismount. Mob vehicle capacity
    /// follows its species; minecarts store one passenger and boats store one
    /// mob and up to two passengers in total. `force` bypasses normal
    /// admission checks, not these storage limits. Player boarding cooldown and the strider's
    /// eye-in-lava admission check are not yet represented by the game.
    fn entity_start_riding(&mut self, entity: EntityRef, vehicle: EntityRef, force: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_start_riding",
            "entity_start_riding",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Gets the entity off its vehicle (`Entity.stopRiding`), where the
    /// vehicle lets it out, as `/ride ... dismount` does. False when it rode
    /// nothing, a mod's `mob_ride` hook kept it on, or for a stale handle.
    fn entity_stop_riding(&mut self, entity: EntityRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_stop_riding",
            "entity_stop_riding",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Gets every passenger off the entity (`Entity.ejectPassengers`), last
    /// first as vanilla walks the list, so `mob_ride` hears them in its
    /// order. False when nothing rode it, or for a stale handle.
    fn entity_eject_passengers(&mut self, entity: EntityRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::entity_eject_passengers",
            "entity_eject_passengers",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
