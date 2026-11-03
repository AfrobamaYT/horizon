//! A mod crate's view of the mod API: `Hooks` is implemented with the names `mod_api` exports.
//! The test is that this file builds with the exported event types.
use horizon_client::mod_api::{self, ClientHost, Hooks, InputKeyEvent, ModEntry, winit};

#[derive(Default)]
struct Keys;

#[mod_api::horizon_mod]
impl Hooks for Keys {
    fn input_key(&self, host: &mut dyn ClientHost, event: &mut InputKeyEvent<'_>) {
        let _: &winit::event::KeyEvent = event.event;
        if let Some(menu) = host.client_menu() {
            let _ = menu.client_slot_count(host);
            let _ = menu.client_slot(host, 0);
            let _ = menu.client_carried(host);
        }
    }
}

#[test]
fn a_mod_implements_the_key_hooks() {
    let _: ModEntry = ModEntry::new("keys", hooks());
}
