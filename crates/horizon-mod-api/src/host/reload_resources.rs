//! The `reload_resources` area: host methods the game implements (`game/host/reload_resources.rs`).
#![allow(unused_variables)]
use crate::*;
/// The server host's methods of the `reload_resources` area, implemented by the game and reached through [`ServerHost`].
pub trait ReloadResourcesServerHost {
    /// Side: server.
    /// The retained server reload listeners of the last successful reload.
    fn reload_resources(&self) -> &dyn ServerReloadResources {
        crate::phase::unprovided(
            "ServerHost::reload_resources",
            "reload_resources",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
