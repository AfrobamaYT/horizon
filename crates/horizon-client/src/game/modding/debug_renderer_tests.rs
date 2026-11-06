//! Native debug renderers in a real game: when the renderer list refreshes, and what a frame's
//! world geometry holds as a renderer is switched and the world goes.
use super::{execution_role::ExecutionRole, mods, native_entity_refs::tests::with_client_game};
use glam::Mat4;
use horizon_client::{
    debug_entries::{self, Category, DebugEntries, Profile, Status},
    world_geometry::WorldGeometry,
};
use horizon_protocol::packets::play::{self, Clientbound as C};
use mod_api::{
    ClientHost, DebugEntry, DebugGroup, DebugRenderer, DebugRendererDrawEvent, Hooks, ModEntry,
    RegisterDebugEntriesEvent, RegisterDebugRenderersEvent, WorldDepth, WorldLine,
};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

const CAMERA: [f64; 3] = [100.0, 64.0, 100.0];
const SHOWN: &str = "marker:pipes";
const OTHER: &str = "marker:wires";
/// Pushed as a renderer, but no entry declares it.
const UNDECLARED: &str = "marker:ghost";

/// The ids `debug_renderer_draw` was asked about since the last frame.
static ASKED: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// What the mod pushes at a refresh.
static PUSHED: Mutex<Vec<&str>> = Mutex::new(Vec::new());
/// The refreshes since the last look (`refreshes`).
static REFRESHES: AtomicUsize = AtomicUsize::new(0);

