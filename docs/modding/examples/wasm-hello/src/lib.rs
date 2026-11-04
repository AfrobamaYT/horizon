//! A mod built to `mod.wasm` (`cargo horizon-mod build --wasm`): the same source builds as a
//! native mod too.
// ANCHOR: mod
use mod_api::{Flow, Hooks, ServerChatEvent, ServerHost, Text};

#[derive(Default)]
pub struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn server_chat(&self, _host: &mut dyn ServerHost, event: &mut ServerChatEvent<'_>) -> Flow {
        *event.message = Text::literal(format!("{} (via mod.wasm)", event.raw_text));
        Flow::Continue
    }
}
// ANCHOR_END: mod
