//! The `client_input_options` area: host methods the game implements (`game/host/client_input_options.rs`).
#![allow(unused_variables)]
use crate::*;
/// The client host's methods of the `client_input_options` area, implemented by the game and reached through [`ClientHost`].
pub trait ClientInputOptionsClientHost {
    /// Side: client.
    /// A snapshot of the current input preferences.
    fn input_options(&self) -> ClientInputOptions {
        crate::phase::unprovided(
            "ClientHost::input_options",
            "input_options",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
}
