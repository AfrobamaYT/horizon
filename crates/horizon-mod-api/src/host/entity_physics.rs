//! The `entity_physics` area: host methods the game implements (`game/host/entity_physics.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `entity_physics` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait EntityPhysicsBackend {
    /// Side: server.
    /// `Entity.noPhysics`: a player's is whether it spectates (`Player.tick`
    /// sets it from `isSpectator`); a mob's is false (a vex sets it only for
    /// the length of its own tick); an arrow's or a trident's is its
    /// `AbstractArrow.noPhysics`, set once a loyal trident turns back. The
    /// ender dragon, an area-effect cloud, an ominous item spawner and a
    /// shulker bullet set it in their constructors and never clear it.
    fn entity_no_physics(&self, entity: EntityRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::entity_no_physics",
            "entity_no_physics",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
