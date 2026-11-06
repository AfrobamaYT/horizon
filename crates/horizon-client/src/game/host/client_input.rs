//! The game's side of the `client_input` area (`horizon-mod-api/src/host/client_input.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl ClientInputClientHost for Game<'_> {
    fn claim_items(&mut self, mod_id: &str, ids: &[String]) {
        mod_api::__private::check_access("claim_items", mod_api::__private::LoadAccess::WorldOnly);
        self.native_claim_items(mod_id, ids)
    }
    fn poll(&mut self, mod_id: &str) -> Vec<NativeModEvent> {
        mod_api::__private::check_access("poll", mod_api::__private::LoadAccess::WorldOnly);
        self.native_poll(mod_id)
    }
    fn cursor(&self) -> [f32; 2] {
        mod_api::__private::check_access("cursor", mod_api::__private::LoadAccess::ClientSetup);
        self.native_cursor()
    }
    fn mod_key(&self, mod_id: &str, name: &str) -> Option<(String, ClientModifiers)> {
        mod_api::__private::check_access("mod_key", mod_api::__private::LoadAccess::ClientSetup);
        self.native_mod_key(mod_id, name)
    }
    fn playing(&self) -> bool {
        mod_api::__private::check_access("playing", mod_api::__private::LoadAccess::ClientSetup);
        self.native_playing()
    }
    fn mapping(&self, name: &str) -> Option<NativeKeyMapping> {
        mod_api::__private::check_access("mapping", mod_api::__private::LoadAccess::ClientSetup);
        self.native_mapping(name)
    }
    fn modifiers(&self) -> ClientModifiers {
        mod_api::__private::check_access("modifiers", mod_api::__private::LoadAccess::ClientSetup);
        self.native_modifiers()
    }
    fn clipboard_text(&self) -> Option<String> {
        mod_api::__private::check_access(
            "clipboard_text",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_clipboard_text()
    }
    fn set_clipboard_text(&self, text: &str) -> bool {
        mod_api::__private::check_access(
            "set_clipboard_text",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_set_clipboard_text(text)
    }
}
