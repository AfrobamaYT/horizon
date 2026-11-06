//! Native debug renderers in a real game: what a frame's world geometry holds as a
//! renderer is switched and the world goes.
use super::{execution_role::ExecutionRole, mods, native_entity_refs::tests::with_client_game};
use glam::Mat4;
use horizon_client::{
    debug_entries::{self, Category, DebugEntries, Status},
    world_geometry::WorldGeometry,
};
use mod_api::{
    ClientHost, DebugRenderer, DebugRendererDrawEvent, Hooks, ModEntry,
    RegisterDebugRenderersEvent, WorldDepth, WorldLine,
};
use std::sync::Mutex;

const CAMERA: [f64; 3] = [100.0, 64.0, 100.0];
const SHOWN: &str = "marker:pipes";
const OTHER: &str = "marker:wires";

/// The ids `debug_renderer_draw` was asked about since the last frame.
static ASKED: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct Marker;
#[mod_api::__subscriptions]
impl Hooks for Marker {
    fn register_debug_renderers(
        &self,
        _: &mod_api::RegistrationContext,
        event: &mut RegisterDebugRenderersEvent<'_>,
    ) {
        for id in [SHOWN, OTHER] {
            event.renderers.push(DebugRenderer { id: id.into() });
        }
    }
    fn debug_renderer_draw(&self, _: &dyn ClientHost, event: &mut DebugRendererDrawEvent<'_>) {
        ASKED.lock().unwrap().push(event.id.to_owned());
        let marker = WorldLine::new(
            [100.25, 64.0, 99.0],
            [101.0, 64.0, 99.0],
            0xFF00FF00,
            2.0,
            WorldDepth::Tested,
        );
        event.draw.lines(&[marker]).unwrap();
    }
}

/// One frame's view: the lines it queued, camera relative, and the renderers asked.
fn frame(game: &super::Game<'_>, draw: &mut WorldGeometry) -> (Vec<[f32; 6]>, Vec<String>) {
    let title = game.shows_panorama();
    game.fill_world_geometry(title, CAMERA, Mat4::IDENTITY, 0.0, draw);
    let lines = draw
        .batch()
        .lines
        .iter()
        .map(|l| std::array::from_fn(|i| l[i]));
    (lines.collect(), std::mem::take(&mut *ASKED.lock().unwrap()))
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_debug_renderer_draws_while_selected_and_leaves_nothing_behind() {
    with_client_game(|game| {
        mod_api::__private::install(vec![ModEntry::new("marker", Box::new(Marker))].into());
        mods::register_debug_renderers(ExecutionRole::Client);
        assert_eq!(
            debug_entries::entry(SHOWN).map(|e| e.category),
            Some(Category::Renderer)
        );
        // Not tied to a file: the statuses below stay out of `debug-profile.json`.
        game.debug_entries = DebugEntries::default();
        game.title = None;
        let mut draw = WorldGeometry::default();
        let none = (Vec::new(), Vec::new());

        // Off by default: no call and no output.
        assert_eq!(frame(game, &mut draw), none);

        // Selected: only that renderer is asked, and its marker is at the world position.
        game.debug_entries.set_status(SHOWN, Status::AlwaysOn);
        let marker = vec![[0.25, 0.0, -1.0, 1.0, 0.0, -1.0]];
        assert_eq!(
            frame(game, &mut draw),
            (marker.clone(), vec![SHOWN.to_owned()])
        );
        // A later frame replaces it rather than adding to it.
        assert_eq!(frame(game, &mut draw).0, marker);

        // Disabled: the marker goes with the next frame.
        game.debug_entries.set_status(SHOWN, Status::Never);
        assert_eq!(frame(game, &mut draw), none);

        // Leaving the world with it on: the title frame asks nothing and shows nothing.
        game.debug_entries.set_status(SHOWN, Status::AlwaysOn);
        assert_eq!(frame(game, &mut draw).0, marker);
        game.leave_server(None).unwrap();
        assert_eq!(frame(game, &mut draw), none);
    });
}