struct Marker;
#[mod_api::__subscriptions]
impl Hooks for Marker {
    fn register_debug_entries(
        &self,
        _: &mod_api::RegistrationContext,
        event: &mut RegisterDebugEntriesEvent<'_>,
    ) {
        for id in [SHOWN, OTHER] {
            event.entries.push(DebugEntry {
                id: id.into(),
                group: DebugGroup::Renderer,
            });
        }
    }
    fn register_debug_renderers(
        &self,
        _: &mod_api::RegistrationContext,
        event: &mut RegisterDebugRenderersEvent<'_>,
    ) {
        REFRESHES.fetch_add(1, Ordering::Relaxed);
        for id in PUSHED.lock().unwrap().iter() {
            event.renderers.push(DebugRenderer {
                id: (*id).to_owned(),
            });
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
fn frame(game: &mut super::Game<'_>, draw: &mut WorldGeometry) -> (Vec<[f32; 6]>, Vec<String>) {
    let title = game.shows_panorama();
    game.fill_world_geometry(title, CAMERA, Mat4::IDENTITY, 0.0, draw);
    let lines = draw
        .batch()
        .lines
        .iter()
        .map(|l| std::array::from_fn(|i| l[i]));
    (lines.collect(), std::mem::take(&mut *ASKED.lock().unwrap()))
}

/// How many times the renderer list refreshed since the last call.
fn refreshes() -> usize {
    REFRESHES.swap(0, Ordering::Relaxed)
}

fn push(ids: &[&'static str]) {
    *PUSHED.lock().unwrap() = ids.to_vec();
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn the_renderer_list_refreshes_with_the_selection_and_leaves_nothing_behind() {
    with_client_game(|game| {
        mod_api::__private::install(vec![ModEntry::new("marker", Box::new(Marker))].into());
        push(&[SHOWN, OTHER, UNDECLARED]);
        mods::register_debug_entries(ExecutionRole::Client);
        // The options exist before any refresh, and pushing an id does not make one.
        assert_eq!(refreshes(), 0);
        for id in [SHOWN, OTHER] {
            assert_eq!(
                debug_entries::entry(id).map(|e| e.category),
                Some(Category::Renderer)
            );
        }
        // Not tied to a file: the statuses below stay out of `debug-profile.json`.
        game.debug_entries = DebugEntries::default();
        game.title = None;
        let mut draw = WorldGeometry::default();
        let none = (Vec::new(), Vec::new());

        // Off by default. The first view refreshes, as `lastDebugEntriesVersion` starts behind;
        // the next one does not.
        assert_eq!(frame(game, &mut draw), none);
        assert_eq!(refreshes(), 1);
        assert_eq!(frame(game, &mut draw), none);
        assert_eq!(refreshes(), 0);

        // Selected: one refresh, only that renderer is asked, and its marker is at the world
        // position.
        game.debug_entries.set_status(SHOWN, Status::AlwaysOn);
        let marker = vec![[0.25, 0.0, -1.0, 1.0, 0.0, -1.0]];
        let shown = (marker.clone(), vec![SHOWN.to_owned()]);
        assert_eq!(frame(game, &mut draw), shown);
        assert_eq!(refreshes(), 1);
        // A later frame replaces it rather than adding to it, with no refresh.
        assert_eq!(frame(game, &mut draw), shown);
        assert_eq!(refreshes(), 0);

        // The overlay changes the selection both ways.
        game.debug_entries.set_status(SHOWN, Status::InOverlay);
        assert_eq!(frame(game, &mut draw), none);
        assert_eq!(refreshes(), 1);
        game.debug = true;
        assert_eq!(frame(game, &mut draw), shown);
        assert_eq!(refreshes(), 1);
        game.debug = false;
        assert_eq!(frame(game, &mut draw), none);
        assert_eq!(refreshes(), 1);

        // So does reduced debug info, which hides a renderer.
        game.debug_entries.set_status(SHOWN, Status::AlwaysOn);
        assert_eq!(frame(game, &mut draw), shown);
        assert_eq!(refreshes(), 1);
        game.settings.reduced_debug_info = true;
        assert_eq!(frame(game, &mut draw), none);
        assert_eq!(refreshes(), 1);
        game.settings.reduced_debug_info = false;
        assert_eq!(frame(game, &mut draw), shown);
        assert_eq!(refreshes(), 1);

        // A built-in profile leaves it out.
        game.debug_entries.load_profile(Profile::Default);
        assert_eq!(frame(game, &mut draw), none);
        assert_eq!(refreshes(), 1);

        // What a mod stops pushing stays until the next refresh, then nothing of it is left. The
        // renderer with no entry is never asked.
        game.debug_entries.set_status(SHOWN, Status::AlwaysOn);
        game.debug_entries.set_status(OTHER, Status::AlwaysOn);
        let both = vec![SHOWN.to_owned(), OTHER.to_owned()];
        assert_eq!(frame(game, &mut draw).1, both);
        assert_eq!(refreshes(), 1);
        push(&[OTHER]);
        assert_eq!(frame(game, &mut draw).1, both);
        assert_eq!(refreshes(), 0);
        game.debug_entries.set_status(SHOWN, Status::AlwaysOn);
        assert_eq!(frame(game, &mut draw).1, [OTHER]);
        assert_eq!(refreshes(), 1);
        push(&[SHOWN]);

        // Disabled: the marker goes with the next frame.
        game.debug_entries.set_status(OTHER, Status::Never);
        game.debug_entries.set_status(SHOWN, Status::Never);
        assert_eq!(frame(game, &mut draw), none);
        assert_eq!(refreshes(), 1);

        // Leaving the world with it on: the title frame asks nothing and shows nothing, and
        // refreshes nothing, as it has no `emitGizmos`.
        game.debug_entries.set_status(SHOWN, Status::AlwaysOn);
        assert_eq!(frame(game, &mut draw), shown);
        assert_eq!(refreshes(), 1);
        game.leave_server(None).unwrap();
        game.debug_entries.set_status(SHOWN, Status::Never);
        assert_eq!(frame(game, &mut draw), none);
        assert_eq!(refreshes(), 0);

        // `handleLogin` refreshes the list, with no change of selection.
        let (pipe, _server) = horizon_protocol::connection::Pipe::pair();
        game.connect_local(pipe, "Steve").unwrap();
        game.handle_packet(C::Login(play::Login {
            player_id: 7,
            hardcore: false,
            levels: vec!["minecraft:overworld".into()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 8,
            reduced_debug_info: false,
            show_death_screen: true,
            limited_crafting: false,
            spawn: play::SpawnInfo {
                dimension_type: 0,
                dimension: "minecraft:overworld".into(),
                seed: 1,
                game_type: 0,
                previous_game_type: None,
                debug: false,
                flat: false,
                last_death: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            online_mode: false,
            enforces_secure_chat: false,
        }))
        .unwrap();
        assert_eq!(refreshes(), 1);
    });
}
