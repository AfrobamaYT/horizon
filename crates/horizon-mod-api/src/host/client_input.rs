//! The `client_input` area: host methods the game implements (`game/host/client_input.rs`).
#![allow(unused_variables)]
use crate::*;
/// The client host's methods of the `client_input` area, implemented by the game and reached through [`ClientHost`].
pub trait ClientInputClientHost {
    /// Side: client.
    /// Right-clicks with these items (at most 64 ids) are mod `mod_id`'s; a later call replaces the list.
    fn claim_items(&mut self, mod_id: &str, ids: &[String]) {
        crate::phase::unprovided(
            "ClientHost::claim_items",
            "claim_items",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// Mod `mod_id`'s events since its last poll (at most 256 wait).
    fn poll(&mut self, mod_id: &str) -> Vec<NativeModEvent> {
        crate::phase::unprovided(
            "ClientHost::poll",
            "poll",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// Where the mouse is, in GUI pixels.
    fn cursor(&self) -> [f32; 2] {
        crate::phase::unprovided(
            "ClientHost::cursor",
            "cursor",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// The saved key and modifiers for an owner-bound registration, or its declared default.
    fn mod_key(&self, mod_id: &str, name: &str) -> Option<(String, ClientModifiers)> {
        crate::phase::unprovided(
            "ClientHost::mod_key",
            "mod_key",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
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
    /// Side: client.
    /// The clipboard's text (`KeyboardHandler.getClipboard`), empty when it holds none,
    /// while the game handles a key, mouse or typed-text event: a mod's key or screen hook
    /// pasting into its own field. None at any other moment (a tick, a frame, a packet),
    /// so no mod reads the clipboard without the player doing something, and None on a
    /// server.
    fn clipboard_text(&self) -> Option<String> {
        crate::phase::unprovided(
            "ClientHost::clipboard_text",
            "clipboard_text",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
    /// Side: client.
    /// Puts `text` on the clipboard (`KeyboardHandler.setClipboard`) on the same terms as
    /// `native_clipboard_text` reads it. False, with nothing copied, at any other moment or
    /// on a server; empty text copies nothing, as vanilla's.
    fn set_clipboard_text(&self, text: &str) -> bool {
        crate::phase::unprovided(
            "ClientHost::set_clipboard_text",
            "set_clipboard_text",
            crate::phase::LoadAccess::ClientSetup,
        )
    }
}
