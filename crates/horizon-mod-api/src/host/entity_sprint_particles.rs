//! The `entity_sprint_particles` area: host methods the game implements (`game/host/entity_sprint_particles.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' client writes of the `entity_sprint_particles` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait EntitySprintParticlesClientBackendMut {
    /// Side: client.
    /// Spawn sprint particles when called explicitly by a client mod. A
    /// valid entity returns true even if the block beneath it is invisible.
    /// Returns false for stale, untracked, server-side, or unsupported entities.
    fn entity_sprint_particles(&mut self, entity: EntityRef) -> bool {
        crate::phase::unprovided(
            "ClientBackendMut::entity_sprint_particles",
            "entity_sprint_particles",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
