//! The `player_criteria` area: host methods the game implements (`game/host/player_criteria.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' server writes of the `player_criteria` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait PlayerCriteriaBackendMut {
    /// Side: server.
    /// Fires the mod's criteria trigger `trigger` (`mymod:pipe_connected`)
    /// for `player`: vanilla's
    /// `SimpleCriterionTrigger.trigger`. Each criterion of theirs with that
    /// trigger, not yet granted, of an advancement not yet done, is tested
    /// by `test` and then by its `player` predicate; those that pass are
    /// granted, and an advancement they complete is awarded with its toast,
    /// chat line and rewards, as the game's own triggers award.
    ///
    /// The acting player is reached inside their hook; another live player
    /// is reached in their own level, including during a guest's turn. An
    /// outer player displaced by a nested turn is unavailable. `Ok(())`
    /// means the player was reached, whether or not anything was granted.
    /// A stale/offline/displaced handle, a remote client, or a `minecraft:`
    /// trigger or empty id is refused with a typed reason.
    ///
    /// `test` is lent to this call only. A failed test stops further testing
    /// and reports `Test`; earlier matching criteria may already be granted.
    /// Wasm's generated resource route exists, but a callback into the group
    /// currently firing the trigger is refused by the runtime's reentry guard.
    fn player_trigger_criterion(
        &mut self,
        player: PlayerRef,
        trigger: &Identifier,
        test: Arc<dyn CriterionTest>,
    ) -> Result<(), TriggerCriterionError> {
        crate::phase::unprovided(
            "BackendMut::player_trigger_criterion",
            "player_trigger_criterion",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
