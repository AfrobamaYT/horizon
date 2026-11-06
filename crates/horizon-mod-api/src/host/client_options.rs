//! The `client_options` area: host methods the game implements (`game/host/client_options.rs`).
#![allow(unused_variables)]
use crate::*;
/// The client host's methods of the `client_options` area, implemented by the game and reached through [`crate::ClientHost`].
pub trait ClientOptionsClientHost {
    /// Side: client.
    /// A snapshot of the current input preferences.
    fn input_options(&self) -> ClientInputOptions {
        crate::phase::unprovided(
            "ClientHost::input_options",
            "input_options",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// A snapshot of the current graphics preferences.
    fn graphics_options(&self) -> ClientGraphicsOptions {
        crate::phase::unprovided(
            "ClientHost::graphics_options",
            "graphics_options",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// A snapshot of the current GUI and accessibility preferences.
    fn gui_options(&self) -> ClientGuiOptions {
        crate::phase::unprovided(
            "ClientHost::gui_options",
            "gui_options",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// A snapshot of the current chat preferences.
    fn chat_options(&self) -> ClientChatOptions {
        crate::phase::unprovided(
            "ClientHost::chat_options",
            "chat_options",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// A snapshot of the current audio preferences.
    fn audio_options(&self) -> ClientAudioOptions {
        crate::phase::unprovided(
            "ClientHost::audio_options",
            "audio_options",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
}
