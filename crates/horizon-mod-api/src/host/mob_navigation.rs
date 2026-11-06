//! The `mob_navigation` area: host methods the game implements (`game/host/mob_navigation.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `mob_navigation` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait MobNavigationBackend {
    /// Side: server.
    /// `PathNavigation.isDone`; None for a stale or unavailable mob, or on a
    /// client, and for the dragon (no navigation).
    fn mob_is_done(&self, mob: MobRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::mob_is_done",
            "mob_is_done",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `PathNavigation.getSpeedModifier`: the multiplier the current path
    /// is followed at (1.0 is the mob's normal speed; 0 until a path sets
    /// one). This is not the mob's movement-speed attribute. None for a
    /// stale or unavailable mob, or on a client, and for the dragon.
    fn mob_navigation_speed(&self, mob: MobRef) -> Option<f64> {
        crate::phase::unprovided(
            "Backend::mob_navigation_speed",
            "mob_navigation_speed",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.getPathfindingMalus`: what a node of `kind` costs this mob's
    /// pathfinder — its own override, else the kind's default (a negative
    /// malus closes the kind). None for a stale or unavailable mob, the
    /// dragon, or on a client.
    fn mob_pathfinding_malus(&self, mob: MobRef, kind: PathType) -> Option<f32> {
        crate::phase::unprovided(
            "Backend::mob_pathfinding_malus",
            "mob_pathfinding_malus",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `mob_navigation` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait MobNavigationBackendMut {
    /// Side: server.
    /// `PathNavigation.moveTo(x, y, z, speed)`: true if a path was started.
    /// Refuses a stale or unavailable mob, a client, non-finite arguments,
    /// and coordinates too large for safe block arithmetic. As in vanilla,
    /// the mob's own goals may replace the path next tick. False for the
    /// dragon, which flies by its phase and keeps no navigation.
    fn mob_move_to(&mut self, mob: MobRef, at: [f64; 3], speed: f64) -> bool {
        crate::phase::unprovided(
            "BackendMut::mob_move_to",
            "mob_move_to",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `PathNavigation.stop`. False for a stale or unavailable mob, or on a
    /// client, and for the dragon (no navigation).
    fn mob_stop(&mut self, mob: MobRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::mob_stop",
            "mob_stop",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `PathNavigation.setSpeedModifier`: sets the multiplier the current
    /// path is followed at, as `move_to`'s speed argument does. The mob's
    /// own AI — a goal's or a brain species' next `moveTo` — sets its own
    /// again, as in vanilla. False for a non-finite speed, a stale or
    /// unavailable mob, or on a client, and for the dragon.
    fn mob_set_navigation_speed(&mut self, mob: MobRef, speed: f64) -> bool {
        crate::phase::unprovided(
            "BackendMut::mob_set_navigation_speed",
            "mob_set_navigation_speed",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Mob.setPathfindingMalus`: what the mob's next paths cost through
    /// `kind` (negative closes it, as the species' own overrides do); not
    /// saved, as in vanilla, so a reload or conversion keeps the species'
    /// own. False for a malus that is not finite, a stale or unavailable mob,
    /// the dragon, or on a client.
    fn mob_set_pathfinding_malus(&mut self, mob: MobRef, kind: PathType, malus: f32) -> bool {
        crate::phase::unprovided(
            "BackendMut::mob_set_pathfinding_malus",
            "mob_set_pathfinding_malus",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
