//! A mod crate's view of the mod API: `Hooks` is implemented with the names `mod_api` exports.
//! The test is that this file builds with the exported event types.
use mod_api::{
    self, ClientHost, Hooks, InputKeyEvent, KeyAction, ModEntry, ScreenExclusionAreasEvent,
};

#[derive(Default)]
struct Keys;

#[mod_api::horizon_mod]
impl Hooks for Keys {
    fn input_key(&self, host: &mut dyn ClientHost, event: &mut InputKeyEvent<'_>) {
        let _: (&str, KeyAction) = (event.key, event.action);
        if let Some(menu) = host
            .local_player()
            .and_then(|player| player.container_menu(host))
        {
            let _ = menu.slot_count(host);
            let _ = menu.slot(host, 0);
            let _ = menu.carried(host);
        }
    }
}

#[test]
fn a_mod_implements_the_key_hooks() {
    let _: ModEntry = ModEntry::new("keys", hooks());
}

mod sidebar {
    use super::*;

    #[derive(Default)]
    struct Sidebar;

    #[mod_api::horizon_mod]
    impl Hooks for Sidebar {
        fn screen_exclusion_areas(
            &self,
            _host: &dyn ClientHost,
            event: &mut ScreenExclusionAreasEvent<'_>,
        ) {
            if event.screen.id == "inventory" {
                event.areas.push([0.0, 0.0, 40.0, event.screen.rect[3]]);
            }
        }
    }

    #[test]
    fn a_mod_adds_screen_exclusion_areas() {
        let _: ModEntry = ModEntry::new("sidebar", hooks());
    }
}

#[test]
fn received_chat_is_replaced_in_order_and_cancel_stops_later_mods() {
    use horizon_client::mod_api::{ClientChatReceivedEvent, Flow, dispatch_client_chat_received};
    use horizon_client::text::Text;
    struct TestHost;
    impl Host for TestHost {}
    struct Tag;
    impl Hooks for Tag {
        fn client_chat_received(
            &self,
            _host: &mut dyn Host,
            event: &mut ClientChatReceivedEvent<'_>,
        ) -> Flow {
            if event.message.plain().contains("drop") {
                return Flow::Cancel;
            }
            let who = if event.system { "server" } else { "player" };
            *event.message = Text::literal(format!("[{who}] {}", event.message.plain()));
            Flow::Continue
        }
    }
    struct Shout;
    impl Hooks for Shout {
        fn client_chat_received(
            &self,
            _host: &mut dyn Host,
            event: &mut ClientChatReceivedEvent<'_>,
        ) -> Flow {
            *event.message = Text::literal(event.message.plain().to_uppercase());
            Flow::Continue
        }
    }
    let mods: Vec<Box<dyn Hooks>> = vec![Box::new(Tag), Box::new(Shout)];
    let receive = |system: bool, text: &str| {
        let (mut message, mut cancel) = (Text::literal(text), false);
        let chat_type = (!system).then_some("minecraft:chat");
        dispatch_client_chat_received(
            &mut TestHost,
            &mods,
            system,
            false,
            7,
            chat_type,
            &mut message,
            &mut cancel,
        );
        (message.plain(), cancel)
    };
    assert_eq!(receive(true, "hi"), ("[SERVER] HI".to_owned(), false));
    assert_eq!(receive(false, "hi"), ("[PLAYER] HI".to_owned(), false));
    // A cancelled line keeps its text: the second mod never ran.
    assert_eq!(receive(false, "drop me"), ("drop me".to_owned(), true));
}
