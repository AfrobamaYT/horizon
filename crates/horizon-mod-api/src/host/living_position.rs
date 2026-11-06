//! The `living_position` area: host methods the game implements (`game/host/living_position.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `living_position` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait LivingPositionBackend {
    /// Side: server.
    /// `LivingEntity.hasLineOfSight(Entity)`: whether nothing that collides
    /// (`ClipContext.Block.COLLIDER`, fluids ignored) lies between the
    /// living entity's eyes and `target`'s eye height, within 128 blocks.
    /// False for a target in another level. None for a stale or unavailable
    /// living entity or target, one outside the level in place, or on a
    /// client.
    fn living_has_line_of_sight(
        &self,
        living: LivingRef,
        target: EntityRef,
    ) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::living_has_line_of_sight",
            "living_has_line_of_sight",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `living_position` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait LivingPositionBackendMut {
    /// Side: server.
    /// `LivingEntity.randomTeleport(x, y, z, true, avoid)`: the point `at`
    /// is clamped into the world border and drops to the first block in
    /// `minecraft:entities_can_teleport_to` below it, losing a block of height
    /// per step down. The entity lands there, keeping its motion, unless that
    /// ground or a block in its box at the landing is in the `avoid` block
    /// tag, the box collides or holds a liquid, or the species refuses the
    /// spot (an enderman never lands on ground holding water). A landing
    /// stops a mob's navigation and broadcasts entity event 46 (the portal
    /// particles). Nothing else is played: a chorus fruit's sound and an
    /// enderman's own teleport effects belong to their callers.
    ///
    /// True when the entity moved. False when no spot was found, for a
    /// non-finite point, a fake player, a player in another level, an armor
    /// stand or the dragon, a stale handle, or on a client.
    fn living_random_teleport(
        &mut self,
        living: LivingRef,
        at: [f64; 3],
        avoid: Option<&str>,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_random_teleport",
            "living_random_teleport",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
