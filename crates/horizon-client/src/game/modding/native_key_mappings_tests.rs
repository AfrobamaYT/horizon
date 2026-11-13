//! A mod's registered mapping read through the client host, fed by the control socket as by the
//! window: a tap is one click, each repeat one more, a screen opening drops them, a binding's
//! modifier must be held, and a mouse press counts for a mapping on its button.
use super::event_loop::EventLoopCtl;
use super::{Game, Screen};
use crate::game::native_entity_refs::tests::{run_fresh_cases, with_client_game};
use horizon_client::keymapping::KeyAction;
use mod_api::prelude::ClientInputClientHost;
use mod_api::*;
use winit::{event_loop::ControlFlow, keyboard::KeyCode};

struct NoLoop;
impl EventLoopCtl for NoLoop {
    fn exit(&self) {}
    fn set_control_flow(&self, _: ControlFlow) {}
}

/// Registers `zoom` on Z, `boost` on Shift + Z and `grab` on the left mouse button.
struct Keys;

#[mod_api::__subscriptions]
impl Hooks for Keys {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        r.register_key_mapping(KeyRegistration::new("zoom", "key.keyboard.z"));
        r.register_key_mapping(
            KeyRegistration::new("boost", "key.keyboard.z").modifiers(ClientModifiers::new(1)),
        );
        r.register_key_mapping(KeyRegistration::new("grab", "key.mouse.left"));
    }
}

fn down(game: &Game<'_>, id: &str) -> bool {
    ClientInputClientHost::is_down(game, "keys", id)
}
fn click(game: &mut Game<'_>, id: &str) -> bool {
    ClientInputClientHost::consume_click(game, "keys", id)
}
fn line(game: &mut Game<'_>, line: &str) {
    game.replay_line(line, &NoLoop);
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_control_socket_key_counts_as_a_window_key() {
    if run_fresh_cases(
        "game::native_key_mappings_tests::a_control_socket_key_counts_as_a_window_key",
        "HORIZON_MOD_KEYS_CHILD",
        &["game"],
    ) {
        return;
    }
    let mods: Mods = vec![ModEntry::new("keys", Box::new(Keys))].into();
    mod_platform::__private::install(mods.clone());
    crate::game::api_values::install_parts(&mods).unwrap();
    with_client_game(|game| {
        game.screen = Screen::Playing;
        game.track_toggle_screens();

        // A tap: one click, consumed once; held only while down.
        line(game, "key z tap");
        assert!(!down(game, "zoom"));
        assert!(click(game, "zoom"));
        assert!(!click(game, "zoom"));
        // The window's tap does the same.
        game.mod_mappings_key(KeyCode::KeyZ, KeyAction::Press);
        assert!(down(game, "zoom"));
        game.mod_mappings_key(KeyCode::KeyZ, KeyAction::Release);
        assert!(!down(game, "zoom"));
        assert!(click(game, "zoom"));
        assert!(!click(game, "zoom"));

        // Held with two repeats: three clicks.
        line(game, "key z down");
        line(game, "key z repeat");
        line(game, "key z repeat");
        assert!(down(game, "zoom"));
        line(game, "key z up");
        assert!(!down(game, "zoom"));
        let clicks: Vec<bool> = (0..4).map(|_| click(game, "zoom")).collect();
        assert_eq!(clicks, [true, true, true, false]);

        // A screen opening drops the clicks and lets go; a press behind it counts nothing.
        line(game, "key z down");
        line(game, "key z repeat");
        game.screen = Screen::Inventory;
        game.track_toggle_screens();
        assert!(!down(game, "zoom"));
        assert!(!click(game, "zoom"));
        line(game, "key z tap");
        assert!(!click(game, "zoom"));
        game.screen = Screen::Playing;
        game.track_toggle_screens();

        // Shift + Z counts only with shift held; Z alone counts with it too.
        line(game, "key z tap");
        assert!(!click(game, "boost"));
        line(game, "key shift down");
        line(game, "key z tap");
        line(game, "key shift up");
        assert!(click(game, "boost"));
        assert!(!click(game, "boost"));
        let clicks: Vec<bool> = (0..3).map(|_| click(game, "zoom")).collect();
        assert_eq!(clicks, [true, true, false]);

        // A mouse press counts for a mapping on its button.
        line(game, "mouse left click");
        assert!(click(game, "grab"));
        assert!(!click(game, "grab"));

        // Another mod's id, or an unregistered one, has nothing.
        assert!(!ClientInputClientHost::consume_click(game, "other", "zoom"));
        assert!(!click(game, "missing"));
    });
}
