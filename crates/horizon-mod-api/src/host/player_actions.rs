//! The `player_actions` area: host methods the game implements (`game/host/player_actions.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' server writes of the `player_actions` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait PlayerActionsBackendMut {
    /// Side: server.
    /// A fake player acts as a player does, through the game's own path for the action
    /// (`ServerGamePacketListenerImpl.handleUseItemOn` / `handleUseItem` / `handleInteract`'s
    /// ATTACK, and `ServerPlayerGameMode.destroyBlock`), as its packet would run it: reach,
    /// protection and the border, then the action. Once it runs, an effect a fake can't take
    /// part in (a menu, sleep, a ride, another dimension) is stopped where it would start, and
    /// the answer says which; the action's own side effects (the swing, the hooks before it,
    /// vanilla's result) have still happened then. The fake is found once, and its turn
    /// starts at that place in the list.
    fn player_perform_action(
        &mut self,
        player: PlayerRef,
        action: PlayerAction,
    ) -> Result<ActionOutcome, ActionRefused> {
        crate::phase::unprovided(
            "BackendMut::player_perform_action",
            "player_perform_action",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// A machine's one use on the persistent actor, with its private state put back even
    /// when the action refuses. Only the existing guest turn runs gameplay and its hooks.
    fn player_perform_action_scoped(
        &mut self,
        player: PlayerRef,
        action: PlayerAction,
        setup: PlayerActionSetup,
    ) -> Result<ScopedActionResult, ActionRefused> {
        crate::phase::unprovided(
            "BackendMut::player_perform_action_scoped",
            "player_perform_action_scoped",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
