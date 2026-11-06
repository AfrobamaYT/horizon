//! The `level_entities_in` area: host methods the game implements (`game/host/level_entities_in.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `level_entities_in` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait LevelEntitiesInBackend {
    /// Side: server.
    /// Entities whose bounding boxes intersect this box (`Level.getEntities`).
    /// Face-only contact does not intersect; spectator players are excluded.
    /// Returns None on a client, for a stale handle, or for a parked level:
    /// its entity handles cannot resolve until the level is in place. The walk
    /// is in entity-list order followed by players; each overlap requires a
    /// handle lookup, so cost grows with the box's contents. Tridents cannot
    /// yet resolve; frames, paintings, other special entities and the dragon
    /// are not on the entity-list walk.
    /// During `hears_projectile_impacts`, the projectile list is taken out of
    /// the game and is not visible; it is restored for `projectile_impact`.
    fn level_entities_in(
        &self,
        level: LevelRef,
        min: [f64; 3],
        max: [f64; 3],
    ) -> Option<Vec<EntityRef>> {
        crate::phase::unprovided(
            "Backend::level_entities_in",
            "level_entities_in",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
