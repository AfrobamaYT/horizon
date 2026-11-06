//! The `living_flags` area: host methods the game implements (`game/host/living_flags.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `living_flags` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait LivingFlagsBackend {
    /// Side: both.
    /// The ticks the entity has glided (`LivingEntity.getFallFlyingTicks`),
    /// counted from 0 again the tick after the glide ends. Players (including
    /// inactive guests) and mobs read their own stored body counter. None
    /// for an armor stand or the dragon.
    fn living_gliding_ticks(&self, living: LivingRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::living_gliding_ticks",
            "living_gliding_ticks",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Whether the entity glides on elytra (`LivingEntity.isFallFlying`;
    /// Paper's `LivingEntity#isGliding`). The player in place and the guests
    /// answer from their fall-flying flag, a mob from its own. None for an
    /// armor stand or the dragon: the game keeps no glide for them.
    fn living_is_gliding(&self, living: LivingRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::living_is_gliding",
            "living_is_gliding",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Whether the entity sleeps in a bed (`LivingEntity.isSleeping`): the
    /// player in place, a guest, or a villager or wandering trader asleep in
    /// its bed. Other mobs never sleep. None for an armor stand or the
    /// dragon.
    fn living_is_sleeping(&self, living: LivingRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::living_is_sleeping",
            "living_is_sleeping",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The head of the bed the entity sleeps in (`LivingEntity.getSleepingPos`),
    /// as the block position it lies at. A guest keeps the bed it last slept
    /// in while it sleeps, a villager or wandering trader its own. None for
    /// an awake entity, another mob, an armor stand or the dragon.
    fn living_sleeping_pos(&self, living: LivingRef) -> Option<[i32; 3]> {
        crate::phase::unprovided(
            "Backend::living_sleeping_pos",
            "living_sleeping_pos",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Whether the entity jumps (`LivingEntity.isJumping`; Paper's
    /// `LivingEntity#isJumping`): the player in place answers from their
    /// own jump state. The guests' jump is their client's, and a mob has no
    /// jump controller in the game's own AI, so both read false. None for
    /// an armor stand or the dragon.
    fn living_is_jumping(&self, living: LivingRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::living_is_jumping",
            "living_is_jumping",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Whether the entity is on a climbable block this tick
    /// (`LivingEntity.onClimbable`; Paper's `LivingEntity#isClimbing`), by
    /// the block at its feet: a ladder, a vine, a scaffold or an open
    /// trapdoor above a ladder. A spectator never climbs; a gliding entity
    /// ignores blocks tagged to glide through, such as vines. None for an
    /// armor stand or the dragon.
    fn living_is_climbing(&self, living: LivingRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::living_is_climbing",
            "living_is_climbing",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Whether the entity is a baby (`LivingEntity.isBaby`): a mob answers
    /// from its age, negative until it grows up. A player, an armor stand
    /// and the dragon are never babies.
    fn living_baby(&self, living: LivingRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::living_baby",
            "living_baby",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `living_flags` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait LivingFlagsBackendMut {
    /// Side: server.
    /// `LivingEntity.stopFallFlying`: clears the glide flag, so the glide's
    /// wear and sound stop and the next fall is a fall. The players around
    /// learn it with the entity's next flag sync, as they do when a glide
    /// ends on its own. True for a player, a guest or a mob; false for an
    /// armor stand or the dragon.
    fn living_stop_gliding(&mut self, living: LivingRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_stop_gliding",
            "living_stop_gliding",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LivingEntity.startSleeping`: the entity lies in the bed whose head
    /// `pos` names, with the effect the game's own sleep has. The player in
    /// place is moved onto the bed, put to sleep and occupies it; a
    /// villager or wandering trader takes a valid, unoccupied bed. Normal
    /// integrated-owner, LAN and dedicated guests use their server body,
    /// inside or outside their own turn, in the level in place. False for
    /// a guest in another level, a fake player, another
    /// mob, an armor stand or the dragon, or a bed without a sleep shape.
    fn living_start_sleeping(&mut self, living: LivingRef, pos: [i32; 3]) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_start_sleeping",
            "living_start_sleeping",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LivingEntity.stopSleeping`: the sleeper stands up beside the bed, as
    /// the game's own waking does, with its pose and packets. True when the
    /// entity slept; false for a fake player, another mob, an armor stand
    /// or the dragon. Normal server guests in the level in place use the
    /// same wake-up path; guests in another level are refused.
    fn living_stop_sleeping(&mut self, living: LivingRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_stop_sleeping",
            "living_stop_sleeping",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LivingEntity.setSleepingPos`: the bed `pos` names becomes the one
    /// the player sleeps in, without moving them; the next entity sync
    /// tells the players around they sleep there, as it tells them the
    /// game's own sleep. The villagers' beds belong to the evening's bed
    /// path (`start_sleeping`). Normal server guests in the level in place
    /// are supported; guests in another level, fake players, mobs, armor
    /// stands and the dragon are refused. This only
    /// changes the sleeping position/pose, not bed occupancy.
    fn living_set_sleeping_pos(&mut self, living: LivingRef, pos: [i32; 3]) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_set_sleeping_pos",
            "living_set_sleeping_pos",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LivingEntity.clearSleepingPos`: the player no longer names a bed, so
    /// `is_sleeping` is false and the next entity sync stands them up.
    /// Normal server guests in the level in place are supported; guests in
    /// another level, fake players, mobs, armor stands and the dragon are
    /// refused. This does not run the wake-up animation
    /// or move the player; use `stop_sleeping` to stand beside the bed.
    fn living_clear_sleeping_pos(&mut self, living: LivingRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_clear_sleeping_pos",
            "living_clear_sleeping_pos",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LivingEntity.setJumping`: whether the player in place's next step
    /// jumps, as their own key does. False for a guest outside its turn,
    /// whose client holds its own jump, a mob, an armor stand or the
    /// dragon.
    fn living_set_is_jumping(&mut self, living: LivingRef, jumping: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_set_is_jumping",
            "living_set_is_jumping",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
