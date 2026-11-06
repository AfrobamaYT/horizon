//! The `client_input_reads` area: host methods the game implements (`game/host/client_input_reads.rs`).
#![allow(unused_variables)]
use crate::*;
/// The client host's methods of the `client_input_reads` area, implemented by the game and reached through [`ClientHost`].
pub trait ClientInputReadsClientHost {
    /// Side: client.
    /// Playing, with nothing over the world: keys and zoom apply.
    fn playing(&self) -> bool {
        crate::phase::unprovided(
            "ClientHost::playing",
            "playing",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// A vanilla mapping (`key.forward`, `key.jump`, `key.attack`…): whether
    /// it is held (`KeyMapping.isDown`), and its key's name as Controls shows
    /// it. None for a mapping it doesn't answer for.
    fn mapping(&self, name: &str) -> Option<NativeKeyMapping> {
        crate::phase::unprovided(
            "ClientHost::mapping",
            "mapping",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// The current window modifier keys in the same GLFW encoding as input hooks.
    /// Cleared when the window loses focus; available even outside a world.
    fn modifiers(&self) -> ClientModifiers {
        crate::phase::unprovided(
            "ClientHost::modifiers",
            "modifiers",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
}
