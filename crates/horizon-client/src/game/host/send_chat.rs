//! The game's side of the `send_chat` area (`horizon-mod-api/src/host/send_chat.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl SendChatClientHost for Game<'_> {
    fn send_chat(&mut self, message: &str) -> Result<(), String> {
        mod_api::__private::check_access("send_chat", mod_api::__private::LoadAccess::WorldOnly);
        self.native_send_chat(message)
    }
}
