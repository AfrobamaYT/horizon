//! The `client_session` area: host methods the game implements (`game/host/client_session.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' client-side reads of the `client_session` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait ClientSessionClientBackend {
    /// Side: client.
    /// The local player's predicted breaking state, including Idle. None for
    /// stale, nonlocal or fake handles and outside a logged-in client world.
    fn player_block_breaking(&self, player: PlayerRef) -> Option<BlockBreakingState> {
        crate::phase::unprovided(
            "Backend::player_block_breaking",
            "player_block_breaking",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// The local player's existing cave ambience mood, normally 0 through 1.
    /// None for stale, nonlocal or fake handles and outside a client world.
    fn player_ambient_mood(&self, player: PlayerRef) -> Option<f32> {
        crate::phase::unprovided(
            "Backend::player_ambient_mood",
            "player_ambient_mood",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
