//! A mod crate's view of the mod API: `Hooks` is implemented with the names `mod_api` exports.
//! The test is that this file builds with the exported event types.
use horizon_client::mod_api::{self, Hooks, Host, InputKeyEvent, ModEntry, winit};

#[derive(Default)]
struct Keys;

#[mod_api::horizon_mod]
impl Hooks for Keys {
    fn input_key(&self, _host: &mut dyn Host, event: &mut InputKeyEvent<'_>) {
        let _: &winit::event::KeyEvent = event.event;
    }
}

#[test]
fn a_mod_implements_the_key_hooks() {
    let _: ModEntry = ModEntry::new("keys", hooks());
}
