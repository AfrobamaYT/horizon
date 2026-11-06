//! The `living_writes` area: host methods the game implements (`game/host/living_writes.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `living_writes` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait LivingWritesBackend {
    /// Side: server.
    /// `LivingEntity.getUseItemRemainingTicks`: the ticks left of the item a
    /// player uses, 0 when they use none. None for the other kinds (a mob's
    /// item use is a counter of its goal, not this timer).
    fn living_item_use_remaining(&self, living: LivingRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::living_item_use_remaining",
            "living_item_use_remaining",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    ///
    fn living_used_item_hand(&self, living: LivingRef) -> Option<Hand> {
        crate::phase::unprovided(
            "Backend::living_used_item_hand",
            "living_used_item_hand",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `living_writes` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait LivingWritesBackendMut {
    /// Side: server.
    /// `LivingEntity.swing(hand, DEFAULT, sendToSwingingEntity)`: starts the
    /// arm swing (shortened by haste, lengthened by mining fatigue) unless
    /// one is under way and past its first half, and tells the players
    /// around at the end of the tick, and the swinger too when
    /// `send_to_self`. True when a swing started. Mobs and players; false
    /// for armor stands and the dragon, and for a player other than the
    /// acting one while another player acts.
    fn living_swing(&mut self, living: LivingRef, hand: Hand, send_to_self: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_swing",
            "living_swing",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LivingEntity.getArrowCount`: the arrows stuck in a player. None for
    /// the other kinds, which Horizon keeps no count for.
    fn living_arrow_count(&mut self, living: LivingRef) -> Option<i32> {
        crate::phase::unprovided(
            "BackendMut::living_arrow_count",
            "living_arrow_count",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LivingEntity.setArrowCount`: players around see the arrows (vanilla
    /// does not limit the count). False for the other kinds.
    fn living_set_arrow_count(&mut self, living: LivingRef, count: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_set_arrow_count",
            "living_set_arrow_count",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LivingEntity.getStingerCount`: the bee stingers stuck in a player.
    /// None for the other kinds.
    fn living_stinger_count(&mut self, living: LivingRef) -> Option<i32> {
        crate::phase::unprovided(
            "BackendMut::living_stinger_count",
            "living_stinger_count",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LivingEntity.setStingerCount`: players around see the stingers. False
    /// for the other kinds.
    fn living_set_stinger_count(&mut self, living: LivingRef, count: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_set_stinger_count",
            "living_set_stinger_count",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LivingEntity.removeArrowTime`: the ticks until the next arrow falls
    /// out of a player; at 0 or less it restarts at `20 * (30 - arrows)`.
    /// False for the other kinds.
    fn living_set_arrow_removal_ticks(&mut self, living: LivingRef, ticks: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_set_arrow_removal_ticks",
            "living_set_arrow_removal_ticks",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `LivingEntity.removeStingerTime`, as `set_arrow_removal_ticks` for the
    /// stingers.
    fn living_set_stinger_removal_ticks(&mut self, living: LivingRef, ticks: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_set_stinger_removal_ticks",
            "living_set_stinger_removal_ticks",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets `LivingEntity.useItemRemaining` of the item a player uses (the
    /// way a mod's tick moves the end of the use): at 1 the use completes on
    /// the next tick, at 0 or less it runs until the player lets go, as in
    /// vanilla. It moves the deadline the use-item Tick hook moves, so only
    /// the uses that count it (food, drink, mod items) follow it: shield, bow,
    /// trident, spear, instrument and bundle uses ignore it, and a call from
    /// inside the Tick hook itself is overwritten by that hook's result. False
    /// when the player uses nothing, and for the other kinds.
    fn living_set_item_use_remaining(&mut self, living: LivingRef, ticks: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_set_item_use_remaining",
            "living_set_item_use_remaining",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    ///
    fn living_start_using_item(&mut self, living: LivingRef, hand: Hand) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_start_using_item",
            "living_start_using_item",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    ///
    fn living_release_using_item(&mut self, living: LivingRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::living_release_using_item",
            "living_release_using_item",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
