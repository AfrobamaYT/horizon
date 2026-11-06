//! The `level_item_entities` area: host methods the game implements (`game/host/level_item_entities.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' server writes of the `level_item_entities` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait LevelItemEntitiesBackendMut {
    /// Side: server.
    /// Drops `stack` as a broken block drops it (`Block.popResource`): an
    /// item entity at a random point within a quarter block of the middle of
    /// `pos` (three draws from the level random, made for an empty stack and
    /// with `block_drops` off too), with its own random motion and the
    /// default pickup delay of 10 ticks. None, with nothing spawned, for an
    /// empty stack or an unknown item, with the `block_drops` rule off, on a
    /// client, for a stale handle, a level not in place, or an item a mod
    /// kept out (`native_entity_join_level`).
    fn level_pop_resource(
        &mut self,
        level: LevelRef,
        pos: [i32; 3],
        stack: ItemStack,
    ) -> Option<EntityRef> {
        crate::phase::unprovided(
            "BackendMut::level_pop_resource",
            "level_pop_resource",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Spawns `stack` as an item entity at `x y z` (`new ItemEntity(level, x,
    /// y, z, stack)` and `addFreshEntity`): its own random motion (up 0.2, up
    /// to 0.1 either way across), with `pickup_delay` ticks before it can be
    /// picked up, as `ItemEntity.setPickupDelay` at construction (vanilla
    /// passes 10 for a block's drop, 40 for a player's throw and 32767 for
    /// never). None, with nothing
    /// spawned, for an empty stack or an unknown item, a value that isn't
    /// finite, on a client, for a stale handle, a level not in place, or an
    /// item a mod kept out (`native_entity_join_level`).
    fn level_spawn_item(
        &mut self,
        level: LevelRef,
        x: f64,
        y: f64,
        z: f64,
        stack: ItemStack,
        pickup_delay: u32,
    ) -> Option<EntityRef> {
        crate::phase::unprovided(
            "BackendMut::level_spawn_item",
            "level_spawn_item",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `spawn_item` with the item entity's motion set to
    /// `motion` (blocks a tick), as `new ItemEntity(level, x, y, z, stack,
    /// deltaX, deltaY, deltaZ)`, and the same `pickup_delay` ticks before it
    /// can be picked up (`ItemEntity.setPickupDelay`). None in the same
    /// cases, and for a motion that isn't finite.
    fn level_spawn_item_with_motion(
        &mut self,
        level: LevelRef,
        x: f64,
        y: f64,
        z: f64,
        stack: ItemStack,
        motion: [f64; 3],
        pickup_delay: u32,
    ) -> Option<EntityRef> {
        crate::phase::unprovided(
            "BackendMut::level_spawn_item_with_motion",
            "level_spawn_item_with_motion",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Awards `amount` experience at `x y z` as vanilla's `ExperienceOrb.award`
    /// does: orbs of the largest sizes first (2477, 1237, ... 3, 1), each
    /// drawing the id `tryMergeToExisting` compares from the level random and
    /// merging into an orb of the same value whose box meets the 1 block box
    /// around `x y z`, or else a new orb with its own random motion. The
    /// `block_drops` rule doesn't apply (vanilla's blocks check it before
    /// they award). True once the orbs are awarded, even if a mod keeps a new
    /// one out (`native_entity_join_level`). False, with no orbs, for 0, a
    /// value that isn't finite, on a client, for a stale handle or a level
    /// not in place.
    fn level_award_experience(
        &mut self,
        level: LevelRef,
        x: f64,
        y: f64,
        z: f64,
        amount: u32,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::level_award_experience",
            "level_award_experience",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
