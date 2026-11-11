use super::*;
use mod_api::{ClientHost, Hooks, ModEntry, Mods, RenderTooltipGatherComponentsEvent};

struct Edit(bool);
#[mod_api::__subscriptions]
impl Hooks for Edit {
    fn render_tooltip_gather_components(
        &self,
        _: &dyn ClientHost,
        e: &mut RenderTooltipGatherComponentsEvent<'_>,
    ) -> mod_api::Flow {
        assert!(matches!(
            &e.elements[1],
            TooltipElement::Image(TooltipImage {
                source: mod_api::TooltipImageSource::Bundle(_),
                ..
            })
        ));
        let image = e.elements.remove(1);
        if self.0 {
            e.elements.push(image);
        }
        mod_api::Flow::Continue
    }
}

fn run(mode: Option<bool>, test: impl FnOnce(&mut Game<'_>)) {
    let mods: Mods = mode
        .map(|mode| ModEntry::new("bundle-test", Box::new(Edit(mode))))
        .into_iter()
        .collect::<Vec<_>>()
        .into();
    crate::game::api_values::install_parts(&mods).unwrap();
    mod_api::testing::with_mods(mods, || {
        crate::game::native_entity_refs::tests::with_game(|game| {
            game.gui.atlas.sprites.insert(
                "gui/sprites/container/bundle/slot_background".into(),
                horizon_client::hud::Rect {
                    u0: 0.25,
                    v0: 0.25,
                    u1: 0.5,
                    v1: 0.5,
                    w: 24,
                    h: 24,
                },
            );
            test(game);
        })
    });
}
fn bundle() -> ItemStack {
    let mut stack = ItemStack::new("minecraft:bundle", 1);
    stack.data_mut().bundle_contents = vec![ItemStack::new("minecraft:stone", 1)];
    stack
}
fn draw(game: &mut Game<'_>, stack: &ItemStack) -> HudBatch {
    let mut batch = HudBatch::scratch([320, 240]);
    game.draw_stack_tooltip_with(
        &mut batch,
        stack,
        false,
        10.0,
        20.0,
        vec![(horizon_client::text::Text::literal("Rest"), u32::MAX)],
    );
    batch
}
fn grid(game: &Game<'_>, batch: &HudBatch, y: f32, present: bool) {
    // Positioner: x + 12, y - 12. Slot right-aligned: 22 + 72 = 94.
    let mut expected = HudBatch::scratch([320, 240]);
    horizon_client::ui::nine_slice(
        &mut expected,
        &game.gui.atlas,
        "gui/sprites/container/bundle/slot_background",
        94.0,
        y,
        24.0,
        24.0,
        u32::MAX,
    );
    assert!(!expected.vertices.is_empty());
    let found = batch
        .vertices
        .windows(expected.vertices.len())
        .any(|window| {
            window
                .iter()
                .zip(&expected.vertices)
                .all(|(a, b)| a.pos == b.pos && a.uv == b.uv && a.color == b.color)
        });
    assert_eq!(found, present);
}
fn bottom(batch: &HudBatch) -> f32 {
    batch.vertices.iter().map(|v| v.pos[1]).fold(0.0, f32::max)
}
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn bundle_no_listener_order_and_draw_position() {
    run(None, |game| {
        let stack = bundle();
        let lines = vec![
            (horizon_client::text::Text::literal("Name"), u32::MAX),
            (horizon_client::text::Text::literal("Rest"), u32::MAX),
        ];
        let elements = game
            .gather_tooltip_components(&stack, lines, vec![], (320.0, 240.0))
            .unwrap();
        assert!(matches!(
            &elements[..],
            [
                Element::Text(..),
                Element::Image(TooltipImage {
                    source: mod_api::TooltipImageSource::Bundle(_),
                    size: [96.0, 45.0]
                }),
                Element::Text(..)
            ]
        ));
        let batch = draw(game, &stack);
        // top 8 + name 10 + first-component padding 2 = 20.
        grid(game, &batch, 20.0, true);
    });
}
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn bundle_gather_moves_image_to_end() {
    run(Some(true), |game| {
        let batch = draw(game, &bundle());
        // top 8 + name 10 + padding 2 + rest 10 = 30.
        grid(game, &batch, 30.0, true);
        grid(game, &batch, 20.0, false);
    });
}
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn bundle_gather_removes_grid_and_height() {
    let mut full_bottom = 0.0;
    run(None, |game| {
        full_bottom = bottom(&draw(game, &bundle()));
    });
    run(Some(false), |game| {
        let batch = draw(game, &bundle());
        grid(game, &batch, 20.0, false);
        // One 24px row + 13px bar + 8px gap = 45px.
        assert_eq!(full_bottom - bottom(&batch), 45.0);
    });
}
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn non_bundle_keeps_text_only_geometry() {
    run(None, |game| {
        let stack = ItemStack::new("minecraft:stick", 1);
        let batch = draw(game, &stack);
        grid(game, &batch, 20.0, false);
        assert_eq!(game.stack_tooltip_lines(&stack, false, false).len(), 1);
        // Two 10px lines: top 8, content height 20, outer border extends through y + h + 4.
        assert_eq!(bottom(&batch), 32.0);
    });
}
