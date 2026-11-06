//! The game's side of the `clipboard` area (`horizon-mod-api/src/host/clipboard.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl ClipboardClientHost for Game<'_> {
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
