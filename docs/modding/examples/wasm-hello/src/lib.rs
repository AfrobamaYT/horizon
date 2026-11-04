//! A mod built to `mod.wasm` (`cargo horizon-mod build --wasm`): the same source builds as a
//! native mod too.
// ANCHOR: mod
use mod_api::{
    Flow, Hooks, ItemPart, Registrar, RegistrationContext, ServerChatEvent, ServerHost, Text,
    UseItemEvent,
};

#[derive(Default)]
pub struct Mod;

/// Sticks announce each use.
struct Stick;

#[mod_api::horizon_part]
impl ItemPart for Stick {
    fn use_item(&self, host: &mut dyn ServerHost, event: &mut UseItemEvent<'_>) -> Flow {
        host.broadcast(Text::literal(format!("a {} was used", event.stack.item)));
        Flow::Continue
    }
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn register(&self, registrar: &mut dyn Registrar, _cx: &RegistrationContext<'_>) {
        registrar.item_part("minecraft:stick", Stick);
    }

    fn server_chat(&self, _host: &mut dyn ServerHost, event: &mut ServerChatEvent<'_>) -> Flow {
        *event.message = Text::literal(format!("{} (via mod.wasm)", event.raw_text));
        Flow::Continue
    }
}
// ANCHOR_END: mod
