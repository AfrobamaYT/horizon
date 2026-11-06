//! The `dragon_refs` area: host methods the game implements (`game/host/dragon_refs.rs`).
#![allow(unused_variables)]
use crate::*;
/// The server host's methods of the `dragon_refs` area, implemented by the game and reached through [`ServerHost`].
pub trait DragonRefsServerHost {
    /// Side: server.
    /// The entity as the ender dragon, or None for another kind, a stale
    /// handle or a client.
    fn dragon(&self, entity: EntityRef) -> Option<DragonRef> {
        crate::phase::unprovided(
            "ServerHost::dragon",
            "dragon",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' reads of the `dragon_refs` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait DragonRefsBackend {
    /// Side: server.
    /// `EnderDragonPhaseManager.getCurrentPhase`.
    fn dragon_phase(&self, dragon: DragonRef) -> Option<NativeDragonPhase> {
        crate::phase::unprovided(
            "Backend::dragon_phase",
            "dragon_phase",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `EnderDragonFight.aliveCrystals`: the crystals the last scan found
    /// (every 100 ticks). None for a stale handle, a client, or a dragon
    /// outside the End or whose fight is not loaded.
    fn dragon_alive_crystals(&self, dragon: DragonRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::dragon_alive_crystals",
            "dragon_alive_crystals",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `EnderDragonFight.hasPreviouslyKilledDragon`. None as
    /// `alive_crystals` is.
    fn dragon_previously_killed(&self, dragon: DragonRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::dragon_previously_killed",
            "dragon_previously_killed",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' server writes of the `dragon_refs` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait DragonRefsBackendMut {
    /// Side: server.
    /// `EnderDragonPhaseManager.setPhase`: the dragon ends its phase and
    /// begins the new one fresh (targets and counters cleared), and the
    /// clients get the synced `DATA_PHASE`. Setting the phase it holds does
    /// nothing, as in vanilla (a strafing dragon keeps its player). No
    /// random draw. False for a stale handle or a client. The new phase
    /// runs from the dragon's next tick.
    fn dragon_set_phase(&mut self, dragon: DragonRef, phase: NativeDragonPhase) -> bool {
        crate::phase::unprovided(
            "BackendMut::dragon_set_phase",
            "dragon_set_phase",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
