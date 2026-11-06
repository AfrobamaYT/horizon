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
fn client_packet_hands_a_mod_the_type_and_custom_payload() {
    use mod_api::{
        ClientPacket, ClientPacketEvent, ClientPayload, Host, Identifier, dispatch_client_packet,
    };
    use std::sync::Mutex;
    static HEARD: Mutex<Vec<(String, Vec<u8>)>> = Mutex::new(Vec::new());
    struct TestHost;
    impl Host for TestHost {}
    impl ClientHost for TestHost {}
    // A plugin-channel listener using only the mod API's plain data.
    #[derive(Default)]
    struct Channels;
    #[mod_api::horizon_mod]
    impl Hooks for Channels {
        fn client_packet(&self, _host: &mut dyn ClientHost, event: &mut ClientPacketEvent<'_>) {
            if let Some(p) = &event.packet.custom_payload {
                HEARD
                    .lock()
                    .unwrap()
                    .push((p.channel.clone(), p.data.clone()));
            }
        }
    }
    mod_api::testing::with_mods(vec![ModEntry::new("channels", hooks())].into(), || {
        dispatch_client_packet(
            &mut TestHost,
            &ClientPacket::new(Identifier::parse("minecraft:ping").unwrap(), None),
        );
        let payload = ClientPayload::new("voicechat:secret".into(), vec![1, 2, 3]);
        dispatch_client_packet(
            &mut TestHost,
            &ClientPacket::new(
                Identifier::parse("minecraft:custom_payload").unwrap(),
                Some(payload),
            ),
        );
    });
    assert_eq!(
        *HEARD.lock().unwrap(),
        [("voicechat:secret".to_string(), vec![1, 2, 3])]
    );
}

// Fluid acceptance is scoped to a FLUIDS provider, not a global Hooks method.
// Its default/refused/accepted writes are covered with a real Game backend in
// game/machines/native_fluids.rs::tests::a_scoped_tank_can_refuse_a_fluid.

mod model_layers {
    use mod_api::{
        CubeListBuilder, Hooks, LayerDefinition, LayerDefinitionsEvent, MeshDefinition, ModEntry,
        PartPose, RegistrationContext,
    };

    struct Golem(&'static str);
    #[mod_api::__subscriptions]
    impl Hooks for Golem {
        // ANCHOR: layer_definitions
        fn layer_definitions(
            &self,
            _context: &RegistrationContext,
            event: &mut LayerDefinitionsEvent<'_>,
        ) {
            let mut mesh = MeshDefinition::new();
            let root = mesh.root();
            let head = CubeListBuilder::create()
                .tex_offs(0, 0)
                .add_box([-4.0, -8.0, -4.0], [8.0, 8.0, 8.0]);
            mesh.add_or_replace_child(root, "head", head, PartPose::offset(0.0, 6.0, 0.0));
            event
                .layers
                .push((self.0.into(), LayerDefinition::create(mesh, 64, 32)));
        }
        // ANCHOR_END: layer_definitions
    }

    #[test]
    fn layer_definitions_collect_each_mods_layers_in_order() {
        // The harness supplies the engine-owned context; the listener above uses only public API.
        use mod_api::__private::game::RegistrationContextNew;
        mod_api::testing::with_mods(
            vec![
                ModEntry::new("first", Box::new(Golem("first:golem#main"))),
                ModEntry::new("second", Box::new(Golem("second:golem#main"))),
            ]
            .into(),
            || {
                let mut layers = Vec::new();
                mod_api::dispatch_layer_definitions(&RegistrationContext::new(), &mut layers);
                let ids: Vec<_> = layers.iter().map(|(id, _)| id.as_str()).collect();
                assert_eq!(ids, ["first:golem#main", "second:golem#main"]);
                let model = layers[0].1.bake_root();
                let head = model.get_child("head").expect("the head part");
                assert_eq!((head.y, head.polygons.len()), (6.0, 6));
            },
        );
    }
}

#[test]
fn tooltip_render_pre_intercepts_rendering_and_draws_gui_items() {
    use horizon_client::hud::HudBatch;
    use horizon_client::items::ItemStack;
    use horizon_client::mod_api::{
        Flow, KeyCode, TooltipRenderPreEvent, dispatch_tooltip_render_pre,
    };
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    struct TestHost {
        draw_count: AtomicU32,
        with_text_seen: AtomicBool,
    }
    impl Host for TestHost {
        fn gui_draw_item(&self, _batch: &mut HudBatch, stack: &ItemStack, at: [f32; 2]) {
            if &*stack.item == "minecraft:shulker_box" && at == [16.0, 32.0] {
                self.draw_count.fetch_add(1, Ordering::SeqCst);
            }
        }
        fn gui_draw_item_with(
            &self,
            _batch: &mut HudBatch,
            stack: &ItemStack,
            at: [f32; 2],
            text: Option<&str>,
        ) {
            if &*stack.item == "minecraft:diamond" && at == [20.0, 40.0] && text == Some("5") {
                self.with_text_seen.store(true, Ordering::SeqCst);
            }
        }
        fn has_shift_down(&self) -> bool {
            true
        }
        fn has_control_down(&self) -> bool {
            false
        }
        fn has_alt_down(&self) -> bool {
            false
        }
        fn is_key_down(&self, key: KeyCode) -> bool {
            key == KeyCode::ShiftLeft
        }
    }

    struct ShulkerPreviewMod;
    impl Hooks for ShulkerPreviewMod {
        fn tooltip_render_pre(
            &self,
            host: &mut dyn Host,
            event: &mut TooltipRenderPreEvent<'_>,
        ) -> Flow {
            if &*event.stack.item == "minecraft:shulker_box"
                && host.has_shift_down()
                && host.is_key_down(KeyCode::ShiftLeft)
            {
                host.gui_draw_item(event.batch, event.stack, [event.x, event.y]);
                let inner = ItemStack::new("minecraft:diamond", 5);
                host.gui_draw_item_with(event.batch, &inner, [20.0, 40.0], Some("5"));
                return Flow::Cancel;
            }
            Flow::Continue
        }
    }

    let mut host = TestHost {
        draw_count: AtomicU32::new(0),
        with_text_seen: AtomicBool::new(false),
    };
    let mods: Vec<Box<dyn Hooks>> = vec![Box::new(ShulkerPreviewMod)];
    let shulker = ItemStack::new("minecraft:shulker_box", 1);
    assert_eq!(shulker.container().len(), 0);
    assert!(shulker.custom_data().is_none());

    let mut batch = HudBatch::new([800, 600]);
    let mut cancel = false;
    dispatch_tooltip_render_pre(
        &mut host,
        &mods,
        &shulker,
        &mut batch,
        16.0,
        32.0,
        &mut cancel,
    );

    assert!(cancel, "tooltip_render_pre cancelled vanilla tooltip");
    assert_eq!(
        host.draw_count.load(Ordering::SeqCst),
        1,
        "gui_draw_item was called"
    );
    assert!(
        host.with_text_seen.load(Ordering::SeqCst),
        "gui_draw_item_with was called"
    );

    let stone = ItemStack::new("minecraft:stone", 64);
    let mut cancel_stone = false;
    dispatch_tooltip_render_pre(
        &mut host,
        &mods,
        &stone,
        &mut batch,
        16.0,
        32.0,
        &mut cancel_stone,
    );
    assert!(
        !cancel_stone,
        "unhandled stack continues to vanilla tooltip"
    );
}
