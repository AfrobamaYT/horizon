//! The playable game loop: streaming world, player simulation at 20 ticks/s,
//! block interaction, inventory and HUD.
use mod_api::__private::game::PartsLookup;

/// The content tables the world plays with, as field-level borrows: the
/// world's data packs' over the game's own. The `Game::items`/`recipes`/
/// `stations` methods read the same tables, but taking `&self` blocks a
/// `&mut self.…` in the same scope; these expand to a match on the fields,
/// which borrows stay disjoint from.
macro_rules! world_items {
    ($s:ident) => {
        match (&$s.server_items, &$s.world_tables) {
            (Some(items), _) => items,
            (None, Some(t)) => &t.items,
            (None, None) => $s.res.items(),
        }
    };
}
macro_rules! world_recipes {
    ($s:ident) => {
        match &$s.world_tables {
            Some(t) => &t.recipes,
            None => $s.res.recipes(),
        }
    };
}
macro_rules! world_stations {
    ($s:ident) => {
        match &$s.world_tables {
            Some(t) => &t.stations,
            None => $s.res.stations(),
        }
    };
}
/// The biome table the world plays with (`Game::biomes`), as a field-level
/// borrow: the world's packs' over the game's own.
macro_rules! world_biomes {
    ($s:expr) => {
        match &$s.world_biomes {
            Some(b) => b,
            None => $s.res.biomes(),
        }
    };
}

/// The world's packs' content files, as a field-level borrow (see the macros).
fn pack_files(world_tables: &Option<WorldTables>) -> horizon_world::loot::PackOverlay<'_> {
    match world_tables {
        Some(t) => horizon_world::loot::PackOverlay {
            conditions: &t.conditions,
            files: &t.files,
        },
        None => horizon_world::loot::PackOverlay::NONE,
    }
}

include!(concat!(env!("OUT_DIR"), "/game_modules.rs"));
/// The game's side of the injection points (#589), whose slots and listeners live in `mod_points`.
#[doc(hidden)]
pub mod points {
    /// A `side = "both"` point's host, out of line: each point's listener call stays a few bytes.
    /// Called only by the `#[point]`s the stage adds to its copy.
    #[inline(never)]
    #[allow(dead_code, reason = "used by the staged copy's points")]
    pub(super) fn host_side<'a>(game: &'a mut super::Game<'_>) -> mod_api::HostSide<'a> {
        super::execution_role::host_side(game)
    }
}
mod host;
mod native_hooks;
use event_loop::EventLoopCtl;
// `Action` here is the title screen's buttons (`title::Action`).
use horizon_client::actions::Action as Mapping;
use horizon_client::held::Source;
use horizon_client::spans::startup;
use horizon_client::{
    Result,
    assets::{Assets, RenderAssets},
    audio::Audio,
    biome::Biomes,
    block_entity_render::{
        self, BlockEntityRenderer, CacheOptions, Scene as BlockEntityScene, StaticCacheFrameStats,
        StaticSectionSnapshot,
    },
    blocks::{BlockFacts, Fluid},
    clock::FixedStep,
    collisions,
    containers::{BlockEntity, Providers},
    crafting::Recipes,
    creative::{Creative, Names},
    entities::{Experience, ExperienceOrb, ItemEntity},
    environment::Environment,
    first_person::{Hand, SwingAnimation},
    furnace::FurnaceCooking,
    gfx::{Frame, Renderer},
    graphics::EntityCullingMode,
    hud::{HudAtlas, HudBatch, rgba},
    hud_layout, hud_overlays,
    items::{ItemStack, Items},
    keymapping::{KeyAction, SNEAK, SPRINT},
    light::LightTables,
    mesher::{MeshTables, Vertex},
    mobs::{Mob, MobType, MobTypes},
    models,
    motion::{MotionEvent, MotionResources, PlayerMotion},
    mouse::{LookOptions, MouseLook},
    particles::Particle,
    save::{Level, Persistent, SaveDir},
    settings::Settings,
    spans,
    sprint::Keys,
    streaming::{Streaming, WorkerContext},
    testgen::TestTerrain,
    title::{Action, Title},
    vanillagen::VanillaTerrain,
    world::World,
};
use horizon_core::{
    biome::{BiomeZoom, obfuscate_seed},
    geometry::{Aabb, Direction},
    position::BlockPos,
};
use horizon_core::{geometry::Vec3, random::RandomSource};
use horizon_mod_package::selection::{Argument, LaunchArguments};
pub use maps::{NativeMapDecoration, NativeMapInfo};
pub use native_block_entity_draw::BlockEntityDraw;
pub use native_claimed_items::{NativeHeldItem, NativeHit, NativeModEvent};
pub use native_components::NativeComponentError;
pub use native_configs::{NativeConfig, NativeConfigFile, NativeConfigKind};
pub use native_data_maps::NativeDataMapType;
pub use native_falling::NativeFalling;
pub use native_hooks::{NativeClock, NativeShake, PhantomSpawnResult, SleepProblem};
pub use native_mob_draw::{NativeMobRender, NativePlayerRender};
pub use native_server::NativeExperienceOrb;
pub use native_world_columns::NativeColumnBlocks;
pub use native_world_edit::{NativeBlockEdit, NativeBlockRun, NativeHeightmap};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
        mpsc::{Receiver, TryRecvError},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::{DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};
use {
    horizon_core::dimension::Dimension as LevelDimension,
    horizon_world::{
        block_collision::BlockCollisionRegistry,
        collision_context::{CollisionContext, CollisionEntity, CollisionItem},
        entity_obstruction::{EntityIdentity, EntityObstructionWorld, ObstructionEntity},
        fluid::FluidState,
        level::{
            Behaviours, BucketContent, EntityQuery, Landing, LevelAccess, LevelEvent, PlaceContext,
            PlaceOutcome, PlacementContext, PlayerContext, PlayerEffect, Ticks,
            flags as level_flags,
        },
        registry::BlockStateId,
    },
};

/// The time a frame spends uploading chunk meshes when the player set no
/// Chunk Updates or Upload Budget: a burst past it waits for the next frames.
const AUTO_UPLOAD_TIME: Duration = Duration::from_millis(2);
/// `AUTO_UPLOAD_TIME` while the view's terrain is still arriving (a world's
/// open, a teleport): the meshes waiting then are the view itself, not edits
/// to it, so a frame gives them more of its time to fill the view sooner.
const AUTO_UPLOAD_TIME_ARRIVING: Duration = Duration::from_millis(6);
/// A frame's finished meshes up to this many upload at once, unqueued.
const AUTO_UPLOADS_AT_ONCE: usize = 8;

#[derive(Clone)]
struct Options {
    resources: PathBuf,
    distance: i32,
    distance_override: Option<i32>,
    seed: i64,
    creative: bool,
    test_terrain: bool,
    /// `--pregen`: enable the command even when the saved Extras switch is off.
    pregen: bool,
    frames: Option<u64>,
    /// A splitscreen player: the split window paces every player's frames, not each its own.
    paced_by_split: bool,
    /// A splitscreen player other than P1: its windows are the split's (P1 alone decides
    /// fullscreen; this game neither asks for it nor adopts it into its options).
    split_guest: bool,
    /// A splitscreen guest drawn in Window 1, the game's own window: its F11 leaves that
    /// window to P1.
    split_in_main_window: bool,
    /// `--world-frames N`: exit after N frames drawn in the world (smoke runs; the world may load slowly).
    world_frames: Option<u64>,
    screenshot_after: Option<u64>,
    timeout: Duration,
    report: Option<PathBuf>,
    world: Option<String>,
    look: Option<(f32, f32)>,
    time: Option<i64>,
    weather: Option<String>,
    dimension: Option<String>,
    /// Options.pauseOnLostFocus off (F3+P), for scripted testing.
    no_pause: bool,
    /// Control socket path (`--control`), for `horizon-ctl`.
    control: Option<PathBuf>,
    /// `HORIZON_SPIKES=MS`: print the timed spans of frames slower than MS.
    spikes: Option<f32>,
    /// Render into a texture instead of the window (`--offscreen`), so a
    /// GPU the display cannot present from can still be benchmarked. The
    /// window stays hidden and never grabs the system pointer.
    offscreen: bool,
    /// `--headless` (#4951): no display, no window and no renderer. The game runs its own
    /// loop (`game/runtime/headless.rs`) instead of winit's, so it starts where no display server
    /// is (an SSH session, a server). It counts as `offscreen` for everything else (an
    /// automated run that never shows a window or grabs the pointer).
    headless: bool,
    /// `--width` and `--height` (vanilla's own arguments): the window's
    /// starting size, 1280x720 by default.
    window_size: [u32; 2],
    /// Multiplayer: `--server HOST[:PORT]` joins at start; `--name` is the offline player name.
    server: Option<String>,
    /// `--name`, `HORIZON_PLAYER_NAME` or the signed-in account's name; else
    /// [`Options::player_name`] uses the kept offline name.
    player_name: Option<String>,
    /// `--dedicated`: run as a dedicated server; vanilla's server arguments
    /// (`--nogui`, `--port`, `--world`, `--universe`, `--initSettings`) follow.
    dedicated: Option<Vec<String>>,
    /// `--transfers SETTINGS`: a dedicated server takes a proxy's hand-over requests
    /// (`server.toml`'s `[transfer]`, which horizon-server passes; lan/arrivals.rs).
    transfers: Option<horizon_client::lan::arrivals::Settings>,
    /// `--benchmark SCENARIO`: run the benchmark, write its report and quit.
    benchmark: Option<String>,
    /// Content selection and the compiled native runtime shared with resource loaders.
    mods: mods::ModsState,
    /// What this Game executes; independent of its current server connection.
    execution_role: execution_role::ExecutionRole,
    /// `--fullscreen` (vanilla's): the window opens fullscreen whatever options.txt says,
    /// as vanilla's DisplayData does, without saving it there.
    fullscreen: bool,
    /// `--game-dir DIR`: the folder `read` makes the working directory (saves, options.txt...);
    /// none for a splitscreen player's game, which shares P1's working directory.
    game_dir: Option<PathBuf>,
    /// `--quickPlaySingleplayer`, checked once the game folder is known.
    quick_play: Option<String>,
    session_file: Option<String>,
    /// `--session-stdin`: the session file's JSON comes on standard input instead.
    session_stdin: bool,
}
impl Options {
    /// Whether this `Game` is the process's client start: not a dedicated server, and not
    /// a splitscreen guest, whose `run` is another local player joining the same process.
    fn is_client_start(&self) -> bool {
        self.execution_role.is_client() && !self.split_guest
    }
    /// The name to join servers with: the one given, else the offline name kept
    /// next to options.txt (created on first use).
    fn player_name(&self) -> String {
        self.player_name.clone().unwrap_or_else(|| {
            std::env::current_dir().map_or_else(
                |_| horizon_client::offline_name::generate(0),
                |_| {
                    horizon_client::offline_name::load_or_create(
                        &horizon_client::logging::player_dir(),
                    )
                },
            )
        })
    }
    /// The command line, then what it asks of the process: the game folder, the account.
    /// A bad command line is a [`launch_api::UsageError`] (exit code 2, no crash report).
    fn read(launch: LaunchArguments) -> Result<Option<Self>> {
        let Some(mut o) =
            Self::from_launch(launch).map_err(|e| launch_api::UsageError(e.to_string()))?
        else {
            return Ok(None);
        };
        // Paths given on the command line stay relative to where the game was started.
        if o.game_dir.is_some() {
            o.control = o.control.map(std::path::absolute).transpose()?;
        }
        // Refused here, before the account's file is used up or anything loads.
        if let Some(path) = &o.control {
            horizon_client::control::check_path(path).map_err(|e| {
                launch_api::UsageError(format!("--control-socket {}: {e}", path.display()))
            })?;
        }
        // The launcher's account: online-mode servers need it. A bad file
        // only leaves the game offline.
        let session = if o.session_stdin {
            let mut text = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut text)
                .map_err(|e| e.to_string())
                .and_then(|_| horizon_client::auth::AuthSession::parse(&text))
                .map(Some)
        } else {
            o.session_file
                .take()
                .filter(|p| !p.is_empty())
                .map(|path| horizon_client::auth::AuthSession::load(path.as_ref()))
                .transpose()
        };
        match session {
            Ok(Some(session)) => {
                o.player_name = Some(session.name.clone());
                horizon_client::auth::set_current(session);
                // The chat key pair, ready before the first join.
                horizon_client::auth::prepare_chat_keys(true);
            }
            Ok(None) => {}
            Err(e) => eprintln!("Horizon: not signed in: {e}"),
        }
        split::launcher::read_now();
        if let Some(dir) = o.game_dir.take() {
            // These too stay relative to where the game was started.
            o.resources = std::path::absolute(&o.resources)?;
            o.report = o.report.map(std::path::absolute).transpose()?;
            std::fs::create_dir_all(&dir)
                .and_then(|()| std::env::set_current_dir(&dir))
                .map_err(|e| format!("game folder {}: {e}", dir.display()))?;
        }
        // Vanilla's quick play (Main, QuickPlay.joinSingleplayerWorld): only a
        // world that exists opens; else the title screen. --world also creates one.
        if let Some(world) = o.quick_play.take() {
            if std::env::current_dir()?
                .join("saves")
                .join(&world)
                .join("level.dat")
                .is_file()
            {
                o.world = Some(world);
            } else {
                eprintln!("Quick play: there is no world {world} in saves/");
            }
        }
        Ok(Some(o))
    }
    /// The command line alone: no file is read and nothing changes. `None` for `--help` and
    /// `--launch-api`, which print and end the run.
    #[cfg(test)]
    fn parse(args: impl Iterator<Item = String>) -> Result<Option<Self>> {
        Self::from_launch(LaunchArguments::parse(
            args,
            &std::env::current_dir()?,
            &horizon_client::logging::game_home(),
            std::env::var_os("HORIZON_HOME").as_deref(),
        )?)
    }

    fn from_launch(LaunchArguments { context, rest }: LaunchArguments) -> Result<Option<Self>> {
        let sibling = std::env::current_exe()?
            .parent()
            .ok_or("missing executable directory")?
            .join("resources");
        let mut o = Self {
            resources: if sibling.join("horizon-resources.json").exists() {
                sibling
            } else {
                std::path::Path::new(".local/horizon-client").join(horizon_core::VANILLA_VERSION)
            },
            distance: 12,
            distance_override: None,
            seed: 0,
            creative: false,
            test_terrain: false,
            pregen: false,
            frames: None,
            paced_by_split: false,
            split_guest: false,
            split_in_main_window: false,
            world_frames: None,
            screenshot_after: None,
            timeout: Duration::from_secs(120),
            report: None,
            world: None,
            look: None,
            time: None,
            weather: None,
            dimension: None,
            no_pause: false,
            control: None,
            offscreen: false,
            headless: false,
            window_size: [1280, 720],
            spikes: std::env::var("HORIZON_SPIKES")
                .ok()
                .and_then(|v| v.parse().ok()),
            server: None,
            player_name: std::env::var("HORIZON_PLAYER_NAME")
                .ok()
                .filter(|n| !n.is_empty()),
            dedicated: None,
            transfers: None,
            benchmark: None,
            mods: mods::ModsState {
                dev: context.dev_mods,
                ..Default::default()
            },
            execution_role: execution_role::ExecutionRole::Client,
            fullscreen: false,
            game_dir: Some(context.game_dir),
            quick_play: None,
            session_file: std::env::var("HORIZON_SESSION_FILE").ok(),
            session_stdin: false,
        };
        for argument in rest {
            match argument {
                Argument::Switch(arg) => match arg.as_str() {
                    "--help" | "-h" => {
                        println!(
                            "Horizon — Minecraft Java Edition {} in Rust\n\n  --resources PATH        Prepared resources (horizon-prepare)\n  --render-distance N     Chunks, 2..128 (default 12)\n  --world NAME            World folder under saves/ (default \"New World\")\n  --seed N                World seed for new worlds (default 0)\n  --width N, --height N   Window size (default 1280 x 720)\n  --creative              Creative mode (default survival)\n  --test-terrain          Simple test terrain instead of vanilla generation\n  --frames N              Exit after N frames (testing)\n  --world-frames N        Exit after N frames drawn in the world (testing)\n  --smoke-report PATH     Write a JSON report on exit\n  --benchmark SCENARIO    Run a benchmark (standard, village, forest, ocean, mountain), write its report to benchmarks/ and quit\n  --control               Accept horizon-ctl commands on a local socket\n  --control-socket PATH   Control socket path (default $XDG_RUNTIME_DIR/horizon-control.sock)\n  --server HOST[:PORT]    Join a multiplayer server (offline mode) at start\n  --quickPlaySingleplayer WORLD  Open this world of saves/ at start, only if it exists (vanilla's)\n  --quickPlayMultiplayer HOST[:PORT]  The same as --server (vanilla's name)\n  --name NAME             Player name for multiplayer (default: generated once, kept in horizon-player.txt)\n  --session-file PATH     Signed-in account from the launcher (read once, then deleted)\n  --session-stdin         The same account as JSON on standard input\n  --game-dir DIR          Game folder: saves, options.txt, packs, mods (default: the working directory)\n  --home DIR              Logs and crash reports (default $HORIZON_HOME, else the platform data folder)\n  --player-dir DIR        This player's own files when several play in one game folder: options.txt, logs, horizon-player.txt, servers.dat, screenshots\n  --fullscreen            Open the window fullscreen\n  --launch-api            Print the Launch API version and options as JSON (docs/api/launch.md)\n  --dev-mod PATH          A content mod (.hmod or folder) to develop: always on\n  --pregen                Add Horizon's /pregen command (docs/pregeneration.md)\n  --wasm-interpreter      Run wasm mods interpreted (Pulley) instead of compiled: slower, for hosts without JIT\n  --wasm-fuel             Limit wasm mods' calls by instructions instead of time: the same on every machine, slower\n\nControls: mouse look, WASD move, Space jump, Left Shift sneak, Left Ctrl sprint,\nleft click break, right click place/use, 1-9/scroll hotbar, E inventory,\nQ drop, F3 debug, Esc pause.",
                            horizon_core::VANILLA_VERSION
                        );
                        return Ok(None);
                    }
                    "--launch-api" => {
                        println!("{}", launch_api::reply(&horizon_client::crash::version()));
                        return Ok(None);
                    }
                    "--fullscreen" => o.fullscreen = true,
                    "--session-stdin" => o.session_stdin = true,
                    "--creative" => o.creative = true,
                    "--test-terrain" => o.test_terrain = true,
                    "--no-pause" => o.no_pause = true,
                    "--offscreen" => o.offscreen = true,
                    "--headless" => {
                        o.headless = true;
                        o.offscreen = true;
                    }
                    "--control" => o.control = Some(horizon_client::control::default_path()),
                    // Single player always runs on its server thread: a launcher's saved arguments
                    // may still name the old switches.
                    "--server-thread" => {}
                    "--in-frame" | "--no-server-thread" => eprintln!(
                        "Horizon: {arg} was removed: single player runs on its server thread."
                    ),
                    // Horizon's own commands (/pregen), off by default as on vanilla.
                    "--pregen" => {
                        o.pregen = true;
                        horizon_client::command::vanilla::set_horizon_commands(true);
                    }
                    // Read by `entry::main_with` when it loads the wasm mods.
                    "--wasm-interpreter" | "--wasm-fuel" => {}
                    _ => return Err(format!("{arg}: unknown option").into()),
                },
                Argument::Dedicated(args) => {
                    o.execution_role = execution_role::ExecutionRole::DedicatedServer;
                    o.dedicated = Some(args);
                    break;
                }
                Argument::Value(arg, value) => {
                    // A launcher shows the error to the player: it names the option.
                    let read = || -> Result<()> {
                        match arg.as_str() {
                            "--resources" => o.resources = value.into(),
                            "--render-distance" | "--radius" => {
                                o.distance = value.parse()?;
                                o.distance_override = Some(o.distance);
                                let max = horizon_client::options_file::MAX_RENDER_DISTANCE;
                                if !(2..=max).contains(&o.distance) {
                                    return Err(format!("render distance must be 2..{max}").into());
                                }
                            }
                            "--seed" => {
                                o.seed = value.parse().unwrap_or_else(|_| {
                                    // Text seeds hash like String.hashCode.
                                    value
                                        .encode_utf16()
                                        .fold(0i32, |h, c| {
                                            h.wrapping_mul(31).wrapping_add(i32::from(c))
                                        })
                                        .into()
                                })
                            }
                            "--frames" => o.frames = Some(value.parse()?),
                            "--world-frames" => {
                                o.world_frames = Some(value.parse()?);
                                // A frame-limited run in every other respect; --timeout-seconds still applies.
                                o.frames.get_or_insert(u64::MAX);
                            }
                            "--timeout-seconds" => o.timeout = Duration::from_secs(value.parse()?),
                            "--smoke-report" => o.report = Some(value.into()),
                            "--screenshot-after" => o.screenshot_after = Some(value.parse()?),
                            "--width" => o.window_size[0] = value.parse::<u32>()?.max(1),
                            "--height" => o.window_size[1] = value.parse::<u32>()?.max(1),
                            // Vanilla opens only a level id that is one folder of saves/. --world, for
                            // harnesses, also takes a path (tools/bundle_tooltip_e2e.py).
                            "--world" => o.world = Some(value),
                            "--quickPlaySingleplayer" => {
                                let mut parts = std::path::Path::new(&value).components();
                                if !matches!(
                                    (parts.next(), parts.next()),
                                    (Some(std::path::Component::Normal(_)), None)
                                ) {
                                    return Err("not a folder name of saves/".into());
                                }
                                o.quick_play = Some(value);
                            }
                            // main has set this player's own folder (logging::set_player_dir).
                            "--player-dir" => {}

                            "--benchmark" => {
                                if horizon_client::benchmark::scenes(&value).is_none() {
                                    return Err(format!(
                                    "unknown benchmark {value}: standard, village, forest, ocean or mountain"
                                )
                                .into());
                                }
                                o.benchmark = Some(value);
                            }
                            "--server" | "--quickPlayMultiplayer" => o.server = Some(value),
                            "--name" => o.player_name = Some(value),
                            "--session-file" => o.session_file = Some(value),
                            "--control-socket" => o.control = Some(value.into()),
                            "--transfers" => {
                                o.transfers =
                                    Some(horizon_client::lan::arrivals::Settings::parse(&value)?)
                            }
                            "--time" => o.time = Some(value.parse()?),
                            "--weather" => o.weather = Some(value),
                            "--dimension" => {
                                o.dimension = Some(if value.contains(':') {
                                    value
                                } else {
                                    format!("minecraft:{value}")
                                })
                            }
                            "--look" => {
                                let (y, p) =
                                    value.split_once(',').ok_or("--look takes YAW,PITCH")?;
                                o.look = Some((y.parse()?, p.parse()?));
                            }
                            _ => return Err("unknown option".into()),
                        }
                        Ok(())
                    };
                    read().map_err(|e| format!("{arg}: {e}"))?;
                }
            }
        }
        Ok(Some(o))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Playing,
    GameModeSwitcher,
    Paused,
    Options,
    Chat,
    Creative,
    Inventory,
    Dead,
    /// The Statistics screen.
    Stats,
    /// The Advancements screen.
    Advancements,
    /// WorldOptionsScreen (game/network/lan_screen.rs).
    WorldOptions,
    /// Play with Friends, from the pause menu's Friends button
    /// (`friends_screen`, game/network/friends.rs).
    Friends,
    /// `SocialInteractionsScreen` (`social_screen`, game/network/social.rs).
    Social,
    /// CommandBlockEditScreen (game/administration/command_blocks.rs).
    CommandBlock,
    /// A game master block's edit screen (game/interface/block_editor.rs).
    BlockEditor,
    /// ServerLinksDialogScreen: a server's links (game/network/server_links.rs).
    ServerLinks,
    /// ConfirmLinkScreen for a server link (game/network/server_links.rs).
    ConfirmLink,
    /// DialogScreen.WarningScreen: the note about custom screens.
    DialogWarning,
    /// WinScreen: the end poem and the credits (game/interface/win.rs).
    Win,
    /// DebugOptionsScreen, F3+F6 (game/interface/debug_options.rs).
    DebugOptions,
    /// A screen of a native mod's own (`mod_screen_open`): it draws and
    /// takes its input through the screen hooks.
    Mod,
    /// PvP & HUD's widget editor (game/interface/hud_editor.rs).
    HudEditor,
}
impl Screen {
    /// The id native mods see for this screen (`NativeScreenView.id`, the
    /// `screen_render_post` hook): one per variant, and a hook's contract, so
    /// it stays the same from release to release. A new variant must add one.
    fn id(self) -> &'static str {
        match self {
            Screen::Playing => "playing",
            Screen::GameModeSwitcher => "game_mode_switcher",
            Screen::Paused => "paused",
            Screen::Options => "options",
            Screen::Chat => "chat",
            Screen::Creative => "creative",
            Screen::Inventory => "inventory",
            Screen::Dead => "dead",
            Screen::Stats => "stats",
            Screen::Advancements => "advancements",
            Screen::WorldOptions => "world_options",
            Screen::Friends => "friends",
            Screen::Social => "social",
            Screen::CommandBlock => "command_block",
            Screen::BlockEditor => "block_editor",
            Screen::ServerLinks => "server_links",
            Screen::ConfirmLink => "confirm_link",
            Screen::DialogWarning => "dialog_warning",
            Screen::Win => "win",
            Screen::DebugOptions => "debug_options",
            Screen::Mod => "mod",
            Screen::HudEditor => "hud_editor",
        }
    }
}
/// The vanilla `state` a `transformer` table entry offers to `ability`: hoe and
/// shovel tables have one action each; the axe's strip, scrape and wax-off lists
/// are one untagged list in `BlockTransformers.bootstrap`, so its particle names it.
fn vanilla_tool_state(
    transformer: &str,
    particle: &str,
    ability: mod_api::ToolAbility,
    state: BlockStateId,
) -> Option<BlockStateId> {
    use mod_api::ToolAbility::*;
    let action = match transformer {
        "minecraft:axe" => match particle {
            "scrape" => AxeScrape,
            "wax_off" => AxeWaxOff,
            _ => AxeStrip,
        },
        "minecraft:shovel" => ShovelFlatten,
        "minecraft:hoe" => HoeTill,
        _ => return None,
    };
    (action == ability).then_some(state)
}

struct Mining {
    pos: [i32; 3],
    progress: f32,
    ticks: u32,
}
impl Mining {
    fn stage(&self) -> Option<u8> {
        (self.progress > 0.0).then(|| ((self.progress * 10.0) as u8).min(9))
    }
}
#[derive(Clone, Copy)]
struct TransformContext<'a> {
    world: &'a World,
    player: Option<u32>,
    hand: mod_api::Hand,
    blocking: bool,
    simulated: bool,
}

#[derive(Clone, Copy)]
struct Hit {
    pos: [i32; 3],
    face: [i32; 3],
    point: [f64; 3],
    /// The ray started inside the block (`BlockHitResult.isInside`).
    inside: bool,
}
impl Hit {
    /// The hit as block-use hooks hear it (`UseOnContext`'s hit result).
    fn native(&self) -> mod_api::BlockHit {
        mod_api::BlockHit::new(self.point, self.face, self.inside)
    }
    /// The block the clicked face leads to (`pos.relative(direction)`).
    fn beside(&self) -> [i32; 3] {
        [
            self.pos[0] + self.face[0],
            self.pos[1] + self.face[1],
            self.pos[2] + self.face[2],
        ]
    }
}
/// What a falling block's tick did (`FallingBlockEntity.tick`).
enum FallStep {
    Falling,
    Gone,
    /// Gave up falling: drops itself as an item here.
    Dropped(Vec3),
    /// Hit the ground in this block.
    Landed([i32; 3]),
}
/// A guest's right click comes as one packet per hand and part
/// (`Minecraft.startUseItem`): `ServerGamePacketListenerImpl.handleUseItemOn`
/// does the block's and `ItemStack.useOn`'s part, `handleUseItem` the
/// item's own (`Item.use`). Running both for either would use the item twice.
#[derive(Clone, Copy)]
enum GuestUse {
    OnBlock(Hand),
    Item(Hand),
    /// `handleInteract`: `Player.interactOn` with the hand, nothing else (the
    /// client sends UseItem itself when the entity passed), on the entity the
    /// packet named: `ServerGamePacketListenerImpl.handleInteract` resolves
    /// `packet.entityId()` once and range checks it, so the click never picks
    /// an entity again from the view.
    Entity(Hand, InteractTarget),
}

/// The entity an interact packet names and where its client clicked it (the
/// packet's location, measured from the entity's position): what
/// `ServerGamePacketListenerImpl.handleInteract` takes from
/// `ServerboundInteractPacket` and hands to `Player.interactOn`, which passes
/// the point on to `interactAt`.
#[derive(Clone, Copy)]
struct InteractTarget {
    id: i32,
    location: [f64; 3],
}

/// The client world as seen by the vanilla block-behaviour library.
struct GameLevel<'w> {
    world: &'w mut World,
    command_block_transition: Option<command_blocks::CommandBlockTransition>,
    redstone_experiments: bool,
    facts: &'w BlockFacts,
    collision: &'w BlockCollisionRegistry<'w>,
    events: &'w mut Vec<LevelEvent>,
    dimension: LevelDimension,
    darken: i32,
    /// The environment of the level's dimension (`SUN_ANGLE`).
    environment: &'w Environment,
    raining: bool,
    thundering: bool,
    /// The world's enabled feature flags as this side knows them (`LevelReader.enabledFeatures`).
    enabled_features: &'w std::sync::Arc<[String]>,
    player_box: Option<([f64; 3], [f64; 3])>,
    /// The LAN guests in this level who trigger blocks (not spectators).
    guest_boxes: Vec<([f64; 3], [f64; 3])>,
    /// Other players that block placement during a guest turn, including live
    /// ancestor bodies; None outside one, where `guest_boxes` are those players.
    building_player_boxes: Option<Vec<([f64; 3], [f64; 3])>>,
    player_collision: Option<PlayerCollision>,
    /// Minecart boxes (detector rails).
    cart_boxes: Vec<([f64; 3], [f64; 3])>,
    /// The item frames a comparator reads through a conductor.
    item_frames: &'w [horizon_client::entities::ItemFrame],
    /// The minecarts, in `cart_boxes` order, for a detector rail's comparator.
    carts: &'w [minecarts::Minecart],
    /// Arrow and thrown trident boxes (wooden buttons).
    arrow_boxes: Vec<([f64; 3], [f64; 3])>,
    /// Every entity on the walk; see `entity::for_each_entity`.
    entities: entity::EntityLists<'w>,
    /// End crystals, which aren't on the walk yet.
    crystals: &'w [dragon::Crystal],
    /// The fishing hooks here, which aren't on the walk either. None ignores
    /// block triggers, so each counts on a plate or tripwire. The flag says
    /// whether it also reaches `entityInside` (`applyEffectsFromBlocks`): not
    /// a hook on an entity, whose tick skips that step.
    trigger_hooks: Vec<(Aabb, bool)>,
    /// The lightning bolts here: none ignores block triggers
    /// (`Entity.isIgnoringBlockTriggers`).
    trigger_extras: Vec<Aabb>,
    biomes: &'w Biomes,
    seed: i64,
    behaviours: &'w Behaviours,
    items: &'w Items,
    /// Cooking recipes' experience, which a removed furnace pays out.
    recipes: &'w horizon_client::crafting::Recipes,
    /// Container loot tables (`unpackLootTable`).
    loot: Option<std::sync::Arc<horizon_world::loot::LootData>>,
    registry: &'static horizon_world::registry::BlockRegistry,
    block_registries: &'w horizon_client::command::args::Registries,
    rules: horizon_world::level::GameRules,
    /// `Difficulty.getId()` (peaceful 0 .. hard 3).
    difficulty: i32,
    /// The feet of the players who keep fire spreading around them: the local
    /// player and the LAN guests in this level, spectators aside.
    fire_players: Vec<[f64; 3]>,
    /// Block loot random sequences (`ServerLevel.getRandomSequence`).
    rand_sequences: &'w mut horizon_client::random_sequences::RandomSequences,
    pending: &'w mut spill::Pending,
    spill: &'w mut spill::Spill,
    /// Where the chunk tickets let blocks tick.
    tickets: horizon_client::ticket_levels::Levels,
    random_tick_sections: &'w mut horizon_client::random_tick_sections::RandomTickSections,
    /// Players with each chest open (a trapped chest's signal), also what a
    /// recount of the openers replaces (`ContainerOpenersCounter.recheckOpeners`).
    lid_openers: &'w mut HashMap<(i32, i32, i32), i32>,
    /// The containers players have open at this moment, by position: the local
    /// player's and the guests' (`getEntitiesWithContainerOpen`). A double
    /// chest is in it as both halves.
    open_containers: &'w [(i32, i32, i32)],
    /// `Level.isClientSide`: a client's copy of the level (a guest of a LAN
    /// world, a window on its server thread, a dedicated server's client).
    client_side: bool,
    /// Only the integrated or dedicated server dispatches native observations.
    native_authoritative: bool,
    native_player: Option<u128>,
    /// The acting player's entity id, beside `native_player`.
    native_player_entity: Option<i32>,
    /// `Level.isDebug`: `setBlock` changes nothing.
    debug: bool,
    native_sounds: &'w mut Vec<creatures::NativeSound>,
    native_particles: &'w mut Vec<lan_particles::QueuedParticle>,
    /// The server sound-seed source (`Level.soundSeedGenerator`), when this
    /// level's host is up: `native_play_sound` draws its seed there.
    sound_seeds: Option<&'w horizon_client::lan::hub::Hub>,
    /// The tool a mod's machine breaks blocks with (`native_machine_destroy_block`):
    /// `Block.dropResources` rolls the loot with it instead of with none.
    drop_tool: Option<ItemStack>,
    /// Where `Block.dropResources` puts the drops instead of popping them,
    /// while a mod's machine breaks a block (`native_machine_destroy_block`).
    drop_into: Option<Vec<ItemStack>>,
}
struct PlayerCollision {
    y: f64,
    descending: bool,
    fall_distance: f64,
    main_hand_item: Arc<str>,
    main_hand_count: i32,
    leather_boots: bool,
}
/// The boxes of the entities that block building, as obstruction candidates.
struct BuildingObstruction(Vec<Aabb>);
impl EntityObstructionWorld for BuildingObstruction {
    type Error = std::convert::Infallible;
    fn obstruction_candidates(
        &mut self,
        _excluded: Option<EntityIdentity>,
        bounds: Aabb,
    ) -> std::result::Result<Vec<ObstructionEntity>, Self::Error> {
        Ok(self
            .0
            .iter()
            .filter(|b| b.intersects(bounds))
            .enumerate()
            .map(|(i, &bounds)| ObstructionEntity {
                identity: EntityIdentity(i as u64),
                root_vehicle: EntityIdentity(i as u64),
                bounds,
                removed: false,
                spectator: false,
                blocks_building: true,
            })
            .collect())
    }
}
/// Whether a block's successful use is `InteractionResult.SUCCESS_SERVER`,
/// which a client does not predict: `useWithoutItem` of a bed, a light
/// block, a charged respawn anchor or a sign. Every other block use the
/// level answers with SUCCESS is `SUCCESS` (`SwingSource.PREDICTED`).
fn server_only_block_use(
    block: &horizon_items::blocks::StateFacts,
    branch: horizon_world::level::UseBranch,
) -> bool {
    branch == horizon_world::level::UseBranch::WithoutItem
        && [
            "AbstractBedBlock",
            "LightBlock",
            "RespawnAnchorBlock",
            "SignBlock",
        ]
        .iter()
        .any(|class| block.has_class(class))
}
/// An entity's box (`EntityDimensions.makeBoundingBox`): `width` wide and
/// `height` tall, standing on `feet`.
fn point_box(feet: [f64; 3], width: f64, height: f64) -> Aabb {
    Aabb::new(
        Vec3::new(feet[0] - width / 2.0, feet[1], feet[2] - width / 2.0),
        Vec3::new(
            feet[0] + width / 2.0,
            feet[1] + height,
            feet[2] + width / 2.0,
        ),
    )
}
impl GameLevel<'_> {
    /// `LilyPadBlock.entityInside`: the boxes of the boats that can break a pad.
    fn boat_trigger_boxes(&self) -> impl Iterator<Item = Aabb> + '_ {
        self.entities.boats.iter().map(boats::Boat::bounds)
    }
    /// How many entities in `area` press plates and tripwires, or with
    /// `LIVING` only the living ones; a constant so a list of other kinds
    /// compiles to no loop at all.
    fn count_triggers<const LIVING: bool>(&self, area: Aabb) -> usize {
        use entity::{Entity as _, for_each_entity};
        let mut count = 0;
        for_each_entity!(self.entities, entity => {
            if (!LIVING || entity.is_living())
                && entity.triggers_blocks()
                && area.intersects(entity.bounds())
            {
                count += 1;
            }
        });
        count
    }
    /// `DetectorRailBlock.getAnalogOutputSignal`: a powered rail reads the
    /// minecart on it, a command block cart first (its success count), else the first container cart's fullness.
    fn detector_rail_signal(&self, pos: BlockPos, powered: bool) -> i32 {
        if !powered {
            return 0;
        }
        let (x, y, z) = (f64::from(pos.x), f64::from(pos.y), f64::from(pos.z));
        let area = Aabb::new(
            Vec3::new(x + 0.2, y, z + 0.2),
            Vec3::new(x + 0.8, y + 0.8, z + 0.8),
        );
        let mut container = None;
        for ((lo, hi), cart) in self.cart_boxes.iter().zip(self.carts) {
            let bounds = Aabb::new(
                Vec3::new(lo[0], lo[1], lo[2]),
                Vec3::new(hi[0], hi[1], hi[2]),
            );
            if !area.intersects(bounds) {
                continue;
            }
            match cart.kind {
                minecarts::CartKind::CommandBlock => return cart.command.success_count,
                minecarts::CartKind::Chest | minecarts::CartKind::Hopper => {
                    // Counted here, for the cart a comparator reads, not for
                    // every cart on every block tick.
                    container = container.or_else(|| Some(self.cart_signal(cart)));
                }
                _ => {}
            }
        }
        container.unwrap_or(0)
    }
    /// A container minecart's comparator signal: its open menu's slots while
    /// someone has it open, else its own.
    fn cart_signal(&self, cart: &minecarts::Minecart) -> i32 {
        let key = (i32::MAX - 15, minecarts::CART_KEY_Y, cart.id as i32);
        let slots = match self.world.block_entities.get(&key) {
            Some(BlockEntity::Chest(open)) => open,
            _ => &cart.items,
        };
        minecarts::Minecart::container_signal(slots, self.items)
    }
    /// Every entity box that blocks building: the players who aren't
    /// spectators, the minecarts and the `Blockers` (a removed mob or a marker
    /// armor stand, whose box is empty, doesn't).
    fn building_blocker_boxes(&self) -> Vec<Aabb> {
        let boxed = |(lo, hi): &([f64; 3], [f64; 3])| {
            Aabb::new(
                Vec3::new(lo[0], lo[1], lo[2]),
                Vec3::new(hi[0], hi[1], hi[2]),
            )
        };
        let b = self.entities;
        self.player_box
            .iter()
            .chain(
                self.building_player_boxes
                    .as_ref()
                    .unwrap_or(&self.guest_boxes),
            )
            .chain(&self.cart_boxes)
            .map(boxed)
            .chain(b.mobs.iter().filter(|m| !m.removed).map(Mob::bounds))
            .chain(
                b.armor_stands
                    .iter()
                    .filter(|s| !s.marker)
                    .map(|s| s.bounds()),
            )
            .chain(b.boats.iter().map(boats::Boat::bounds))
            .chain(self.crystals.iter().map(dragon::Crystal::bounds))
            .chain(b.tnts.iter().map(tnt::PrimedTnt::bounds))
            .chain(
                b.falling
                    .iter()
                    .map(|f| horizon_client::falling_block::bounds(f.position)),
            )
            .collect()
    }
}
impl LevelAccess for GameLevel<'_> {
    fn redstone_experiments(&self) -> bool {
        self.redstone_experiments
    }
    fn allow_hopper_pickup_item(&self, pos: BlockPos, item: &str, count: u32) -> bool {
        let mut cancel = false;
        if self.native_authoritative {
            self.native_hopper_pickup_item(self.dimension, pos, item, count, &mut cancel);
        }
        !cancel
    }
    fn observe_block_changed(&self, pos: BlockPos, old: BlockStateId, new: BlockStateId) {
        if self.native_authoritative {
            self.native_block_changed(self.dimension, pos, old, new);
        }
    }
    fn neighbor_notify(&self, pos: BlockPos, skipped: Option<Direction>) -> bool {
        if !self.native_authoritative
            || !mod_api::__private::listened(mod_api::HookId::NeighborNotify)
        {
            return false;
        }
        let mut cancel = false;
        let state = self.block_state(pos);
        self.native_neighbor_notify(self.dimension, pos, state, skipped, &mut cancel);
        cancel
    }
    fn observe_neighbor_changed(
        &self,
        ticks: &mut Ticks,
        pos: BlockPos,
        state: BlockStateId,
        changed: horizon_world::level::BlockId,
        moved: bool,
    ) {
        if self.native_authoritative {
            self.native_neighbor_changed(ticks, self.dimension, pos, state, changed, moved);
        }
    }
    fn affect_neighbors_after_removal(
        &self,
        pos: BlockPos,
        state: BlockStateId,
        moved: bool,
        updates: &mut Vec<horizon_core::neighbour_update::NeighbourUpdate>,
    ) {
        if self.native_authoritative {
            self.native_affect_neighbors_after_removal(self.dimension, pos, state, moved, updates);
        }
    }
    fn allow_spread(
        &self,
        kind: horizon_world::level::Spread,
        from: BlockPos,
        to: BlockPos,
    ) -> bool {
        let mut canceled = false;
        if self.native_authoritative {
            let kind = match kind {
                horizon_world::level::Spread::Fluid => "fluid",
                horizon_world::level::Spread::Fire => "fire",
            };
            self.native_block_spread(self.dimension, kind, from, to, &mut canceled);
        }
        !canceled
    }
    fn fluid_place_block(
        &self,
        pos: BlockPos,
        liquid_pos: BlockPos,
        state: BlockStateId,
    ) -> Option<BlockStateId> {
        let mut new_state = state;
        let mut cancel = false;
        if self.native_authoritative {
            self.native_fluid_place_block(
                self.dimension,
                pos,
                liquid_pos,
                state,
                &mut new_state,
                &mut cancel,
            );
        }
        (!cancel).then_some(new_state)
    }
    fn allow_fluid_level_change(
        &self,
        pos: BlockPos,
        fluid: &'static str,
        from: i32,
        to: i32,
    ) -> bool {
        let mut cancel = false;
        if self.native_authoritative {
            self.native_fluid_level_change(self.dimension, pos, fluid, from, to, &mut cancel);
        }
        !cancel
    }
    fn create_fluid_source(&self, pos: BlockPos, state: BlockStateId, vanilla: bool) -> bool {
        let mut can_convert = vanilla;
        if self.native_authoritative {
            self.native_create_fluid_source(self.dimension, pos, state, &mut can_convert);
        }
        can_convert
    }
    fn allow_block_ignite(&self, pos: BlockPos, cause: &'static str) -> bool {
        let mut cancel = false;
        if self.native_authoritative {
            self.native_block_ignite(self.dimension, self.native_player, pos, cause, &mut cancel);
        }
        !cancel
    }
    fn block_grow_feature<'a>(
        &mut self,
        pos: BlockPos,
        feature: Option<&'a str>,
    ) -> Option<std::borrow::Cow<'a, str>> {
        if !self.native_authoritative
            || !mod_api::__private::listened(mod_api::HookId::BlockGrowFeature)
        {
            return feature.map(std::borrow::Cow::Borrowed);
        }
        let mut feature = feature.map(str::to_owned);
        let mut cancel = false;
        self.native_block_grow_feature(self.dimension, pos, &mut feature, &mut cancel);
        feature.filter(|_| !cancel).map(std::borrow::Cow::Owned)
    }
    fn crop_grow_pre(&mut self, pos: BlockPos, state: BlockStateId, vanilla: bool) -> bool {
        if !self.native_authoritative || !mod_api::__private::listened(mod_api::HookId::CropGrowPre)
        {
            return vanilla;
        }
        let mut result = mod_api::NativeTriState::Default;
        self.native_crop_grow_pre(self.dimension, pos, state, &mut result);
        result.or(vanilla)
    }
    fn crop_grow_post(&mut self, pos: BlockPos, original: BlockStateId) {
        if self.native_authoritative && mod_api::__private::listened(mod_api::HookId::CropGrowPost)
        {
            let state = self.block_state(pos);
            self.native_crop_grow_post(self.dimension, pos, original, state);
        }
    }
    fn allow_block_burn(&self, pos: BlockPos) -> bool {
        let mut cancel = false;
        if self.native_authoritative {
            self.native_block_burn(self.dimension, pos, &mut cancel);
        }
        !cancel
    }
    fn allow_leaves_decay(&self, pos: BlockPos) -> bool {
        let mut cancel = false;
        if self.native_authoritative {
            self.native_leaves_decay(self.dimension, pos, &mut cancel);
        }
        !cancel
    }
    fn allow_moisture_change(&self, pos: BlockPos, from: i32, to: i32) -> bool {
        let mut cancel = false;
        if self.native_authoritative {
            self.native_moisture_change(self.dimension, pos, from, to, &mut cancel);
        }
        !cancel
    }
    fn observe_sponge_absorb(&self, pos: BlockPos) {
        if self.native_authoritative {
            self.native_sponge_absorb(self.dimension, pos);
        }
    }
    fn allow_block_fade(&self, pos: BlockPos, from: BlockStateId, to: BlockStateId) -> bool {
        let mut cancel = false;
        if self.native_authoritative {
            self.native_block_fade(self.dimension, pos, from, to, &mut cancel);
        }
        !cancel
    }
    fn allow_block_form(&self, pos: BlockPos, from: BlockStateId, to: BlockStateId) -> bool {
        let mut cancel = false;
        if self.native_authoritative {
            self.native_block_form(self.dimension, pos, from, to, &mut cancel);
        }
        !cancel
    }
    fn note_block_change(
        &self,
        pos: BlockPos,
        instrument: u8,
        old: i32,
        mut new: i32,
    ) -> Option<i32> {
        let mut cancel = false;
        if self.native_authoritative {
            let context = mod_api::NativeNoteBlockChange {
                dimension: self.dimension,
                pos,
                instrument: mod_api::NativeNoteInstrument::ALL[usize::from(instrument)],
                old_note: old,
            };
            self.native_note_block_change(&context, &mut new, &mut cancel);
        }
        (!cancel).then_some(new)
    }
    fn note_block_play(
        &self,
        pos: BlockPos,
        by_player: bool,
        instrument: &mut u8,
        note: &mut i32,
    ) -> bool {
        use mod_api::NativeNoteInstrument;
        if !self.native_authoritative
            || !mod_api::__private::listened(mod_api::HookId::NoteBlockPlay)
        {
            return true;
        }
        let mut played_instrument = NativeNoteInstrument::ALL[usize::from(*instrument)];
        let mut played = *note;
        let mut cancel = false;
        let (player, player_entity_id) = if by_player {
            (self.native_player, self.native_player_entity)
        } else {
            (None, None)
        };
        self.native_note_block_play(mod_api::__private::NoteBlockPlayCall {
            dimension: self.dimension,
            player,
            player_entity_id,
            pos,
            instrument: &mut played_instrument,
            note: &mut played,
            cancel: &mut cancel,
        });
        *instrument = played_instrument as u8;
        // A note out of 0 to 24 keeps the block's own.
        if (0..=24).contains(&played) {
            *note = played;
        }
        !cancel
    }
    fn allow_portal_spawn(
        &self,
        pos: BlockPos,
        state: BlockStateId,
        shape: &horizon_world::level::PortalShape,
    ) -> bool {
        let mut cancel = false;
        if self.native_authoritative && mod_api::__private::listened(mod_api::HookId::PortalSpawn) {
            let b = shape.bottom_left;
            let shape =
                mod_api::NativePortalShape::from(mod_api::__private::built::NativePortalShape {
                    x_axis: shape.x_axis,
                    bottom_left: [b.x, b.y, b.z],
                    width: shape.width,
                    height: shape.height,
                });
            self.native_portal_spawn(self.dimension, pos, state, shape, &mut cancel);
        }
        !cancel
    }
    fn allow_bell_ring(&self, pos: BlockPos) -> bool {
        let mut cancel = false;
        if self.native_authoritative {
            self.native_bell_ring(self.dimension, self.native_player, pos, &mut cancel);
        }
        !cancel
    }
    fn allow_cauldron_level_change(
        &self,
        pos: BlockPos,
        from: i32,
        to: i32,
        reason: &'static str,
    ) -> bool {
        let mut cancel = false;
        if self.native_authoritative {
            self.native_cauldron_level_changed(mod_api::__private::CauldronLevelChangedCall {
                dimension: self.dimension,
                player: self.native_player,
                pos,
                from_level: from,
                to_level: to,
                reason,
                cancel: &mut cancel,
            });
        }
        !cancel
    }
    fn allow_piston_move(
        &self,
        piston: BlockPos,
        direction: horizon_core::geometry::Direction,
        extending: bool,
        push: &[BlockPos],
        destroy: &[BlockPos],
    ) -> bool {
        let mut canceled = false;
        if self.native_authoritative {
            let step = direction.normal();
            self.native_piston_pre(mod_api::__private::PistonPreCall {
                dimension: self.dimension,
                piston,
                direction: step,
                extending,
                push,
                destroy,
                canceled: &mut canceled,
            });
        }
        !canceled
    }
    fn observe_piston_moved(
        &self,
        piston: BlockPos,
        direction: horizon_core::geometry::Direction,
        extending: bool,
        push: &[BlockPos],
        destroy: &[BlockPos],
    ) {
        if self.native_authoritative {
            let step = direction.normal();
            self.native_piston_post(self.dimension, piston, step, extending, push, destroy);
        }
    }
    fn adjust_signal_source(&self, state: BlockStateId, vanilla: bool) -> bool {
        let mut source = vanilla;
        if self.native_authoritative {
            self.native_is_signal_source(state, &mut source);
        }
        source
    }
    fn has_weak_power_hook(&self) -> bool {
        self.native_authoritative
            && mod_api::__private::has_scoped_hook(mod_api::HookId::ShouldCheckWeakPower)
    }
    fn adjust_should_check_weak_power(
        &self,
        pos: BlockPos,
        state: BlockStateId,
        direction: Direction,
        vanilla: bool,
    ) -> bool {
        let mut check = vanilla;
        self.native_should_check_weak_power(self.dimension, pos, state, direction, &mut check);
        check
    }
    fn has_redstone_wire_connection_hook(&self) -> bool {
        self.native_authoritative
            && mod_api::__private::has_scoped_hook(mod_api::HookId::ShouldRedstoneWireConnectTo)
    }
    fn adjust_should_redstone_wire_connect_to(
        &self,
        pos: BlockPos,
        state: BlockStateId,
        direction: Option<Direction>,
        vanilla: bool,
    ) -> bool {
        let mut connected = vanilla;
        self.native_should_redstone_wire_connect_to(
            self.dimension,
            pos,
            state,
            direction,
            &mut connected,
        );
        connected
    }
    fn has_connection_hook(&self) -> bool {
        self.native_authoritative
            && mod_api::__private::has_scoped_hook(mod_api::HookId::ConnectsTo)
    }
    fn adjust_connects_to(
        &self,
        kind: horizon_core::connection_kind::ConnectionKind,
        pos: BlockPos,
        state: BlockStateId,
        neighbour: BlockStateId,
        direction: Direction,
        face_solid: bool,
        vanilla: bool,
    ) -> bool {
        let mut connected = vanilla;
        self.native_connects_to(mod_api::__private::ConnectsToCall {
            dimension: self.dimension,
            pos,
            state,
            kind,
            neighbour,
            direction,
            face_solid,
            connected: &mut connected,
        });
        connected
    }
    fn adjust_signal(
        &self,
        pos: BlockPos,
        state: BlockStateId,
        direction: Direction,
        direct: bool,
        vanilla: i32,
    ) -> i32 {
        native_level_signals::adjust_signal(direction, vanilla, |face, signal| {
            if self.native_authoritative {
                self.native_block_signal(self.dimension, pos, state, face, direct, signal);
            }
        })
    }
    fn adjust_has_analog_output(&self, state: BlockStateId, vanilla: bool) -> bool {
        let mut output = vanilla;
        if self.native_authoritative {
            self.native_has_analog_output(state, &mut output);
        }
        output
    }
    fn adjust_analog_output(
        &self,
        pos: BlockPos,
        state: BlockStateId,
        direction: Direction,
        vanilla: i32,
    ) -> i32 {
        let mut output = vanilla;
        if self.native_authoritative {
            self.native_analog_output(self.dimension, pos, state, direction, &mut output);
        }
        output.clamp(0, 15)
    }
    fn before_random_tick(
        &mut self,
        ticks: &mut Ticks,
        pos: BlockPos,
        state: &mut BlockStateId,
        handled: &mut bool,
    ) {
        if self.native_authoritative {
            // Any block the hook writes moves the world's revision on.
            let revision = self.world.revision;
            self.native_random_tick(ticks, self.dimension, pos, *state, handled);
            if !*handled && self.world.revision != revision {
                *state = self.block_state(pos);
            }
        }
    }
    fn before_tick_block(
        &mut self,
        ticks: &mut Ticks,
        pos: BlockPos,
        state: &mut BlockStateId,
        handled: &mut bool,
    ) {
        if self.native_authoritative {
            // Any block the hook writes moves the world's revision on.
            let revision = self.world.revision;
            self.native_tick_block(ticks, self.dimension, pos, *state, handled);
            if !*handled && self.world.revision != revision {
                *state = self.block_state(pos);
            }
        }
    }
    fn before_update_shape(
        &self,
        ticks: &mut Ticks,
        pos: BlockPos,
        state: BlockStateId,
        direction: Direction,
        neighbour_pos: BlockPos,
        neighbour: BlockStateId,
        new: &mut Option<BlockStateId>,
    ) {
        if self.native_authoritative {
            self.native_update_shape(
                ticks,
                mod_api::__private::UpdateShapeCall {
                    dimension: self.dimension,
                    pos,
                    state,
                    direction,
                    neighbour_pos,
                    neighbour,
                    new,
                },
            );
        }
    }
    #[allow(clippy::option_option)]
    fn before_state_for_placement(
        &self,
        state: BlockStateId,
        context: &PlacementContext,
        fluid: FluidState,
        answer: &mut Option<Option<BlockStateId>>,
    ) {
        if self.native_authoritative
            && let Some(fluid) = native_world_read::api_fluid_state(fluid)
        {
            self.native_state_for_placement(self.dimension, state, context, fluid, answer);
        }
    }
    fn before_can_survive(&self, pos: BlockPos, state: BlockStateId, survives: &mut Option<bool>) {
        if self.native_authoritative {
            self.native_can_survive(self.dimension, pos, state, survives);
        }
    }
    fn before_may_place_on(
        &self,
        state: BlockStateId,
        ground: BlockStateId,
        ground_pos: BlockPos,
        may: &mut Option<bool>,
    ) {
        if self.native_authoritative {
            self.native_may_place_on(self.dimension, state, ground, ground_pos, may);
        }
    }
    fn is_client_side(&self) -> bool {
        self.client_side
    }
    fn is_valid_bonemeal_target(&self, pos: BlockPos, state: BlockStateId) -> Option<bool> {
        // Asked on both sides, as vanilla asks `isValidBonemealTarget`: a
        // client predicts the use with it. Only a mod answers.
        let mut valid = None;
        self.native_is_valid_bonemeal_target(self.dimension, pos, state, &mut valid);
        valid
    }
    fn is_bonemeal_success(&mut self, pos: BlockPos, state: BlockStateId) -> bool {
        let mut success = false;
        if self.native_authoritative {
            self.native_is_bonemeal_success(self.dimension, pos, state, &mut success);
        }
        success
    }
    fn perform_bonemeal(&mut self, ticks: &mut Ticks, pos: BlockPos, state: BlockStateId) {
        if self.native_authoritative {
            self.native_perform_bonemeal(ticks, self.dimension, pos, state);
        }
    }
    fn before_fire_odds(&self, state: BlockStateId, odds: &mut Option<(i32, i32)>) {
        if self.native_authoritative {
            self.native_fire_odds(self.dimension, state, odds);
        }
    }
    fn before_smoke_source(&self, state: BlockStateId, smoke: &mut Option<bool>) {
        if self.native_authoritative {
            self.native_smoke_source(self.dimension, state, smoke);
        }
    }
    fn world_surface(&self, x: i32, z: i32) -> i32 {
        self.world.world_surface(x, z, |s| self.facts.is_air(s))
    }
    fn precipitation(&self, pos: BlockPos) -> Option<horizon_world::level::Precipitation> {
        self.behaviours.precipitation_info(self, pos)
    }
    fn is_raining_at(&self, pos: BlockPos) -> bool {
        self.behaviours.is_raining_at(self, pos)
    }
    /// `ContainerOpenersCounter.recheckOpeners` for the block entity at `pos`,
    /// through `ChestBlock.tick`, `EnderChestBlock.tick` and `BarrelBlock.tick`:
    /// the count a recount of the players with that container open finds, and
    /// the count it replaces. Only the block entities that keep a counter and
    /// a scheduled recheck answer; anything else is `None`.
    fn container_open(&self, pos: BlockPos) -> bool {
        self.open_containers.contains(&(pos.x, pos.y, pos.z))
    }
    fn recheck_container_openers(&mut self, pos: BlockPos) -> Option<(i32, i32)> {
        use horizon_world::level::classes::Class;
        let data = self.behaviours.data(self.world.block(pos.x, pos.y, pos.z));
        if !(data.is(Class::Chest) || data.is(Class::EnderChest) || data.is(Class::Barrel)) {
            return None;
        }
        Some(crate::game::block_entity_anims::recheck_opener_count(
            self.lid_openers,
            self.open_containers,
            (pos.x, pos.y, pos.z),
        ))
    }

    fn block_entity_analog_signal(
        &self,
        pos: BlockPos,
        state: BlockStateId,
        direction: Direction,
    ) -> i32 {
        if let Some(s) = self.registry.state(state)
            && s.name == "minecraft:detector_rail"
        {
            return self.detector_rail_signal(
                pos,
                s.properties.get("powered").is_some_and(|v| v == "true"),
            );
        }
        if let Some(signal) = command_blocks::analog_signal(self.world, (pos.x, pos.y, pos.z)) {
            return signal;
        }
        if let Some(signal) = sculk::sensor_signal(self.world, self.behaviours, state, pos) {
            return signal;
        }
        if let Some((_, comparator)) = jukebox::jukebox_signals(self.world, self.items, pos) {
            return comparator;
        }
        if let Some(signal) = creaking::heart_signal(self.world, pos) {
            return if self.registry.state(state).is_some_and(|s| {
                s.properties
                    .get("creaking_heart_state")
                    .is_some_and(|v| v == "uprooted")
            }) {
                0
            } else {
                signal
            };
        }
        if let Some(signal) = books::lectern_signal(self.world, pos) {
            return signal;
        }
        if let Some(signal) =
            machines::shelf_signal(self.world, self.registry, state, pos, direction)
        {
            return signal;
        }
        machines::container_signal(self.world, self.behaviours, self.items, self.registry, pos)
    }
    fn item_frame_analog_output(&self, pos: BlockPos, facing: Direction) -> Option<i32> {
        let mut frames = self
            .item_frames
            .iter()
            .filter(|f| f.pos == [pos.x, pos.y, pos.z] && f.facing == facing as u8);
        let frame = frames.next().filter(|_| frames.next().is_none())?;
        // ItemFrame.getAnalogOutput.
        Some(
            frame
                .item
                .as_ref()
                .map_or(0, |_| i32::from(frame.rotation % 8) + 1),
        )
    }
    fn block_entity_signal(
        &self,
        pos: BlockPos,
        _state: BlockStateId,
        _direction: Direction,
    ) -> i32 {
        native_level_signals::block_entity_signal(self.world, self.items, self.lid_openers, pos)
    }
    fn biome_name(&self, pos: BlockPos) -> Option<&str> {
        let chunk = self.world.chunks.get(&(pos.x >> 4, pos.z >> 4))?;
        Some(&self.biomes.get(chunk.biome(pos.x, pos.y, pos.z)).name)
    }
    fn world_seed(&self) -> i64 {
        self.seed
    }
    fn day_time(&self) -> i64 {
        self.world.day_time
    }
    fn game_rules(&self) -> horizon_world::level::GameRules {
        self.rules
    }
    fn difficulty(&self) -> i32 {
        self.difficulty
    }
    /// `ServerLevel.canSpreadFireAround`: a player (not a spectator) closer
    /// than the `fire_spread_radius_around_player` rule to the block's corner
    /// (`ChunkMap.anyPlayerCloseEnoughTo`); -1 spreads everywhere.
    fn can_spread_fire_around(&self, pos: BlockPos) -> bool {
        let radius = self.rules.fire_spread_radius_around_player;
        radius == -1
            || self.fire_players.iter().any(|p| {
                let d = [
                    p[0] - f64::from(pos.x),
                    p[1] - f64::from(pos.y),
                    p[2] - f64::from(pos.z),
                ];
                d[0] * d[0] + d[1] * d[1] + d[2] * d[2] < f64::from(radius) * f64::from(radius)
            })
    }
    fn increased_fire_burnout(&self, pos: BlockPos) -> bool {
        self.world
            .chunks
            .get(&(pos.x >> 4, pos.z >> 4))
            .is_some_and(|chunk| {
                self.biomes
                    .get(chunk.biome(pos.x, pos.y, pos.z))
                    .increased_fire_burnout
            })
    }
    fn block_state(&self, pos: BlockPos) -> BlockStateId {
        self.world.block(pos.x, pos.y, pos.z)
    }
    fn set_raw(&mut self, pos: BlockPos, state: BlockStateId) -> Option<BlockStateId> {
        // `Level.setBlock`: a server level of the Debug preset never changes.
        if self.debug && !self.client_side {
            return None;
        }
        if !self.world.loaded(pos.x, pos.z)
            || pos.y < self.world.bottom_y()
            || pos.y >= self.world.top_y()
        {
            return None;
        }
        let previous = self.world.block(pos.x, pos.y, pos.z);
        self.world
            .set_with_notification(pos.x, pos.y, pos.z, state, false)
            .ok()?;
        Some(previous)
    }
    fn send_block_updated(&mut self, pos: BlockPos) {
        if (self.client_side || self.tickets.block_ticking(pos.x >> 4, pos.z >> 4))
            && let Some(log) = &mut self.world.block_log
        {
            log.push((pos.x, pos.y, pos.z));
        }
    }
    fn is_loaded(&self, pos: BlockPos) -> bool {
        self.world.loaded(pos.x, pos.z)
    }
    fn min_y(&self) -> i32 {
        self.world.shape.min_y
    }
    fn max_y(&self) -> i32 {
        // Exclusive, as LevelAccess::max_y documents.
        self.world.shape.max_y()
    }
    fn random(&mut self) -> &mut horizon_core::random::LegacyRandom {
        &mut self.world.random
    }
    fn game_time(&self) -> i64 {
        self.world.game_time
    }
    fn block_light(&self, pos: BlockPos) -> i32 {
        i32::from(self.world.light(pos.x, pos.y, pos.z) & 15)
    }
    fn sky_light(&self, pos: BlockPos) -> i32 {
        i32::from(self.world.light(pos.x, pos.y, pos.z) >> 4)
    }
    fn sky_darken(&self) -> i32 {
        self.darken
    }
    fn sun_angle(&self, _pos: BlockPos) -> f32 {
        self.environment
            .sample(self.world.day_time, None, None)
            .sun_angle
    }
    fn is_raining(&self) -> bool {
        self.raining
    }
    fn is_thundering(&self) -> bool {
        self.thundering
    }
    fn dimension(&self) -> LevelDimension {
        self.dimension
    }
    /// `Level.isUnobstructed`: the placed state's collision shape against the player.
    /// `Level.isUnobstructed(state, pos, CollisionContext.of(player))`: no
    /// entity that blocks building is in the state's collision shape.
    fn is_unobstructed(&self, pos: BlockPos, state: BlockStateId) -> bool {
        let player = self.player_collision.as_ref();
        let context = player.map_or_else(
            || CollisionContext::empty(false),
            |player| {
                CollisionContext::for_entity(
                    CollisionEntity {
                        identity: EntityIdentity(0),
                        y: player.y,
                        descending: player.descending,
                        main_hand: Some(CollisionItem {
                            item: &player.main_hand_item,
                            count: player.main_hand_count,
                        }),
                    },
                    false,
                )
            },
        );
        let mut scene = collisions::SceneCollisions {
            world: &*self.world,
            binding: self.collision,
            subject: player.map(|player| horizon_client::collisions::PowderSnowSubject {
                fall_distance: player.fall_distance,
                falling_block: false,
                leather_boots: player.leather_boots,
                walkable_type: false,
                lava_walker: false,
            }),
        };
        let Ok(shape) = self
            .collision
            .collision_shape_with_context(&mut scene, state, pos, &context)
        else {
            return false;
        };
        if shape.is_empty() {
            return true;
        }
        let shape = shape.moved(horizon_core::geometry::Vec3::new(
            f64::from(pos.x),
            f64::from(pos.y),
            f64::from(pos.z),
        ));
        horizon_world::entity_obstruction::is_unobstructed(
            &mut BuildingObstruction(self.building_blocker_boxes()),
            None,
            &shape,
        )
        .unwrap_or(false)
    }
    /// `ServerLevel.isPositionTickingWithEntitiesLoaded`: block ticking, and
    /// a ticking chunk.
    fn is_ticking_chunk(&self, chunk: horizon_core::position::ChunkPos) -> bool {
        self.tickets.block_ticking(chunk.x, chunk.z) && ticking_chunk(self.world, chunk.x, chunk.z)
    }
    /// Entities that trigger pressure plates, tripwires, detector rails and
    /// wooden buttons: the players (the host and the LAN guests), the mobs,
    /// the minecarts and the other entities that don't ignore block triggers,
    /// and for `EntityQuery::Arrows` the arrows.
    fn count_entities(&self, area: Aabb, query: EntityQuery) -> usize {
        let hit = |(lo, hi): &([f64; 3], [f64; 3])| {
            area.intersects(Aabb::new(
                Vec3::new(lo[0], lo[1], lo[2]),
                Vec3::new(hi[0], hi[1], hi[2]),
            ))
        };
        let carts = || self.cart_boxes.iter().filter(|b| hit(b)).count();
        let others = || {
            self.crystals
                .iter()
                .map(dragon::Crystal::bounds)
                .chain(self.trigger_hooks.iter().map(|&(bounds, _)| bounds))
                .chain(self.trigger_extras.iter().copied())
                .filter(|b| area.intersects(*b))
                .count()
        };
        let arrows = || self.arrow_boxes.iter().filter(|b| hit(b)).count();
        let players = || {
            self.player_box
                .iter()
                .chain(&self.guest_boxes)
                .filter(|b| hit(b))
                .count()
        };
        match query {
            EntityQuery::Minecart => carts(),
            EntityQuery::Arrows => arrows(),
            EntityQuery::Mobs => players() + self.count_triggers::<true>(area),
            EntityQuery::Everything => players() + self.count_triggers::<false>(area) + others(),
        }
    }
    fn emit(&mut self, event: LevelEvent) {
        match event {
            // Block.dropResources draws its loot and spots right away.
            // `popResource` only drops server-side (`!level.isClientSide`):
            // a client's moveBlocks (a piston's block event) skips it.
            LevelEvent::DropResources { pos, state } if !self.client_side => {
                self.drop_resources_now(pos, state)
            }
            LevelEvent::DropResources { .. } => {}
            // `ClientLevel.playSeededSound` plays only when the excluded
            // player is this client's own: the block's `playSound(null, ...)`
            // (a piston's move, a note) reaches it from the server instead.
            LevelEvent::Sound { .. } if self.client_side => {}
            // NoteBlock.triggerEvent plays the head's sound with a null
            // player, so only the server's level plays it; a guest or a
            // remote client hears it from the sound packet.
            LevelEvent::BlockEvent {
                kind: "note_custom_sound",
                ..
            } if self.client_side => {}
            // Block.popResource of the block's own item (tall flowers, flower beds).
            LevelEvent::BlockEvent {
                pos,
                state,
                kind: "pop_block_item",
            } => self.pop_block_item(pos, state),
            // ComposterBlock.extractProduce: bone meal above the composter.
            LevelEvent::Player {
                pos,
                effect: PlayerEffect::DropItem(item),
                ..
            } => self.pop_above(pos, item),
            // Picking a sweet berry bush or cave vines: the harvest loot,
            // popped before the pick sound draws its pitch.
            LevelEvent::Player {
                pos,
                state,
                effect: PlayerEffect::InteractLoot(table),
            } => self.drop_interact_loot(pos, state, table),
            event => self.events.push(event),
        }
    }
    fn has_mod_block_entity(&self, pos: BlockPos) -> bool {
        // Asked on every block change: no lookup until a mod block entity exists.
        self.world.block_entities.has_mods()
            && matches!(
                self.world.block_entities.get(&(pos.x, pos.y, pos.z)),
                Some(BlockEntity::Mod(_))
            )
    }
    /// `ShulkerBoxBlockEntity.isClosed`: the lid state the block-entity tick
    /// copied from `be_anims` (`ShulkerBoxBlock.getBlockSupportShape`).
    fn shulker_is_closed(&self, pos: BlockPos) -> Option<bool> {
        self.world
            .shulkers
            .get(&[pos.x, pos.y, pos.z])
            .map(|s| s.status == horizon_client::block_entity_anim::ShulkerStatus::Closed)
    }
    fn remove_block_entity(&mut self, pos: BlockPos, old: BlockStateId, side_effects: bool) -> u8 {
        if let Some(change) = &mut self.command_block_transition
            && change.preserve(self.world, pos, old)
        {
            // Returning 0 skips remove_entity's side effects once; command blocks have no contents or drops.
            return 0;
        }
        self.remove_entity(pos, old, side_effects)
    }
    fn load_block_entity(&mut self, pos: BlockPos, mut nbt: horizon_core::nbt::Compound) {
        use horizon_core::nbt::Tag;
        for (k, v) in [("x", pos.x), ("y", pos.y), ("z", pos.z)] {
            nbt.insert(k.into(), Tag::Int(v));
        }
        let block = self.world.block(pos.x, pos.y, pos.z);
        let Some(block) = self.registry.state(block) else {
            return;
        };
        let name = &block.name;
        let tag = Tag::Compound(nbt);
        if let Some((key, entity)) = horizon_client::save::decode_block_entity_for_block(&tag, name)
        {
            self.world.block_entities.insert(key, entity);
            if tag.get("LootTable").is_some() {
                self.emit(LevelEvent::ContainerLootTable { pos, tag });
            }
        }
    }
    fn set_command_block_powered(&mut self, pos: BlockPos, powered: bool) -> Option<(bool, bool)> {
        command_blocks::set_powered(self.world, self.registry, (pos.x, pos.y, pos.z), powered)
    }
    /// `PotentSulfurBlockEntity.resetCountdown`.
    fn reset_potent_sulfur_countdown(&mut self, pos: BlockPos) {
        if let Some(BlockEntity::PotentSulfur(sulfur)) =
            self.world.block_entities.get_mut(&(pos.x, pos.y, pos.z))
        {
            sulfur.waiting_countdown = -1;
        }
    }
}
/// FallingBlockEntity carrying a block state.
#[derive(Clone, Debug)]
struct FallingBlock {
    /// `Entity.yRot` and `xRot` (`/tp`, `/data get ... Rotation`).
    pub rotation: [f32; 2],
    /// When it joined its list (`horizon_client::join_seq`); set by the list.
    pub(super) join: horizon_client::join_seq::JoinSeq,
    /// Its entity id: from the entity counter on the server, the server's on
    /// a client.
    id: u32,
    state: BlockStateId,
    /// `DATA_START_POS`: spawn position used to select weighted block models.
    start_pos: BlockPos,
    position: Vec3,
    previous: Vec3,
    velocity: Vec3,
    /// `FallingBlockEntity.time`: signed, a save can start it below zero.
    age: i32,
    /// `Entity.isNoGravity`.
    no_gravity: bool,
    /// `FallingBlockEntity.tick` handles portals after moving, before landing.
    portal: horizon_client::nether_portal::PortalState,
    /// `FallingBlockEntity.dropItem`.
    drop_item: bool,
    /// `Entity.fallDistance` (powder snow holds a block that fell far at 0.9).
    fall_distance: f64,
    /// `Entity.stuckSpeedMultiplier`: a cobweb it is in scales its next move.
    stuck: Vec3,
    damage: horizon_client::falling_block::Damage,
    /// The UUID and compound it was loaded with, kept while it moves.
    saved: Option<Box<([i32; 4], horizon_core::nbt::Tag)>>,
}
impl FallingBlock {
    /// Where `FallingBlockRenderer` draws the block's model corner, `alpha`
    /// into the tick: centred on the entity, then moved by `offset`, the
    /// block's random model offset (`ModelBlockRenderer.tesselateBlock`:
    /// `BlockState.getOffset` of `BlockPos.containing(x, maxY, z)`), so a
    /// pointed dripstone keeps the place it hung at while it falls.
    fn model_origin(
        &self,
        alpha: f64,
        offset: impl Fn(BlockStateId, i32, i32) -> Option<[f64; 3]>,
    ) -> glam::DVec3 {
        let [dx, dy, dz] = offset(
            self.state,
            self.position.x.floor() as i32,
            self.position.z.floor() as i32,
        )
        .unwrap_or([0.0; 3]);
        glam::DVec3::new(
            self.previous.x + (self.position.x - self.previous.x) * alpha - 0.5 + dx,
            self.previous.y + (self.position.y - self.previous.y) * alpha + dy,
            self.previous.z + (self.position.z - self.previous.z) * alpha - 0.5 + dz,
        )
    }
}

/// `FallingBlockRenderer.shouldRender` drops a falling block whose own state is
/// the state of the cell it is in (`level.getBlockState(entity.blockPosition())`):
/// a `NoGravity` block, or one that has just landed, sits inside an identical
/// block, and vanilla draws no model for it.
fn falling_block_visible(state: BlockStateId, cell: BlockStateId) -> bool {
    state != cell
}

/// The data loaded once at startup and never changed: blocks, items,
/// recipes, biomes, mobs, collision and motion. Read-only and `Send + Sync`,
/// so the client and an integrated server thread can share one copy.
pub struct Resources {
    core: WorldCore,
    rest: std::sync::OnceLock<WorldRest>,
}

struct WorldCore {
    item_data: Arc<horizon_world::item_defaults::ItemData>,
    facts: BlockFacts,
    biomes: Arc<Biomes>,
    environment: Environment,
    collision: Arc<BlockCollisionRegistry<'static>>,
    motion: MotionResources,
    suffocation: horizon_world::suffocation::SuffocationRegistry<'static>,
    spawn_registries: horizon_client::mobs::finalize::SpawnRegistries,
    threads: usize,
}

struct WorldRest {
    items: Items,
    recipes: Recipes,
    transformers: horizon_client::block_transform::Transformers,
    mob_types: HashMap<String, Arc<MobType>>,
    mob_data: horizon_client::mobs::MobData,
    providers: Providers,
    block_entity_render: BlockEntityRenderer,
    animate: horizon_client::animate_tick::AnimateTables,
    fog: horizon_client::fog::FogTables,
    music: horizon_client::music::MusicTables,
    ambient: horizon_client::ambience::AmbientTables,
    nether_environment: Environment,
    end_environment: Environment,
    behaviours: Behaviours,
    /// The player model's right and left arms.
    player_arm: [Option<horizon_client::mobs::Part>; 2],
    /// The slim model's right and left arms (3 pixels wide).
    player_arm_slim: [Option<horizon_client::mobs::Part>; 2],
    player_type: Option<Arc<horizon_client::mobs::MobType>>,
    /// `SpinAttackEffectModel` (Riptide's spinning boxes).
    spin_attack: Option<horizon_client::mobs::Part>,
    stations: horizon_client::stations::StationData,
}

/// The content tables a world's data packs change, loaded when the world
/// opens and on `/reload`: `ReloadableServerResources` loads a server's
/// recipes, loot tables and tags from the world's pack stack, over the
/// resource root's own. Worlds without packs use `Resources.rest` directly.
struct WorldTables {
    conditions: horizon_world::conditions::LoadConditions,
    suffocation: horizon_world::suffocation::SuffocationRegistry<'static>,
    /// The pack files the tables were built from (a native mod's among
    /// them), for the advancement tree reload and the loot data.
    files: Vec<(String, Vec<Vec<u8>>)>,
    items: Items,
    recipes: Recipes,
    stations: horizon_client::stations::StationData,
    reload_sources: Option<Arc<horizon_client::crafting::ResolvedSources>>,
    /// The values of the data map types native mods registered
    /// (`native_data_map_types`).
    data_maps: native_data_maps::DataMaps,
    /// The entity type, biome, structure and damage type tags with the packs'
    /// over the game's own, by registry (`TagLoader.loadTagsForExistingRegistries`):
    /// what the predicates' `#tag`s of those registries read.
    pack_tags: HashMap<&'static str, HashMap<String, HashSet<String>>>,
}

struct PreparedParts {
    blocks: Vec<mod_api::__private::ResolvedBlock>,
    tags: HashMap<String, horizon_world::level::data::BlockSet>,
    parts: Option<mod_api::Parts>,
}

struct PreparedWorldReload {
    tables: Option<WorldTables>,
    health: horizon_client::health::HealthResources,
    parts: PreparedParts,
    wire: Option<Arc<horizon_client::lan::data::SyncData>>,
}

impl WorldTables {
    /// The registries [`WorldTables::pack_tags`] holds.
    const PACK_TAG_REGISTRIES: [&'static str; 4] = [
        "entity_type",
        "worldgen/biome",
        "worldgen/structure",
        "damage_type",
    ];
}

impl Resources {
    fn rest(&self) -> &WorldRest {
        self.rest
            .get()
            .expect("world resources joined before entry")
    }
    fn facts(&self) -> &BlockFacts {
        &self.core.facts
    }
    fn item_defaults(&self) -> &horizon_world::item_defaults::ItemDefaults {
        self.core.item_data.defaults()
    }
    fn items(&self) -> &Items {
        &self.rest().items
    }
    fn recipes(&self) -> &Recipes {
        &self.rest().recipes
    }
    fn transformers(&self) -> &horizon_client::block_transform::Transformers {
        &self.rest().transformers
    }
    fn biomes(&self) -> &Arc<Biomes> {
        &self.core.biomes
    }
    fn environment(&self) -> &Environment {
        &self.core.environment
    }
    fn mob_types(&self) -> &HashMap<String, Arc<MobType>> {
        &self.rest().mob_types
    }
    fn mob_data(&self) -> &horizon_client::mobs::MobData {
        &self.rest().mob_data
    }
    fn providers(&self) -> &Providers {
        &self.rest().providers
    }
    fn block_entity_render(&self) -> &BlockEntityRenderer {
        &self.rest().block_entity_render
    }
    fn animate(&self) -> &horizon_client::animate_tick::AnimateTables {
        &self.rest().animate
    }
    fn fog(&self) -> &horizon_client::fog::FogTables {
        &self.rest().fog
    }
    fn music(&self) -> &horizon_client::music::MusicTables {
        &self.rest().music
    }
    fn ambient(&self) -> &horizon_client::ambience::AmbientTables {
        &self.rest().ambient
    }
    fn nether_environment(&self) -> &Environment {
        &self.rest().nether_environment
    }
    fn end_environment(&self) -> &Environment {
        &self.rest().end_environment
    }
    fn behaviours(&self) -> &Behaviours {
        &self.rest().behaviours
    }
    fn collision(&self) -> &BlockCollisionRegistry<'static> {
        &self.core.collision
    }
    fn suffocation(&self) -> &horizon_world::suffocation::SuffocationRegistry<'static> {
        &self.core.suffocation
    }
    fn motion(&self) -> &MotionResources {
        &self.core.motion
    }
    /// The variant registries finalizeSpawn selects from
    /// (`data/minecraft/<registry>`), shared with world generation.
    pub fn spawn_registries(&self) -> &horizon_client::mobs::finalize::SpawnRegistries {
        &self.core.spawn_registries
    }
    fn threads(&self) -> usize {
        self.core.threads
    }
    fn player_arm(&self, slim: bool, left: bool) -> &Option<horizon_client::mobs::Part> {
        let arms = if slim {
            &self.rest().player_arm_slim
        } else {
            &self.rest().player_arm
        };
        &arms[usize::from(left)]
    }
    fn player_type(&self) -> &Option<Arc<horizon_client::mobs::MobType>> {
        &self.rest().player_type
    }
    fn spin_attack(&self) -> &Option<horizon_client::mobs::Part> {
        &self.rest().spin_attack
    }
    fn stations(&self) -> &horizon_client::stations::StationData {
        &self.rest().stations
    }
}

/// The equipment slot of `armor[index]`: head, chest, legs, feet.
/// `doHurtEquipment`'s `durabilityDamage` before the cast to int: `max(1, damage / 4)`.
pub(super) fn durability_damage(damage: f32) -> f32 {
    (damage / 4.0).max(1.0)
}
/// The durability damage a mod left for `slot` (`NativeArmorHurt::new_damage`, cast to int
/// as vanilla casts it); an entry it removed keeps vanilla's `fallback`.
pub(super) fn answered_wear(
    entries: &[mod_api::NativeArmorHurt],
    slot: horizon_core::equipment_slot::EquipmentSlot,
    fallback: u32,
) -> u32 {
    entries
        .iter()
        .find(|e| e.slot == slot)
        .map_or(fallback, |e| e.new_damage as u32)
}

pub(super) fn armor_slot(index: usize) -> horizon_core::equipment_slot::EquipmentSlot {
    use horizon_core::equipment_slot::EquipmentSlot;
    match index {
        0 => EquipmentSlot::Head,
        1 => EquipmentSlot::Chest,
        2 => EquipmentSlot::Legs,
        _ => EquipmentSlot::Feet,
    }
}

/// Everything the title and menus draw: the GUI atlas with its font and the
/// title panorama. Kept apart from the world bundle so the title can appear
/// before the block sprites, models and gameplay tables are ready.
pub struct UiResources {
    /// The GUI, font, particle, map and skin atlas.
    pub atlas: HudAtlas,
    /// The title screen's panorama faces.
    pub panorama: Arc<horizon_client::gfx::panorama::Faces>,
}
impl UiResources {
    /// The resolved resource generation, including the base layer and pack filters.
    pub fn load(root: &horizon_assets::resource_source::ResourceSource) -> Result<Self> {
        // The title screen waits for both: the panorama's six faces decode
        // beside the atlas.
        std::thread::scope(|scope| {
            let panorama = named_scope(scope, "panorama", || {
                startup::time("ui.panorama", || {
                    horizon_client::gfx::panorama::Faces::load(root, root)
                })
            });
            let mod_image = horizon_client::gfx::native_render::world_drawn_hooked();
            let atlas = startup::time("ui.atlas", || {
                HudAtlas::build_over_with_image(root, root, mod_image)
            });
            let panorama = panorama
                .join()
                .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
            Ok(Self {
                atlas: atlas?,
                panorama: Arc::new(panorama),
            })
        })
    }
    /// What a server's game holds, since it draws nothing: the font, which books
    /// and chat measure with, and a one-pixel panorama.
    pub fn server(root: &horizon_assets::resource_source::ResourceSource) -> Result<Self> {
        Ok(Self {
            atlas: HudAtlas::fonts(root)?,
            panorama: Arc::new(horizon_client::gfx::panorama::Faces::colour([0; 4])),
        })
    }
}
/// What a resource pack reload replaces (`Minecraft.reloadResourcePacks`):
/// block models and sprites with their meshing tables, item models,
/// translations and the textures read on the CPU. Each game holds its own;
/// cloning shares the loaded data.
#[derive(Clone)]
pub struct PackResources {
    source: horizon_assets::resource_source::ResourceSource,
    data_source: horizon_assets::resource_source::ResourceSource,
    data_changes: horizon_assets::resource_source::ResourceSource,
    additional_data: Arc<[Arc<horizon_client::datapack::DataPack>]>,
    assets: Arc<Assets>,
    block_registries: Arc<horizon_client::command::args::Registries>,
    render: Arc<RenderAssets>,
    context: Arc<WorkerContext>,
    layer_names: Arc<HashMap<u32, String>>,
    /// Item definitions, baked item models and special renderers.
    item_renderer: Arc<horizon_client::item_render::ItemRenderer>,
    names: Arc<Names>,
    /// clouds.png: (width, height, filled cells).
    clouds: Arc<(u32, u32, Vec<bool>)>,
    /// misc/forcefield.png: (width, height, RGBA) for the world border wall.
    forcefield: Arc<(u32, u32, Vec<u8>)>,
}
impl PackResources {
    /// Everything a pack overlay at `root` provides, over `assets` and `render`
    /// loaded from it.
    fn load(
        root: &horizon_assets::resource_source::ResourceRoot,
        assets: Arc<Assets>,
        render: Arc<RenderAssets>,
        facts: &BlockFacts,
        biomes: Arc<Biomes>,
        light: Arc<LightTables>,
        item_data: &horizon_world::item_defaults::ItemData,
    ) -> Result<Self> {
        let visibility = models::load_visibility(root.prepared(), assets.registry)?;
        let tables = Arc::new(MeshTables::new(&assets, &render, facts, visibility)?);
        let item_renderer = horizon_client::item_render::ItemRenderer::load_with_library(
            root,
            render.model_library.clone(),
            item_data,
        )?;
        Ok(Self::assemble(
            root,
            assets,
            render,
            tables,
            item_renderer,
            biomes,
            light,
        ))
    }
    /// The workers' context over `biomes`: this pack's own when `biomes` is
    /// the table it was built with, else the same parts with that table.
    fn context_with(&self, biomes: Arc<Biomes>) -> Arc<WorkerContext> {
        let old = &self.context;
        if Arc::ptr_eq(&old.biomes, &biomes) {
            return old.clone();
        }
        Arc::new(WorkerContext {
            assets: old.assets.clone(),
            render: old.render.clone(),
            tables: old.tables.clone(),
            biomes,
            light: old.light.clone(),
            hidden_states: old.hidden_states.clone(),
        })
    }
    /// The pack tables over their parts, which `load_resources` builds on
    /// separate workers.
    fn assemble(
        root: &horizon_assets::resource_source::ResourceRoot,
        assets: Arc<Assets>,
        render: Arc<RenderAssets>,
        tables: Arc<MeshTables>,
        item_renderer: horizon_client::item_render::ItemRenderer,
        biomes: Arc<Biomes>,
        light: Arc<LightTables>,
    ) -> Self {
        Self {
            layer_names: Arc::new(
                render
                    .sprite_layers
                    .iter()
                    .map(|(name, layer)| (*layer, name.trim_start_matches("minecraft:").to_owned()))
                    .collect(),
            ),
            context: Arc::new(WorkerContext {
                assets: assets.clone(),
                render: render.clone(),
                tables,
                biomes,
                light,
                hidden_states: Arc::from([]),
            }),
            block_registries: native_block_states::block_registries(assets.registry),
            assets,
            render,
            item_renderer: Arc::new(item_renderer),
            source: root.source().clone(),
            data_source: root.data().clone(),
            data_changes: root.data_changes().clone(),
            additional_data: Arc::from([]),
            names: Arc::new(Names::load(root.source())),
            clouds: Arc::new(load_clouds(root.source())),
            forcefield: Arc::new(load_forcefield(root.source())),
        }
    }
}
/// A quick craft drag as the screen draws it: each dragged slot's count and
/// whether the slot caps it (None: hidden), and what stays in hand.
struct DragView {
    slots: HashMap<usize, Option<(u32, bool)>>,
    left: Option<u32>,
}
fn equipment_modifier_applies(group: &str, slot: &str) -> bool {
    group == slot
        || group == "any"
        || (group == "armor" && slot != "mainhand" && slot != "offhand")
        || (group == "hand" && (slot == "mainhand" || slot == "offhand"))
}
/// Ctrl+Q reads the physical keyboard, independently of sprint key mappings.
/// A chunk whose ticking future can complete (`ChunkMap.prepareTickingChunk`):
/// lit, with its neighbours loaded, as `postprocess_chunks` waits for.
fn ticking_chunk(world: &World, x: i32, z: i32) -> bool {
    let Some(center) = world.chunks.get(&(x, z)) else {
        return false;
    };
    if !center.lit {
        return false;
    }
    for dz in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dz == 0 {
                continue;
            }
            if !world.chunks.contains_key(&(x + dx, z + dz)) {
                return false;
            }
        }
    }
    true
}
/// Key events carry the scancode's US-layout code, which ignores XKB options like
/// `caps:swapescape`. For the keys whose meaning the keymap may remap, take the code
/// from the keysym the keymap resolved instead. This includes function keys on
/// remapped media rows; ordinary character bindings still use their physical key.
fn follow_keymap(event: &mut WindowEvent) {
    if let WindowEvent::KeyboardInput { event, .. } = event {
        follow_named_keymap(&event.logical_key, &mut event.physical_key);
    }
}

fn follow_named_keymap(logical_key: &winit::keyboard::Key, physical_key: &mut PhysicalKey) {
    use winit::keyboard::{Key, NamedKey};
    if let Key::Named(named) = logical_key {
        let code = match named {
            NamedKey::Escape => KeyCode::Escape,
            NamedKey::CapsLock => KeyCode::CapsLock,
            NamedKey::F1 => KeyCode::F1,
            NamedKey::F2 => KeyCode::F2,
            NamedKey::F3 => KeyCode::F3,
            NamedKey::F4 => KeyCode::F4,
            NamedKey::F5 => KeyCode::F5,
            NamedKey::F6 => KeyCode::F6,
            NamedKey::F7 => KeyCode::F7,
            NamedKey::F8 => KeyCode::F8,
            NamedKey::F9 => KeyCode::F9,
            NamedKey::F10 => KeyCode::F10,
            NamedKey::F11 => KeyCode::F11,
            NamedKey::F12 => KeyCode::F12,
            NamedKey::F13 => KeyCode::F13,
            NamedKey::F14 => KeyCode::F14,
            NamedKey::F15 => KeyCode::F15,
            NamedKey::F16 => KeyCode::F16,
            NamedKey::F17 => KeyCode::F17,
            NamedKey::F18 => KeyCode::F18,
            NamedKey::F19 => KeyCode::F19,
            NamedKey::F20 => KeyCode::F20,
            NamedKey::F21 => KeyCode::F21,
            NamedKey::F22 => KeyCode::F22,
            NamedKey::F23 => KeyCode::F23,
            NamedKey::F24 => KeyCode::F24,
            NamedKey::F25 => KeyCode::F25,
            NamedKey::F26 => KeyCode::F26,
            NamedKey::F27 => KeyCode::F27,
            NamedKey::F28 => KeyCode::F28,
            NamedKey::F29 => KeyCode::F29,
            NamedKey::F30 => KeyCode::F30,
            NamedKey::F31 => KeyCode::F31,
            NamedKey::F32 => KeyCode::F32,
            NamedKey::F33 => KeyCode::F33,
            NamedKey::F34 => KeyCode::F34,
            NamedKey::F35 => KeyCode::F35,
            _ => return,
        };
        *physical_key = PhysicalKey::Code(code);
    }
}

/// Q throws the whole stack with Control down: the same modifier word the
/// screens read (`has_control_down`), never a latched sprint toggle.
fn drop_whole_stack(modifiers: winit::keyboard::ModifiersState) -> bool {
    modifiers.control_key()
}

/// The modifier `modifiers` stands for, as GLFW words it in the native mods'
/// input hooks: `GLFW_MOD_SHIFT` 1, `GLFW_MOD_CONTROL` 2, `GLFW_MOD_ALT` 4,
/// `GLFW_MOD_SUPER` 8 — the word a NeoForge listener reads off its event
/// (`getModifiers`). The lock keys are not modifiers there.
fn glfw_modifiers(modifiers: winit::keyboard::ModifiersState) -> u32 {
    let mut bits = 0;
    if modifiers.shift_key() {
        bits |= 1;
    }
    if modifiers.control_key() {
        bits |= 2;
    }
    if modifiers.alt_key() {
        bits |= 4;
    }
    if modifiers.super_key() {
        bits |= 8;
    }
    bits
}

/// The GLFW mouse button index `button` is, as the native mods' mouse hooks
/// hand it over: 0 left, 1 right, 2 middle, 3 back, 4 forward. `Other(n)` is
/// winit's n, the platform's own number (X11 or evdev on Linux), so a button
/// past 4 is not GLFW's index everywhere. Buttons 0–4 are the numbering
/// `keybinds::mouse_name` names (`key.mouse.<index + 1>`) and vanilla's
/// `InputConstants` reads.
fn glfw_button(button: MouseButton) -> i32 {
    match button {
        MouseButton::Left => 0,
        MouseButton::Right => 1,
        MouseButton::Middle => 2,
        MouseButton::Back => 3,
        MouseButton::Forward => 4,
        MouseButton::Other(n) => i32::from(n),
    }
}

fn select_creative_tab(
    creative: &mut Creative,
    clicks: &mut horizon_client::container_screen::Clicks,
    tab: usize,
) {
    creative.select(tab);
    *clicks = Default::default();
    // Tabs are outside the panel; their release must not drop a carried stack.
    clicks.skip_release();
}

/// The icon the Select World list shows for the world in play
/// (`GameRenderer.tryTakeScreenshotIfNeeded`): its path in the save, and when
/// the last frame grab was asked for.
struct WorldIcon {
    path: PathBuf,
    last_attempt: Option<Instant>,
}

#[derive(Debug, PartialEq)]
struct QueuedLevelEvent {
    kind: i32,
    pos: [i32; 3],
    data: i32,
    source: Option<horizon_world::level::LevelEventPlayer>,
}

/// Physical and scaled GUI dimensions last reported by `window_resize`.
#[derive(Clone, Copy, PartialEq)]
struct ResizeState {
    size: winit::dpi::PhysicalSize<u32>,
    gui_size: [u32; 2],
}

impl ResizeState {
    /// Records `self` as the last report and says whether it differs from the previous one.
    fn differs_from(self, last: &mut Option<Self>) -> bool {
        last.replace(self) != Some(self)
    }
}

struct Game<'a> {
    /// Shared with an integrated server; `collision` and `motion` borrow from it.
    res: Arc<Resources>,
    rest_loader: Option<WorldRestLoader>,
    pending_entry: Option<PendingWorldEntry>,
    /// `DebugEntryChunkGeneration`'s `lastPos` and `result`: F3's `Density` line is sampled
    /// again only when the feet block (or the dimension) changes.
    debug_density: RefCell<debug_keys::DensityCache>,
    pending_pack_reload: bool,
    /// When `resourcepacks/` was last looked at for the open pack screen, and what it held
    /// (`watch_pack_folder`).
    pack_folder: Option<(
        std::time::Instant,
        Vec<(String, u64, Option<std::time::SystemTime>)>,
    )>,
    /// A pack reload staged on a worker (`LoadingOverlay`): the frame draws
    /// its progress until `drive_pack_reload` applies it.
    pack_reload: Option<packs::PackReload>,
    /// A server's resource packs (game/world/server_packs.rs).
    server_packs: server_packs::ServerPackState,
    /// The link a chat click asks to open (`ConfirmLinkScreen`).
    link_prompt: Option<String>,
    /// A command the player didn't type, waiting for their answer
    /// (game/network/remote_chat.rs).
    command_prompt: Option<remote_chat::CommandPrompt>,
    /// A server's Code of Conduct to accept (game/interface/code_of_conduct.rs).
    code_of_conduct: Option<code_of_conduct::Prompt>,
    dialogs: dialogs::DialogState,
    /// Single player's server while it loads its world (`Minecraft.doWorldLoad`).
    integrated_starting: Option<integrated::Starting>,
    world_renderer_ready: bool,
    collision: &'a BlockCollisionRegistry<'a>,
    motion: &'a MotionResources,
    /// This world's frozen damage definitions and reloadable tag snapshot.
    pub(super) health_resources: horizon_client::health::HealthResources,
    damage_tag_generation: u64,
    /// This game's resource packs (`reload_packs` replaces them).
    pack: PackResources,
    /// The title and menu bundle (GUI atlas, font and panorama).
    gui: UiResources,
    standalone_geometry: native_block_entity_draw::StandaloneGeometry,
    /// `FogRenderer`'s evolving state.
    fog: horizon_client::fog::FogState,
    screen_fx: horizon_client::screen_effects::ScreenEffects,
    /// `LocalPlayer.itemActivation`: the totem flying up to the screen.
    item_activation: horizon_client::item_activation::ItemActivation,
    weather_parameters: horizon_client::weather::ParameterCache,
    panorama_rotation: horizon_client::gfx::panorama::Rotation,
    world: World,
    /// The biome table the world plays with: the game's own with the world's
    /// packs over it (`Biomes::with_packs`), `None` before a world opens.
    world_biomes: Option<Arc<Biomes>>,
    /// `world_biomes` as the biome files made it, before `native_biome_colors`.
    world_biomes_base: Option<Arc<Biomes>>,
    streaming: Streaming,
    lod: Option<horizon_client::lod::Lod>,
    lod_source_attempted: bool,
    lod_observe: horizon_client::fast_hash::FxHashSet<(i32, i32)>,
    /// The server level far terrain keeps the chunks of.
    lod_level: Option<String>,
    /// The server's BlueMap far terrain draws from, when the player named one
    /// or Find BlueMap found it.
    lod_blue_map: Option<Arc<horizon_client::bluemap::BlueMap>>,
    /// Beacon beam sections kept after their chunk unloads, so far terrain
    /// LOD still draws their beams (`beacon_geometry` refreshes them).
    far_beacons: HashMap<(i32, i32, i32), Vec<(u32, i32)>>,
    /// The detail revision, centre and radius `Lod::update` last had covered for.
    lod_covered_at: Option<(u64, (i32, i32), i32)>,
    /// Whether the view's columns have all arrived, and how new it is: large
    /// translucent pages wait for both before they shrink (`set_terrain_loading`).
    view_arrival: horizon_client::streaming::ViewArrival,
    options: Options,
    player: Option<PlayerMotion>,
    spawn: Option<Vec3>,
    clock: FixedStep,
    previous_position: Vec3,
    /// `yRotO` / `xRotO` of the local player: the view at the start of the tick,
    /// moved by `Entity.turn` and a boat's `clampRotation` as the view is
    /// (`LocalPlayer.getRopeHoldPosition` eases the held lead from it).
    rope_rot_old: (f32, f32),
    eye: f32,
    previous_eye: f32,
    yaw: f32,
    pitch: f32,
    inventory: Vec<Option<ItemStack>>,
    selected: usize,
    carried: Option<ItemStack>,
    craft_grid: Vec<Option<ItemStack>>,
    craft_size: usize,
    /// The crafting table the 3x3 grid belongs to (`CraftingMenu.access`).
    craft_table: Option<[i32; 3]>,
    /// A quick craft drag over inventory slots: (button, game slots so far).
    inv_drag: Option<(i32, Vec<usize>)>,
    /// `AbstractContainerScreen`'s quick craft and skip-release state.
    inv_clicks: horizon_client::container_screen::Clicks,
    /// The Mouse Tweaks extra's drag and wheel (`tweaks`).
    tweaks: tweaks::Tweaks,
    /// The wheel and hovered slot of `BundleMouseActions`.
    bundle_mouse: bundles::BundleMouse,
    /// The last inventory click, for the double click that gathers a stack.
    last_inv_click: Option<(std::time::Instant, usize, bool)>,
    /// `lastQuickMoved`: what the last Shift click moved, which a Shift
    /// double click moves every stack of.
    last_quick_moved: Option<ItemStack>,
    /// Open container block entity, if any.
    container: Option<(i32, i32, i32)>,
    /// Open workstation menu (enchanting table, anvil, ...).
    station: Option<stations::Station>,
    /// Status effects and thrown potions (game/items/potions.rs).
    potions: potions::PotionState,
    scratch_slot: Option<ItemStack>,
    mining: Option<Mining>,
    /// The other players' cracks (`LevelRenderer.destroyingBlocks`): breaker id to the block,
    /// the stage (0..=9) and the game time it last changed.
    destroying: HashMap<i32, ([i32; 3], u8, i64)>,
    destroy_delay: u32,
    use_delay: u32,
    creative: bool,
    /// The game mode; `creative` mirrors `mode == Creative`.
    mode: horizon_client::command::args::GameMode,
    keys: horizon_client::held::HeldKeys,
    // Physical input is separate from remapped and toggled movement keys.
    physical_keys: HashSet<KeyCode>,
    /// Keys whose first press a mod cancelled (`key_mapping_pre`): their repeats are never held.
    rejected_keys: HashSet<KeyCode>,
    physical_buttons: HashSet<MouseButton>,
    drag_button: Option<MouseButton>,
    /// Control + left click as a right click (macOS).
    right_click_emulation: horizon_client_input::mouse::RightClickEmulation,
    left: bool,
    right: bool,
    cursor: (f32, f32),
    /// The cursor icon the window shows (`widgets::take_cursor` each frame).
    cursor_icon: horizon_client::widgets::Cursor,
    captured: bool,
    /// Native grab mode used by winit. Linux window managers can drop the
    /// grab without a focus event, leaving `captured` true but stale.
    capture_mode: Option<CursorGrabMode>,
    /// The window held the pointer when it lost focus: the system lets a
    /// grabbed pointer go with the focus, and a window that comes back still
    /// wanting it takes it again (`MouseHandler.grabMouse`, whose grab
    /// outlives the window's focus).
    regrab_on_refocus: bool,
    /// Only the local player owning keyboard and mouse changes the native pointer.
    keyboard_mouse: bool,
    /// Offscreen rendering into a visible splitscreen window still accepts native input.
    split_window: bool,
    /// This view's corner in its shared window; cursor warps include the offset.
    view_at: [u32; 2],
    /// Splitscreen: what it draws over this player's view (its badge, questions, notices).
    split_hud: split::Hud,
    /// Splitscreen: this player's own Microsoft account session (one of the launcher's other
    /// accounts), in place of the process's (P1's) for its connections and skin.
    split_auth: Option<std::sync::Arc<horizon_client::auth::AuthSession>>,
    focused: bool,
    /// Whether the window currently accepts input method events.
    ime_allowed: bool,
    /// The candidate window's area last given to the window, in pixels.
    ime_area: Option<[i32; 4]>,
    /// When the window lost focus, while it is still without
    /// (`Minecraft.lastActiveTime`: pausing waits 500 ms).
    focus_lost: Option<Instant>,
    /// The keyboard's modifier state (`KeyEvent.modifiers`).
    modifiers: winit::keyboard::ModifiersState,
    /// The window's fullscreen request and reported state.
    fullscreen: horizon_client::window::FullscreenState,
    /// The monitor whose video modes `settings.video_modes` holds.
    video_modes: horizon_client::window::VideoModeList,
    /// MouseHandler's accumulated look and wheel state.
    mouse: MouseLook,
    /// An agent moved the mouse (`mouse-move`) since the pointer was last
    /// grabbed or released: its motion turns the view like a grabbed pointer.
    agent_pointer: bool,
    /// `Options.smoothCamera` (cinematic camera), not persisted.
    smooth_camera: bool,
    /// Sneak, sprint, attack and use as ToggleKeyMappings.
    toggles: horizon_client::keymapping::ToggleKeys,
    /// A world screen with the pointer captured was open last frame.
    was_playing: bool,
    /// Minecraft.pause: a single-player screen that pauses the game is open.
    pause: bool,
    debug: bool,
    /// `horizon-ctl gpu-passes`: time Horizon's own passes on the GPU with F3 shut.
    gpu_passes: bool,
    screen: Screen,
    observed_screen: Option<native_hooks::ObservedScreen>,
    /// The layout the last frame drew the inventory's effect panels with.
    effects_layout: std::cell::Cell<Option<horizon_client::inventory_effects::EffectsLayout>>,
    window: Option<Arc<Window>>,
    /// What `window_resize` last reported: the physical size and the GUI settings behind the scaled size.
    resize_state: Option<ResizeState>,
    renderer: Option<Renderer>,
    /// Avoid an endless rebuild loop if the new device is lost as well.
    last_device_recovery: Option<Instant>,
    /// CPU geometry storage retained across frames, including empty/title frames.
    entity_vertices: Vec<Vertex>,
    /// Horizon Extras' dynamic lights this frame (`dynamic_lights.rs`), and how brightly each
    /// item glows (built the first time the extra is on).
    dynamic_lights: Vec<[f32; 4]>,
    luminance: std::cell::OnceCell<horizon_client::dynamic_light::Luminance>,
    /// Name tags drawn over everything (`RenderPipelines.TEXT_SEE_THROUGH`).
    see_through_vertices: Vec<Vertex>,
    /// The first-person player's model, which only a native pack's shadow pass draws.
    shadow_only_vertices: std::cell::RefCell<Vec<Vertex>>,
    /// Runs of `entity_vertices` by entity, for a shader pack's per-entity draws; recorded only
    /// while `pack_ids` is set (game/interface/shader_packs.rs).
    entity_spans: std::cell::RefCell<Vec<horizon_client::gfx::pack_draw::EntitySpan>>,
    /// Which entity this frame's entity vertices belong to (`Frame::entity_motion`).
    entity_motion: std::cell::RefCell<Vec<(u32, u64)>>,
    /// The GPU-posed vertex count at each of `entity_motion`'s marks (`Frame::entity_motion_posed`).
    entity_motion_posed: std::cell::RefCell<Vec<u32>>,
    /// Mob models posed on the GPU (`Frame::skin`, `mobs::skin`): the models stored there,
    /// and this frame's draws.
    skin_models: std::cell::RefCell<horizon_client::mobs::skin::SkinModels>,
    skin_frame: std::cell::RefCell<horizon_client::gfx::entity_skin::SkinFrame>,
    /// The glowing entities' silhouettes this frame (game/entities/glowing.rs).
    glowing_vertices: std::cell::RefCell<Vec<Vertex>>,
    /// Client mods' sounds (`play_local_sound`): those waiting for the next audio update
    /// (`tick_sound_engine`), the stops and fades asked of them, and their handles.
    mod_sounds: native_local_sounds::ModSounds,
    /// A client mod asked to clear the particles (`clear_particles`), done before
    /// the next frame or particle tick (`apply_mod_particle_clear`).
    mod_clear_particles: std::cell::Cell<bool>,
    /// The shader pack's entity and block entity ids, while one draws entities.
    pack_ids: Option<shader_packs::PackIds>,
    started: Instant,
    /// `SpectatorGui`: the menu a spectator opens with the hotbar keys.
    spectator_gui: horizon_client::spectator_menu::SpectatorGui,
    previous: Instant,
    /// When the last frame's work ended: what runs before the next one starts (`idle`, the
    /// event loop) shows in HORIZON_SPIKES as the time between frames.
    frame_end: Instant,
    /// FramerateLimitTracker.latestInputTime.
    last_input: Instant,
    /// Controllers (Horizon's own).
    pad: pad::Pads,
    frames: u64,
    /// The glint clock and options, read once at the start of each frame
    /// (game/interface/item_draw.rs).
    glint_frame: horizon_client::item_render::GlintFrame,
    /// Mod block entities' geometry their mods kept
    /// (game/modding/native_block_entity_draw.rs).
    kept_block_entities: std::cell::RefCell<native_block_entity_draw::KeptGeometry>,
    /// Prepared block-entity copies for this frame, in entity-stream order.
    prepared_model_draws: std::cell::RefCell<Vec<horizon_client::gfx::PreparedModelDraw>>,
    world_geometry: Option<horizon_client::world_geometry::WorldGeometry>,
    /// `DebugRenderer.renderers`: the ids `debug_renderer_draw` is asked about.
    debug_renderers: Vec<String>,
    /// `DebugRenderer.lastDebugEntriesVersion`: none before the first view of a world.
    debug_renderers_version: Option<u64>,
    /// The mod block entities' view distances, asked of the parts once (`native_block_entity_draw`).
    mod_view_distances: std::cell::OnceCell<native_block_entity_draw::ViewDistances>,
    /// `item_level` for one frame, game time, day time and dimension.
    item_level_cache: std::cell::RefCell<
        Option<(
            (u64, i64, i64, &'static str),
            horizon_assets::item_model::LevelState,
        )>,
    >,
    /// Frames drawn with the player in the world, for `--world-frames`.
    world_frames: u64,
    fps: f32,
    fps_frames: u64,
    fps_at: Instant,
    ticks: u64,
    /// The sprite animation clock (`TextureManager.tick`): counts only the ticks the level
    /// runs normally, so `/tick freeze` holds it and `/tick step` moves it on.
    texture_ticks: u64,
    health_hud: hud_layout::HealthHud,
    /// What ends the game with an error; kept as it is, so main can tell a debug crash.
    error: Option<Box<dyn std::error::Error>>,
    message: Option<(String, Instant)>,
    save: Arc<SaveDir>,
    level: Level,
    /// Must precede session_lock: dropping the writer drains writes while the world is locked.
    player_autosaves: horizon_client::lan::playerdata::Autosaves,
    session_lock: Option<horizon_client::save::SessionLock>,
    /// The prepared world is installed. The session lock is taken earlier so tag
    /// reads see ready tables, and is not this marker: a final save before the
    /// prepared `Level` is installed would replace the world with placeholder state.
    world_adopted: bool,
    audio: Audio,
    /// The side, registries and mod settings of this game's resource generation, which
    /// its menus lend to the hooks they fire inside a loop; rebuilt by `refresh_loop_context`.
    loop_context: Arc<horizon_world::native_load_context::LoopContext>,
    /// The native damage hooks every level of this game lends to its hits.
    damage_hooks: horizon_client::health::DamageHooks,
    music: horizon_client::music::MusicManager,
    /// The tracks native mods play over the music (`native_music_layers`).
    music_layers: horizon_client::music::MusicLayers,
    /// `ToastManager.nowPlayingToast`: there while Music Toast is not Never.
    now_playing: Option<horizon_client::toasts::NowPlayingToast>,
    /// The Music Toast option the toast was last set up for.
    music_toast_state: u8,
    music_notes: horizon_client::toasts::MusicNotes,
    ambience: horizon_client::ambience::Ambience,
    /// The sound engine was told the game paused (pauseAllExcept).
    sound_paused: bool,
    /// Minecarts whose `MinecartSoundInstance` is playing.
    cart_sounds: HashSet<u32>,
    /// The minecart the riding sounds were started for (`LocalPlayer.startRiding`).
    riding_cart_sounds: Option<u32>,
    /// The happy ghast its riding sound was started for (`LocalPlayer.startRiding`).
    riding_sound: Option<u32>,
    /// `LocalPlayer.wasFallFlying` for the elytra's flying sound.
    was_fall_flying: bool,
    /// Bees whose buzz is playing, and whether it is the aggressive one.
    bee_sounds: HashMap<u32, bool>,
    /// Guardians whose attack hum is playing.
    guardian_sounds: HashSet<u32>,
    dropped: horizon_client::join_seq::JoinList<ItemEntity>,
    next_entity: u32,
    /// The last entity id the running server tick found before its levels
    /// ticked: an item with a later one was added this tick and first ticks
    /// on the next (`EntityTickList`). `u32::MAX` until a tick has run.
    ticked_entities_through: u32,
    particles: horizon_client::particles::ParticleEngine,
    /// `ItemPickupParticle`s: taken items, orbs and arrows flying to their collector.
    pickups: pickups::Pickups,
    /// Sign text being edited (`SignEditScreen`).
    sign_edit: Option<sign_edit::SignEditor>,
    /// An open book, book and quill or lectern screen.
    book: Option<books::BookScreen>,
    /// The last click on the book screen, for double clicks.
    book_last_click: Option<std::time::Instant>,
    /// Lecterns whose page-turn pulse is running (ticks left).
    lectern_pulses: Vec<((i32, i32, i32), i32)>,
    /// LevelLoadingScreen.smoothedProgress while the player is not placed yet.
    loading_progress: f32,
    /// `CreateWorldScreen.openCreateWorldScreen`'s data read, on a worker,
    /// while the title says `createWorld.preparing`.
    creation_job: Option<std::thread::JoinHandle<std::result::Result<(), String>>>,
    /// When `loading_progress` last eased (the server thread's load, 50 ms a step).
    loading_progress_at: Option<std::time::Instant>,
    /// The world spawn search's origin chunk and fallback height, once known.
    spawn_origin: Option<((i32, i32), i32)>,
    /// `native_create_spawn_position` was asked, and a mod chose the spawn.
    spawn_hook_fired: bool,
    spawn_hook_cancelled: bool,
    spawn_spiral: horizon_client::world_spawn::ChunkSpiral,
    /// A new player's spawn search, while its candidates' chunks load.
    player_spawn: Option<horizon_client::world_spawn::PlayerSpawnFinder>,
    /// Chunks a respawn search waits for this tick, kept loading beside the
    /// player's own (taken by `update_world`).
    spawn_wait: Vec<(i32, i32)>,
    mobs: horizon_client::join_seq::JoinList<Mob>,
    /// `mobs` by network id, for the packets that name one (`game/mobs/mob_index.rs`).
    mob_index: mob_index::MobIndex,
    creatures: creatures::Creatures,
    sculk: sculk::SculkState,
    raiding: raids::Raiding,
    attack_ticks: u32,
    /// The main hand's item at the last player tick, whose attack damage and
    /// speed modifiers apply (`LivingEntity.detectEquipmentUpdates`): a hotbar
    /// swap just before an attack keeps them, with the new item's enchantments
    /// (attribute swapping).
    attack_item: Option<Arc<str>>,
    /// Mobs a player hurt (`LivingEntity.lastHurtByPlayer`), by player UUID.
    last_hurt_by_player: HashMap<u32, combat::PlayerCredit>,
    /// Leashed mobs a player killed, with the killer's UUID: the lead drops
    /// on the corpse's next tick (`Leashable.tickLeash`), for the mods. With
    /// the corpse's dimension: each level's tick prunes only its own.
    lead_killers: HashMap<u32, (u128, Dim)>,
    /// Animals a player fed into love (`Animal.loveCause`), with the guest it was by
    /// connection (None: the host). Breeding credits them, wherever they are.
    love_cause: HashMap<u32, Option<u32>>,
    /// Mobs whose death (`LivingEntity.die`) has been handled.
    /// Looked up per mob each tick: a fast hasher (never iterated).
    died: horizon_client::fast_hash::FxHashSet<u32>,
    /// Client-ticked block-entity animation state (`game/blocks/block_entity_anims.rs`).
    be_anims: horizon_client::block_entity_anim::BlockEntityAnims,
    /// Item being used (eaten): hotbar slot and ticks used.
    using: Option<(usize, u32)>,
    /// `getUseDuration` when that use started, as a mod left it
    /// (`game/items/item_use.rs`): consumables and mod items count to it.
    use_duration: i32,
    /// A mod's `Start` or `Tick` set that use's ticks left (`game/items/item_use.rs`).
    use_ticks_set: bool,
    /// The crossbow charge's start and middle loading sounds played so far
    /// (`game/items/crossbow.rs`): each charge plays each of them once, however
    /// far a mod's tick moved the charge at a time.
    crossbow_sounds: [bool; 2],
    /// Camera.oldFovModifier / fovModifier.
    /// `Camera` (eye height and field-of-view easing, view placement).
    view: horizon_client::camera::Camera,
    /// `FramerateLimiter` and its clock.
    limiter: horizon_client::window::FramerateLimiter,
    frame_clock: horizon_client::window::SystemClock,
    /// Hud.scopeScale (spyglass overlay size).
    scope_scale: f32,
    /// Duration of the last frame in ticks (GUI easing).
    frame_ticks: f32,
    title: Option<Title>,
    /// Armor slots: head, chest, legs, feet.
    armor: [Option<ItemStack>; 4],
    /// The off hand (Inventory slot 40, saved as slot 150; menu slot 504 here).
    offhand: Option<ItemStack>,
    /// The body and saddle slots (`EquipmentSlot.BODY`, `SADDLE`; Inventory
    /// slots 41 and 42, saved as 151 and 152). The hands and armor stay in
    /// the fields above. Nothing in the game reads these yet; they are kept
    /// for mods and saves.
    equipment: horizon_items::equipment::Equipment,
    /// `ItemStack.popTime` of the nine hotbar stacks, then the off hand: set to 5
    /// when a slot gains items, counted down each tick.
    hotbar_pop: [u8; 10],
    /// The hand an item use acts with (`InteractionHand`): the off hand only
    /// for its turn in `use_item` and while its item is being used.
    hand: Hand,
    /// The shield's item cooldown after an axe disabled it (ticks).
    shield_cooldown: u32,
    /// `Player.getCooldowns()`: the `use_cooldown` component's entries, what
    /// `itemDecorations` shades a slot by. A server's players have their own
    /// (`GuestPlay::cooldowns`), swapped in for their turn.
    cooldowns: horizon_client::cooldowns::Cooldowns,
    /// The source of a command run through /execute (the player otherwise).
    command_source: Option<commands::execute::Source>,
    native_command_filter: bool,
    titles: titles::Titles,
    /// `Hud.toolHighlightTimer`: the selected item name.
    tool_highlight: horizon_client::hud_overlays::ToolHighlight,
    /// `Hud.autosaveIndicatorValue`.
    saving_indicator: horizon_client::hud_overlays::SavingIndicator,
    /// This game's world is autosaving this tick (`MinecraftServer.isSaving`).
    autosaving: bool,
    /// `MinecraftServer.ticksUntilAutosave`: the ticks left before the next
    /// autosave, counted down a tick at a time.
    ticks_until_autosave: i64,
    applied_modifiers: Vec<(String, String)>,
    experience: Experience,
    orbs: horizon_client::join_seq::JoinList<ExperienceOrb>,
    xp_delay: u32,
    ordered: ordered_tick::OrderedTicks,
    level_up_sound: horizon_client::entities::LevelUpSound,
    /// The arm swing (`LivingEntity.SwingState`) and how many swings started.
    swing: horizon_client::first_person::SwingState,
    swings_started: u32,
    /// Swings of uses whose client predicts them (`SwingSource.PREDICTED`):
    /// a server does not send these back to the player who swung.
    predicted_swings: u32,
    /// The first-person equip heights and the items the hands show (with
    /// the slot each came from).
    heights: horizon_client::first_person::HandHeights,
    /// The hands' lag behind the view (`LocalPlayer.xBob`, `yBob`); a new
    /// LocalPlayer (joining, respawning, changing dimension) starts at 0.
    sway: horizon_client::first_person::HandSway,
    visible_hands: [(usize, Option<ItemStack>); 2],
    /// Whether the main and off hand's slots were set to a new stack that
    /// the hand doesn't show yet (`Game::hand_stack_replaced`).
    hands_replaced: [bool; 2],
    /// `Minecraft`'s attack handling, attack clicks since the last tick and
    /// the attack strength tickers the hand reads.
    clicks: horizon_client::first_person::Clicks,
    attack_clicks: u32,
    /// Use clicks since the last tick; `handleKeybinds` consumes them after the attack clicks.
    use_clicks: u32,
    hand_strength: horizon_client::attack_strength::AttackStrength,
    settings: Settings,
    tutorial: tutorial::Tutorial,
    tutorial_toasts: Vec<tutorial::HintToast>,
    /// The `SystemToastId.NARRATOR_TOGGLE` toast, wherever the player is.
    narrator_toast: Option<horizon_client::toasts::SystemToast>,
    /// The open screen's narration (`Screen`'s narration state and timing).
    narration: horizon_client::screen_narrator::ScreenNarrator,
    /// Where the options screen returns: true = title, false = pause menu.
    options_from_title: bool,
    dragging: bool,
    /// `ClientAvatarState`: walked distance and view bob.
    avatar: horizon_client::camera::AvatarBob,
    flash: u32,
    /// `ClientLevel.rainSoundTime`: ticks of rain since the last rain sound.
    rain_sound_time: i32,
    /// Bolts that struck here and sky flashes they made, in total (`status`).
    lightning_seen: [u32; 2],
    /// The head of the bed while sleeping; the visual timer also survives waking.
    sleeping: Option<[i32; 3]>,
    sleep_timer: u32,
    /// The beds players sleep in that this game marked `occupied`, by level
    /// (game/blocks/beds.rs).
    player_beds: Vec<(Dim, [i32; 3])>,
    /// `SleepStatus`'s active and sleeping players at the last announcement.
    sleep_seen: (i64, i64),
    dim: Dim,
    world_root: PathBuf,
    /// The world's data packs' content tables, when it has packs (`None`
    /// keeps `Resources.rest`'s own in use).
    world_tables: Option<WorldTables>,
    /// Server-owned loot and item snapshots, independent of client preloads.
    server_loot: horizon_client::structure_blocks::LootSnapshot,
    server_items: Option<Items>,
    /// The world is superflat (`ServerLevel.isFlat`, for every dimension).
    world_flat: bool,
    /// The world is the Debug preset (`Level.isDebug`, for every dimension): its blocks
    /// cannot be changed and nothing ticks.
    world_debug: bool,
    /// A server's players on their way to another level, each waiting for
    /// the destination to load around them.
    server_trips: Vec<server_travel::Trip>,
    /// Players a command sends to another level, once it has run.
    level_teleports: Vec<server_travel::LevelTeleport>,
    /// Teleports a command made inside a level, for `native_player_teleported`
    /// once the player's move has reached their client (`leave_guest`).
    teleport_notes: Vec<server_travel::TeleportNote>,
    /// A server's levels other than the one in place (game/world/levels.rs).
    levels: levels::Levels,
    /// Which pass of a server tick runs (`tick_server`): the first one reads
    /// the network, the last one ends the players' tick.
    level_pass: levels::Pass,
    /// Nether portals: the player's, items' and mobs' (game/blocks/nether_portal.rs).
    portals: nether_portal::Portals,
    /// The respawn point when it is not the world spawn or a bed: a respawn
    /// anchor (`game/blocks/respawn_anchor.rs`).
    respawn_config: Option<respawn_anchor::RespawnConfig>,
    /// The next enter_dimension call comes from an End portal.
    end_portal_travel: bool,
    /// The open WinScreen (game/interface/win.rs).
    win: Option<win::Win>,
    end: end::EndState,
    block_ticks: Ticks,
    tick_chunks: horizon_client::tick_chunk_membership::TickChunkMembership,
    tickets: horizon_client::ticket_levels::ChunkTickets,
    random_tick_sections: horizon_client::random_tick_sections::RandomTickSections,
    level_events: Vec<LevelEvent>,
    falling: horizon_client::join_seq::JoinList<FallingBlock>,
    /// Blocks the server shows for a moment (`ClientboundAddTransientBlockPacket`).
    transient_blocks: transient_blocks::TransientBlocks,
    paintings: horizon_client::join_seq::JoinList<horizon_client::entities::Painting>,
    painting_variants: paintings::PaintingVariants,
    item_frames: horizon_client::join_seq::JoinList<horizon_client::entities::ItemFrame>,
    frame_model: frames::FrameModel,
    glow_frame_model: frames::FrameModel,
    /// The `map=true` frame variants.
    map_frame_model: frames::FrameModel,
    glow_map_frame_model: frames::FrameModel,
    armor_stands: horizon_client::join_seq::JoinList<horizon_client::entities::ArmorStand>,
    /// Worn armor as of the last equipment check (frost walker).
    location_armor: [Option<ItemStack>; 4],
    /// `Cushion` decoration entities (game/entities/cushions.rs).
    cushions: horizon_client::join_seq::JoinList<horizon_client::entities::Cushion>,
    /// Entities of types Horizon does not simulate.
    others: horizon_client::join_seq::ResidentList<commands::OtherEntity>,
    armor_stand_assets: armor_stands::ArmorStandAssets,
    /// `WaypointStyleManager`.
    waypoint_styles: horizon_client::locator_bar::WaypointStyles,
    /// `ClientWaypointManager`: the locator bar's waypoints.
    waypoints: horizon_client::locator_bar::ClientWaypoints,
    /// The camera the last frame drew from (position, yaw, pitch, fov), for the locator bar.
    locator_camera: Option<([f64; 3], f32, f32, f32)>,
    native_camera_frame: Option<mod_api::NativeCamera>,
    /// The yaw and pitch a native mod's camera drew the last frame from (`native_camera_pose`),
    /// none when it drew from vanilla's.
    posed_rotation: Option<(f32, f32)>,
    /// `LocalPlayer.postEffects`: the chains the server has the screen run
    /// (`ClientboundPostEffectsPacket`), in order.
    post_effects: Vec<String>,
    /// `GameRenderer.spectatedEntityPostEffect` (`checkEntityPostEffect`).
    spectated_post_effect: Option<&'static str>,
    /// `LocalPlayer.experienceDisplayStartTick`: (last progress, seen a change, tick).
    experience_display: (f32, bool, u64),
    boats: horizon_client::join_seq::JoinList<boats::Boat, false>,
    /// The boats a mob's lead can hang on, for this tick's mob ticks.
    leash_boats: Vec<(u32, horizon_client::mobs::leash::HolderState)>,
    boat_models: boats::BoatModels,
    /// EndCrystalModel's baked layer.
    crystal_model: Option<horizon_client::mobs::Part>,
    /// CushionModel's baked layer.
    cushion_model: Option<horizon_client::mobs::Part>,
    /// Models drawn only once such an entity shows up (lead knots, wind charges...).
    entity_models: EntityModels,
    carts: horizon_client::join_seq::JoinList<minecarts::Minecart>,
    /// The boats and minecarts `native_vehicle_moved` follows (lan_vehicles.rs).
    watched_vehicles: Vec<lan_vehicles::WatchedVehicle>,
    /// Songs playing from jukeboxes (tracked sound handles).
    jukebox_sounds: HashMap<(i32, i32, i32), u64>,
    bolts: Vec<lightning::Bolt>,
    maps: maps::Maps,
    /// The ender dragon fight (game/mobs/dragon.rs).
    fight: dragon::Fight,
    dragon: Option<dragon::Dragon>,
    crystals: horizon_client::join_seq::JoinList<dragon::Crystal>,
    dragon_fireballs: Vec<dragon::DragonFireball>,
    breath_clouds: horizon_client::join_seq::ResidentList<dragon::Breath>,
    dragon_model: Option<horizon_client::mobs::Part>,
    /// Map textures waiting for the next HUD batch to upload them.
    map_patches: Vec<horizon_client::font::Patch>,
    name_tag_layouts: names::NameTagLayouts,
    text_displays: std::collections::HashMap<i32, text_displays::RemoteTextDisplay>,
    interactions: std::collections::HashMap<i32, interactions::RemoteInteraction>,
    /// The fence knots a server tracks to this client, by entity id, at their
    /// block (`LeashFenceKnotEntity`).
    remote_knots: HashMap<u32, [i32; 3]>,
    /// The entity ids a hosted server gives the knots its mobs hang on, by
    /// block, while a lead is tied there.
    knot_ids: HashMap<[i32; 3], u32>,
    text_display_vertices: usize,
    /// Player skin downloads, and which skin each atlas skin slot holds
    /// (least recently shown first).
    skin_downloads: RefCell<horizon_client::skins::Downloader>,
    /// `ProfileResolver.Cached`: `Dynamic` profiles (name- or id-only heads
    /// and mannequins) resolve through the services in the background.
    skin_profiles: RefCell<horizon_client::skins::Resolver>,
    skin_slots: horizon_client::skins::SkinSlots,
    /// 3D skin layers: each player's outer layer as one slab per painted
    /// texel, built once per (skin, slim) (`mobs::skin_layers`).
    skin_3d: RefCell<skin_models::SkinModels>,
    /// Atlas trim slots in use: (sprite name, slot), least recently drawn first.
    trim_slots: Vec<(String, usize)>,
    cart_models: minecarts::CartModels,
    /// The boat the player rides.
    riding: Option<u32>,
    /// Vehicles that gained a rider this tick whose `START_RIDING_TRIGGER` was
    /// not fired yet (`game/entities/boarding.rs`), for a guest's turn.
    started_riding: Vec<u32>,
    /// A rotation-only write (`/rotate`, `Entity.forceSetRotation`) to the player of this
    /// turn: `leave_guest` sends it as `ClientboundPlayerRotationPacket`, not a teleport.
    rotation_forced: bool,
    /// The mount the last `position_rider` put the player on, to tell the tick
    /// a seat is taken (`Entity.moveTo`, no camera sweep) from the ones that
    /// follow it (`positionRider`, interpolated as usual).
    posed_rider: Option<u32>,
    /// The vehicle the player's file names, waiting for it to be in the world
    /// (`ServerPlayer.loadAndSpawnParentVehicle`).
    pending_vehicle: Option<[i32; 4]>,
    /// The ender inventory (27 slots) and the ender chest showing it.
    ender_items: Vec<Option<ItemStack>>,
    ender_open: Option<(i32, i32, i32)>,
    /// Block entity data of broken blocks waiting for their dropped item.
    pending: spill::Pending,
    /// Item entities spawned and block entities removed while the block
    /// behaviour ran (spilled contents, popped resources).
    spill: spill::Spill,
    /// The recipe book of the open screen (game/items/recipe_book.rs).
    recipe_ui: recipe_book::RecipeUi,
    /// The horse inventory (game/interface/mount_menu.rs).
    mount_menu: mount_menu::MountMenu,
    /// The player's fishing hook.
    hook: Option<fishing::Hook>,
    /// The other players' hooks a server tracks to this client.
    remote_hooks: Vec<fishing::RemoteHook>,
    tridents: horizon_client::join_seq::JoinList<trident::ThrownTrident>,
    rockets: horizon_client::join_seq::JoinList<fireworks::Rocket>,
    /// Ominous trial spawners' item spawners.
    ominous_spawners:
        horizon_client::join_seq::ResidentList<ominous_item_spawner::OminousItemSpawner>,
    creaking: creaking::CreakingState,
    /// Monster spawners' display mobs (`game/mobs/spawner.rs`).
    spawners: spawner::SpawnerState,
    /// Reused candidate buffers for natural spawning.
    spawn_scratch: natural_spawning::SpawnScratch,
    trident_model: Option<horizon_client::mobs::Part>,
    arrow_model: Option<horizon_client::mobs::Part>,
    bee_stinger_model: Option<horizon_client::mobs::Part>,
    tnts: horizon_client::join_seq::JoinList<tnt::PrimedTnt>,
    hoppers: HashMap<(i32, i32, i32), machines::HopperState>,
    /// The open `host.transaction`s, innermost last (`transactions.rs`).
    transactions: Vec<transactions::Level>,
    /// Backing blocks whose contents notice is on the call stack, innermost last.
    contents_changing: Vec<(mod_api::LevelRef, BlockPos)>,
    /// The screen of a mod block's menu open on this client (`native_screen`).
    mod_screen: Option<native_hooks::OpenNativeScreen>,
    /// The players with each mod container open, by level (`tick_mod_container_viewers`).
    mod_container_viewers: HashMap<(&'static str, (i32, i32, i32)), u32>,
    /// Where item entities are, while the block entities tick (hoppers).
    item_columns: Option<machines::ItemColumns>,
    creative_screen: Creative,
    /// `CreativeModeInventoryScreen.ignoreTextInput`: the chat key opened the
    /// search tab, so the letter it carries is not typed into the box.
    creative_ignore_text: bool,
    control: Option<horizon_client::control::Server>,
    /// The Agent Control setting the socket was last opened or closed for (`agent_settings`).
    agent_control_on: bool,
    /// When an agent's last command came.
    agent_last: Option<Instant>,
    frame_stats: horizon_client::control::FrameStats,
    /// Control replies waiting for a rendered frame or a benchmark deadline.
    pending_screenshot: Option<std::sync::mpsc::Sender<String>>,
    /// `render-capture DIR`: replied once the renderer wrote the scene.
    pending_render_capture: Option<std::sync::mpsc::Sender<String>>,
    pending_bench: Option<(Instant, std::sync::mpsc::Sender<String>)>,
    /// Recent frame times for the performance overlay.
    frame_graph: horizon_client::perf_overlay::FrameGraph,
    /// Horizon's extras (minimap, auto-reconnect; mods' counters): `game/interface/extras.rs`.
    extras: extras::ExtrasState,
    /// The PvP & HUD options' state (`game/entities/pvp.rs`).
    pvp: pvp::PvpState,
    /// Horizon mods (`game/modding/mods/mod.rs`), none running unless turned on.
    mods: mods::ModsState,
    /// The in-game browser's worker, separate from the cloneable reload inputs.
    mod_jobs: mod_browser::Jobs,
    /// Every item and recipe, for mods (`game/modding/native_recipes.rs`), made once.
    recipe_tables: native_recipes::RecipeTables,
    /// The item panel beside container screens (`game/modding/native_panel.rs`).
    panel: native_panel::Panel,
    /// Horizon's own shader pack, while one is picked (`game/modding/native_shaders.rs`).
    native_shaders: native_shaders::NativeShaders,
    /// The items native client mods claim and the events they haven't polled (game/modding/native_claimed_items.rs).
    native_input: native_claimed_items::ModInput,
    /// The native lifecycle bookkeeping.
    native_lifecycle: native_lifecycle::NativeLifecycle,
    /// Entities whose copies are held out of storage; their handles refuse until stored back.
    held_out: Vec<u32>,
    /// Native mods' config files this game loaded (game/modding/native_configs.rs).
    native_configs: native_configs::NativeConfigs,
    mod_phase: mod_api::__private::PhaseOwner,
    /// Native mods' messages waiting (`game/modding/native_net.rs`).
    native_net: native_net::NativeNet,
    /// The mod whose own screen is open (`Screen::Mod`).
    native_mod_screen: Option<String>,
    /// Meshed sections waiting for an upload budget (Horizon settings).
    pending_uploads: Vec<((i32, i32, i32), horizon_client::mesher::SectionMesh)>,
    /// This frame's view frustum, for culling entities and block entities.
    view_frustum: Option<horizon_client::gfx::Frustum>,
    /// Whether the renderer's section-occlusion graph is active for this view.
    entity_section_visibility_valid: bool,
    /// The camera section in the current world layout.
    entity_section_camera: Option<(i32, i32, i32)>,
    sync_occlusion_culler: Option<horizon_client::voxel_occlusion::Culler>,
    async_occlusion_culler: Option<Box<occlusion_culling::AsyncOcclusionCuller>>,
    async_occlusion_culling_failed: bool,
    async_particle_culler: RefCell<particle_culling::ParticleAsyncState>,
    /// `Renderer::pack_shadow_casters` for this frame's entities: (keep what the culler hides,
    /// build the player's model in first person); (false, false) without a shader pack.
    pack_shadow_casters: (bool, bool),
    /// A scripted flight from the control socket (`fly`).
    flight: Option<control::Flight>,
    /// The running agent action from the control socket (game/runtime/agent/mod.rs).
    agent: agent::Agent,
    /// Widget commands waiting for the frame after a click (game/runtime/ui_control.rs).
    ui_wait: ui_control::UiWait,
    /// What happened, for `events` and `wait-for` (game/runtime/events.rs).
    events: events::Events,
    /// Who caused each input event: a person, an agent or the game (game/runtime/input_trace.rs).
    input_trace: input_trace::InputTrace,
    /// A repro recording, export or replay (game/runtime/repro.rs).
    repro: repro::Repro,
    /// `HORIZON_TICK_SCRIPT`'s scenario (game/world/tick_script.rs).
    tick_script: tick_script::TickScript,
    /// The overworld's last `sky_darken` and its inputs (time, rain, thunder).
    sky_darken_memo: std::cell::Cell<Option<((i64, u32, u32), i32)>>,
    /// The last water plane `water_plane` found, kept while none is in reach.
    water_plane_memo: std::cell::Cell<Option<f32>>,
    /// The camera's block and the second `water_plane` last scanned at.
    water_plane_scan: std::cell::Cell<Option<([i32; 3], i64)>>,
    /// The mobs' box reach while the boats and minecarts tick, with the mob count it was
    /// worked out for (`vehicle_mob_reach`).
    vehicle_mob_reach: std::cell::Cell<Option<(usize, horizon_client::mobs::BoxReach)>>,
    persist: persist::EntityMemory,
    /// Missing profession IDs already reported for this world session.
    missing_professions: HashSet<String>,
    entity_attachments: HashMap<[i32; 4], horizon_mod_runtime::attachments::Attachments>,
    /// Typed per-level membership follows dimensions, not the temporary world swap.
    attachment_indexes: attachment_index::Indexes,
    /// Connected to a server: the world comes from the network (game/network/remote/mod.rs).
    remote: Option<remote::Remote>,
    /// This world is open to the network (game/network/lan/mod.rs).
    lan: Option<lan::LanHost>,
    /// Single player's server thread, which `remote` is connected to.
    integrated: Option<integrated::Integrated>,
    /// The shapes of the open world's levels, from its dimension types.
    level_shapes: horizon_client::chunk::LevelShapes,
    /// This game is that server thread's (game/world/integrated.rs).
    serving: Option<integrated::Serving>,
    /// The open world is shared behind an invite code (game/network/friends.rs).
    friends: Option<friends::Sharing>,
    /// The Social Interactions screen and the players whose chat is hidden (game/network/social.rs).
    social: social::Social,
    world_options: Option<lan_screen::WorldOptions>,
    /// Chat Settings opened from RestrictionsScreen: Done returns to World Options.
    options_to_world_options: bool,
    /// HDR's settings opened with their key from the game: Done returns to the game.
    options_to_game: bool,
    command_block_screen: Option<command_blocks::CommandBlockScreen>,
    block_editor: Option<block_editor::BlockEditScreen>,
    /// `StructureTemplateManager.structureRepository`: the templates in memory
    /// by id, a miss included (game/administration/structure_blocks.rs).
    structure_templates: HashMap<String, Option<structure_blocks::TemplateEntry>>,
    /// The game tests read so far, by id (the test instance registry).
    test_defs: std::cell::RefCell<HashMap<String, Option<test_instances::TestDef>>>,
    /// `GameTestBlockHighlightRenderer.markers`: block, text, removal time.
    test_highlights: HashMap<(i32, i32, i32), (String, std::time::Instant)>,
    /// `GameTestTicker.SINGLETON`: the game tests running on this server.
    game_tests: game_tests::GameTests,
    /// KeyboardHandler's debug key state (game/interface/debug_keys.rs), the physical
    /// keys held, and the debug renderers F3+B and F3+G switch on.
    debug_keys: horizon_client::debug_keys::DebugKeys,
    /// The F3 overlay's charts (F3+2, F3+3).
    debug_charts: horizon_client::debug_chart::DebugCharts,
    /// `Minecraft.debugEntries`: what the overlay and the debug renderers
    /// show (`debug-profile.json`, F3+F6).
    debug_entries: horizon_client::debug_entries::DebugEntries,
    /// `DebugOptionsScreen` while it is open (game/interface/debug_options.rs).
    debug_options: Option<debug_options::DebugOptionsUi>,
    /// `DebugEntryMemory`'s readings (game/interface/debug_lines.rs).
    memory_meter: std::cell::RefCell<horizon_client::debug_overlay::MemoryMeter>,
    /// The F3 overlay's vertices between rebuilds (`draw_debug_overlay_under`).
    debug_paint: RefCell<horizon_client::debug_overlay::PaintCache>,
    /// The F3 overlay's last reading of the refresh rate, and when it was taken.
    debug_refresh: std::cell::Cell<Option<(Instant, Option<u32>)>>,
    /// `Minecraft.gpuUtilization`: the last timed frame's GPU time over its
    /// CPU time, in percent.
    gpu_utilization: f64,
    /// The mobs `mob_geometry` drew last frame (`lastEntityRenderStateCount`).
    rendered_mobs: std::cell::Cell<usize>,
    /// Whether a native mod requested background blur for this frame (`apply_screen_blur`).
    screen_blur_requested: std::cell::Cell<bool>,
    /// `ServerChunkCache.lastSpawnState` of each level: its spawnable chunks
    /// and each `MobCategory`'s count.
    spawn_counts: HashMap<Dim, (i32, [i32; 8])>,
    /// PauseScreen.showPauseMenu (false after F3+Escape).
    pause_menu: bool,
    pause_focus: Option<u8>,
    death_focus: Option<u8>,
    /// The screen ([`Game::key_screen`]) the menu focus belongs to. Another
    /// screen in front drops it, as `Gui.setScreen` drops the focus of the
    /// screen it leaves (`Screen.clearFocus`).
    focus_screen: u64,
    /// MultiPlayerGameMode.previousLocalPlayerMode (F3+N goes back to it).
    previous_mode: Option<horizon_client::command::args::GameMode>,
    gamemode_switcher: Option<horizon_client::game_mode_switcher::GameModeSwitcher>,
    /// The system clipboard (`KeyboardHandler.clipboardManager`).
    clipboard: horizon_client::clipboard::SystemClipboard,
    /// The game is handling a key, mouse or typed-text event, from the window or the
    /// control socket: the only time a mod reaches the clipboard (`with_user_input`).
    user_input: std::cell::Cell<bool>,
    /// Select All in the append-only text fields.
    text_selection: horizon_client::clipboard::AppendSelection,
    /// The world was saved for the application ending (`save_for_exit`).
    saved_for_exit: bool,
    pending_save: Option<saving::ClosingSave>,
    /// The last world frame reached the swap chain. A window the compositor
    /// stops showing gets no frames, and the event loop must not spin on it.
    frame_drawn: bool,
    /// The world list's icon in progress (`GameRenderer.tryTakeScreenshotIfNeeded`):
    /// `None` without a local world, or once the save has its icon.png.
    world_icon: Option<WorldIcon>,
    /// The block a guest is acting on (game/network/lan_actions/interaction.rs).
    forced_hit: Option<Hit>,
    /// `Minecraft.hitResult` while the clicks of a tick act (game/network/remote/mod.rs
    /// `remote_handle_input`): picked once, so a creative break does not
    /// move the use that follows it past the broken block.
    tick_hit: Option<Option<Hit>>,
    /// The mob of that `hitResult`, picked with the item held at the pick
    /// (`LocalPlayer.raycastHitResult`: a spear's `attack_range`), before the
    /// tick's number keys change it.
    tick_mob: Option<Option<(usize, f64, [f64; 3])>>,
    /// Number keys since the last tick (`keyHotbarSlots[i].consumeClick`):
    /// `Minecraft.handleKeybinds` selects them after the tick's pick.
    hotbar_presses: Vec<usize>,
    /// `Player.entityInteractionRange` and `blockInteractionRange`, refreshed
    /// each tick from the attribute (a server's reach modifier, creative's
    /// +2): the pick helpers' `&self` reach.
    entity_reach: f64,
    block_reach: f64,
    /// The part of a right click a guest's packet asks for (game/network/lan_actions/interaction.rs).
    guest_use: Option<GuestUse>,
    /// The last `use_hand` that ended the click did so with `FAIL`, which
    /// does not consume the action (handleUseItemOn's ANY_BLOCK_USE).
    use_failed: bool,
    /// Level events of this tick for the guests, with their excluded source.
    lan_events: Vec<QueuedLevelEvent>,
    /// Jukebox level events 1010 (song name) and 1011 (`None`) for the guests;
    /// the song's network id is only known once the hub is in hand.
    lan_songs: Vec<([i32; 3], Option<String>)>,
    lan_cushion_breaks: Vec<(horizon_world::registry::BlockStateId, [f64; 3])>,
    /// Block events (position, b0, b1, state) and entity events (id, event)
    /// for the guests.
    lan_block_events: Vec<([i32; 3], i32, i32, u32)>,
    lan_entity_events: Vec<(i32, i8)>,
    /// Successful crits (attacking guest, hit entity), queued while the hub is taken.
    lan_critical_hits: Vec<(Option<u32>, i32, bool)>,
    /// Spear stabs that landed (the stabbing guest), queued while the hub is
    /// taken: `broadcastEntityEvent(player, 2)`.
    lan_kinetic_hits: Vec<Option<u32>>,
    /// `LivingEntity.recentKineticEnemies`: when each player (a guest, or the
    /// window's own) last stabbed each entity this use.
    stabbed: HashMap<(Option<u32>, u32), i64>,
    /// The movement a guest's client reported this tick
    /// (`ServerPlayer.getKnownMovement`), for their turn.
    known_movement: Vec3,
    /// `LivingEntity.lastKineticHitFeedbackTime` of the entities that felt a
    /// spear's hit (`spear::OWN` is this window's player), in game time.
    kinetic_feedback: HashMap<i32, i64>,
    /// Global level events for every guest (`ServerLevel.globalLevelEvent`).
    lan_global_events: Vec<(i32, [i32; 3], i32)>,
    /// `ContainerOpenersCounter`: how many players (the host and the guests)
    /// have each chest open.
    lid_openers: HashMap<(i32, i32, i32), i32>,
    lan_acting: Option<u32>,
    /// A mod asked for the command trees again (`native_refresh_commands`):
    /// the next server tick asks every mod for its commands and resends them.
    native_commands_refresh: bool,
    /// The abilities of the server player whose turn it is (`as_guest`): their record's
    /// copy, which the turn reads and writes and `leave_guest` stores. None outside a turn.
    acting_abilities: Option<horizon_client::lan::abilities::PlayerAbilities>,
    /// Other players in the current guest turn, including borrowed ancestor bodies.
    lan_building_boxes: building_players::BuildingPlayers,
    /// A guest's `/save-all`: saved once their turn ends (`run_guest_command`),
    /// when their own state is back in their record.
    save_after_guest_turn: bool,
    /// Guests whose records are displaced by nested player swaps, outer first.
    lan_guest_swaps: Vec<u32>,
    /// The active player's map; a guest turn moves it from its owning record.
    player_attachments: horizon_mod_runtime::attachments::ResidentAttachments,
    /// The name of the guest `lan_acting` acts for (`Player.getName`).
    lan_acting_name: Option<String>,
    /// Their UUID (`Player.getUUID`).
    lan_acting_uuid: Option<u128>,
    /// Where the mods refused to prime TNT: the sound and game event the level
    /// queued right after the prime are dropped (`prime_tnt_event`).
    tnt_refused: Option<BlockPos>,
    /// Their permission level: an op's above 0 (`PlayerList.isOp`).
    lan_acting_permission: Option<u8>,
    /// Their entity id while their player state is in place.
    lan_acting_entity: Option<u32>,
    /// Whether the player currently swapped into the game is connectionless.
    lan_acting_fake: bool,
    /// A fake player's `perform_action` in progress, and what it refused.
    fake_action: std::sync::Arc<native_server::FakeAction>,
    /// Command feedback to forward to a guest.
    chat_capture: Option<Vec<horizon_client::text::Text>>,
    /// Packets for the guest acting now, sent once its action returns.
    guest_packets: Vec<horizon_protocol::packets::play::Clientbound>,
    /// The acting guest's cooldowns, as (item, ticks): their packet needs the
    /// hub's item registry, which the guest's turn does not hold.
    guest_cooldowns: Vec<(Arc<str>, i32)>,
    /// The acting guest's own entity events (`ServerPlayer.connection.send`),
    /// sent with their entity id, which the guest's turn does not hold.
    guest_entity_events: Vec<i8>,
    /// The acting guest's equipment breaks (`onEquippedItemBroken`'s entity
    /// events), broadcast with their entity id once their turn ends.
    guest_equipment_breaks: Vec<i8>,
    /// `Player.playSound` of the acting guest (event, position, volume,
    /// pitch): `level.playSound(player, ...)` leaves the guest out, whose
    /// own client plays it.
    guest_player_sounds: Vec<(String, [f64; 3], f32, f32)>,
    /// Sounds bound to an entity this tick, for the players (`game/interface/level_sounds.rs`).
    bound_sounds: Vec<level_sounds::BoundSound>,
    /// `broadcastToAdmins` lines a guest's command queued, sent once it
    /// returns: the line, whether operators get it and whether the console
    /// logs it.
    admin_feedback: Vec<(horizon_client::text::Text, bool, bool)>,
    /// The server console is running a command: whether its feedback goes to
    /// the operators (`DedicatedServer.shouldInformAdmins`,
    /// `broadcast-console-to-ops`).
    console_broadcast: Option<bool>,
    /// The command the console is running came over RCON: its source is named `Rcon`
    /// (`RconConsoleSource.createCommandSourceStack`), not `Server`.
    rcon_source: bool,
    /// A dedicated server's operator, whitelist and ban lists, for `/op`,
    /// `/ban` and friends (`DedicatedPlayerList`).
    server_lists: Option<std::sync::Arc<std::sync::Mutex<dedicated::Lists>>>,
    /// State of `/spectate`, `/posteffect`, `/debug`, `/waypoint` and friends.
    cmds: commands::extra::CommandExtras,
    /// The retained server reload listeners of the last successful reload.
    reload_resources: mod_api::RetainedListeners,
    ui: chat::UiState,
    combat: combat::CombatTracker,
    /// Each guest's own `CombatTracker`, out of `combat` between turns.
    guest_combat: std::collections::HashMap<u32, combat::CombatTracker>,
    mount: mounts::MountState,
    stats: stats::Stats,
    score: scoreboard::ScoreState,
    fns: functions::FnState,
    bosses: bossbars::BossState,
    /// `Hud.bossOverlay`: the bars this window is showing.
    boss_overlay: horizon_client::boss_overlay::BossOverlay,
    exec: commands::execute::ExecState,
    rand_sequences: horizon_client::random_sequences::RandomSequences,
    /// `BrushableBlockEntity`'s brush counts of the blocks being brushed.
    brushing: brush::Brushing,
    border: horizon_client::worldborder::WorldBorder,
    /// `WorldBorderRenderer` and its render state.
    border_renderer: horizon_client::border_render::BorderRenderer,
    /// The camera's 16-block section when the wall was last invalidated.
    border_section: Option<[i32; 3]>,
    border_state: horizon_client::border_render::BorderRenderState,
    /// `Hud.tabList`: the player list's header, footer and heart states.
    tab_list: horizon_client::tab_list::TabList,
    subtitles: subtitle_hud::SubtitleState,
    adv: advancements::AdvState,
    restore_player_state: bool,
    stats_ui: stats_screen::StatsUi,
    adv_ui: advancements_screen::AdvUi,
    /// The Server Links screens' state (game/network/server_links.rs).
    links_ui: server_links::LinksUi,
    camera_type: horizon_client::camera::CameraType,
    requested_camera_type: std::cell::Cell<Option<horizon_client::camera::CameraType>>,
    fangs: fangs::FangState,
    /// The local player as a model (third person, the inventory).
    player_model: Option<Mob>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Dim {
    Overworld,
    Nether,
    End,
}
impl Dim {
    /// The level as block behaviour and the mods name it.
    fn level(self) -> LevelDimension {
        match self {
            Self::Overworld => LevelDimension::Overworld,
            Self::Nether => LevelDimension::Nether,
            Self::End => LevelDimension::End,
        }
    }
    fn id(self) -> &'static str {
        match self {
            Self::Overworld => "minecraft:overworld",
            Self::Nether => "minecraft:the_nether",
            Self::End => "minecraft:the_end",
        }
    }
    fn from_id(id: &str) -> Self {
        match id {
            "minecraft:the_nether" => Self::Nether,
            "minecraft:the_end" => Self::End,
            _ => Self::Overworld,
        }
    }
    /// The dimension as mods' level handles name it.
    fn key(self) -> mod_api::DimensionKey {
        match self {
            Self::Overworld => mod_api::DimensionKey::OVERWORLD,
            Self::Nether => mod_api::DimensionKey::NETHER,
            Self::End => mod_api::DimensionKey::END,
        }
    }
    /// The level's shape in a world of vanilla's dimension types.
    fn shape(self) -> horizon_client::save::LevelShape {
        use horizon_client::save::LevelShape;
        match self {
            Self::Overworld => LevelShape::OVERWORLD,
            Self::Nether => LevelShape::NETHER,
            Self::End => LevelShape::END,
        }
    }
}
/// `Inventory.add` on a player's 36 slots (hotbar first) and off hand,
/// `max` the stack's own maximum: onto equal stacks first
/// (`getSlotWithRemainingSpace`: the selected slot, the off hand, then the
/// hotbar and the main inventory), then into empty slots of the inventory
/// (`getFreeSlot`, never the off hand). What does not fit stays in `stack`;
/// true when some of it is left.
fn inventory_add(
    inventory: &mut [Option<ItemStack>],
    offhand: &mut Option<ItemStack>,
    selected: usize,
    max: u32,
    stack: &mut ItemStack,
) -> bool {
    let order = std::iter::once(selected)
        .chain(std::iter::once(shield::OFFHAND_SLOT))
        .chain((0..36).filter(|&i| i != selected));
    for i in order {
        let slot = if i == shield::OFFHAND_SLOT {
            &mut *offhand
        } else {
            &mut inventory[i]
        };
        if let Some(slot) = slot
            && slot.same_item_same_components(stack)
            && slot.count < max
        {
            let moved = (max - slot.count).min(stack.count);
            slot.count += moved;
            stack.count -= moved;
            if stack.count == 0 {
                return false;
            }
        }
    }
    for slot in inventory.iter_mut().take(36) {
        if slot.is_none() {
            let moved = stack.count.min(max);
            *slot = Some(ItemStack {
                count: moved,
                ..stack.clone()
            });
            stack.count -= moved;
            if stack.count == 0 {
                return false;
            }
        }
    }
    true
}

fn facing_offset(facing: &str) -> [i32; 3] {
    match facing {
        "north" => [0, 0, -1],
        "south" => [0, 0, 1],
        "west" => [-1, 0, 0],
        "east" => [1, 0, 0],
        "up" => [0, 1, 0],
        _ => [0, -1, 0],
    }
}
/// The point a `ServerboundInteractPacket` carries: `EntityHitResult.getLocation()
/// - entity.position()`, measured from the entity's feet, as
/// `MultiPlayerGameMode.interact` computes it.
fn interact_location(point: Vec3, feet: Vec3) -> [f64; 3] {
    [point.x - feet.x, point.y - feet.y, point.z - feet.z]
}
/// How far along `dir` (within `reach`) a ray from `eye` enters `b`.
fn ray_box(eye: [f64; 3], dir: [f64; 3], reach: f64, b: Aabb) -> Option<f64> {
    let (min, max) = (b.min_position(), b.max_position());
    let (lo, hi) = ([min.x, min.y, min.z], [max.x, max.y, max.z]);
    let (mut t0, mut t1) = (0.0f64, reach);
    for a in 0..3 {
        if dir[a].abs() < 1e-9 {
            if eye[a] < lo[a] || eye[a] > hi[a] {
                return None;
            }
            continue;
        }
        let (mut ta, mut tb) = ((lo[a] - eye[a]) / dir[a], (hi[a] - eye[a]) / dir[a]);
        if ta > tb {
            std::mem::swap(&mut ta, &mut tb);
        }
        t0 = t0.max(ta);
        t1 = t1.min(tb);
    }
    (t0 <= t1).then_some(t0)
}
/// `Entity.calculateViewVector`, which `GameRenderer.pick` aims with: float radians through `Mth.sin` and
/// `Mth.cos`, vanilla's 65536-entry table. A placement's cursor and an interaction's point come from this ray, and
/// the exact trigonometry below moved them by up to ~1e-4 of a block.
fn pick_view_vector(yaw: f32, pitch: f32) -> [f64; 3] {
    let v = horizon_world::view_direction::view_vector(pitch, yaw);
    [v.x, v.y, v.z]
}
fn view_vector(yaw: f32, pitch: f32) -> [f64; 3] {
    let (y, p) = (f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    [-y.sin() * p.cos(), -p.sin(), y.cos() * p.cos()]
}
/// `LivingEntity.canGlideUsing`: for each worn piece (head, chest, legs,
/// feet), whether it is a glider (its own component, or one the server
/// patched onto the stack) in the slot its `equippable` names, and will not
/// break with its next point of wear (`ItemStack.nextDamageWillBreak`).
fn gliding_pieces(items: &Items, armor: &[Option<ItemStack>; 4]) -> [bool; 4] {
    const SLOTS: [&str; 4] = ["head", "chest", "legs", "feet"];
    // `canGlideUsing`: the stack's own `glider` and `equippable` (its
    // components, a server's patch), else its item's.
    std::array::from_fn(|i| {
        armor[i]
            .as_ref()
            .is_some_and(|s| horizon_client::item_sync::can_glide_using(s, SLOTS[i], items))
    })
}
/// `Level.canHaveWeather`: sky light, no ceiling, and not the End (the one
/// level the rule names by id).
fn level_can_have_weather(sky_light: bool, has_ceiling: bool, is_end: bool) -> bool {
    sky_light && !has_ceiling && !is_end
}
/// `Camera.setRotation` (`rotationYXZ(π - yaw, -pitch, 0)`) for the player's
/// view, turned round in third-person front view (`Camera.setup`).
fn camera_rotation(
    camera_type: horizon_client::camera::CameraType,
    yaw: f32,
    pitch: f32,
) -> glam::Quat {
    let (view_yaw, view_pitch) =
        if camera_type == horizon_client::camera::CameraType::ThirdPersonFront {
            (yaw + 180.0, -pitch)
        } else {
            (yaw, pitch)
        };
    glam::Quat::from_euler(
        glam::EulerRot::YXZ,
        std::f32::consts::PI - view_yaw.to_radians(),
        -view_pitch.to_radians(),
        0.0,
    )
}
impl Game<'_> {
    /// The ticks shown by this frame; still mode follows world time.
    pub(super) fn visual_ticks(&self) -> u64 {
        if self.clock.still() {
            self.world.game_time.max(0) as u64
        } else {
            self.ticks
        }
    }

    /// Frame time in milliseconds, fixed to world time in still mode.
    pub(super) fn visual_millis(&self) -> i64 {
        if self.clock.still() {
            self.visual_ticks() as i64 * 50
        } else {
            self.started.elapsed().as_millis() as i64
        }
    }

    /// `Level.canHaveWeather`, from the level's dimension type (sky light, no
    /// ceiling) and the End, which vanilla names by id. Every rain, thunder
    /// and lightning rule asks this, never the dimension's id.
    fn can_have_weather(&self) -> bool {
        level_can_have_weather(
            self.world.shape.sky_light,
            self.world.shape.has_ceiling,
            self.dim == Dim::End,
        )
    }
    /// The rain and thunder levels (`Level.getRainLevel`, `getThunderLevel`):
    /// the world's, where the level can have weather, else none.
    fn weather_levels(&self, partial: f32) -> (f32, f32) {
        if self.can_have_weather() {
            self.world.weather.levels(partial)
        } else {
            (0.0, 0.0)
        }
    }
    fn eye_position(&self) -> Option<[f64; 3]> {
        let p = self.player.as_ref()?;
        Some([
            p.position.x,
            p.position.y + f64::from(p.dimensions.eye_height),
            p.position.z,
        ])
    }
    /// Where targeting starts and points: the eyes along the view. Every
    /// crosshair target (blocks, fluids, entities, mod raycasts) is found
    /// along this ray, so a tracked controller can stand in for it.
    fn aim(&self) -> Option<([f64; 3], [f64; 3])> {
        // Minecraft.pick: from the camera entity's eyes along its view
        // (LivingEntity.getViewYRot: the head's yaw).
        if let Some(m) = self.camera_mob() {
            return Some((
                [m.position.x, m.eye_y(), m.position.z],
                pick_view_vector(m.head_yaw, m.head_pitch),
            ));
        }
        Some((self.eye_position()?, pick_view_vector(self.yaw, self.pitch)))
    }
    /// `Player.blockInteractionRange`: the block pick's and mining's reach.
    fn reach(&self) -> f64 {
        self.block_reach
    }
    /// Outline shapes that depend on more than the block state, as one box
    /// in block space: `LightBlock` and `ScaffoldingBlock.getShape` are a full
    /// block while the main hand holds the block's own item
    /// (`CollisionContext.isHoldingItem`), and `ShulkerBoxBlock.getShape`
    /// grows with the lid (`ShulkerBoxBlockEntity.getBoundingBox`).
    fn context_outline(
        &self,
        pos: [i32; 3],
        state: BlockStateId,
        player_context: bool,
        anims: &horizon_client::block_entity_anim::BlockEntityAnims,
    ) -> Option<[f64; 6]> {
        let name = &self.res.facts().get(state).name;
        if matches!(&**name, "minecraft:light" | "minecraft:scaffolding") {
            return (player_context && self.held().is_some_and(|s| *s.item == **name))
                .then_some([0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        }
        if !name.ends_with("shulker_box") {
            return None;
        }
        let progress = anims.shulkers.get(&pos).map_or(0.0, |s| s.progress);
        let facing = self
            .pack
            .assets
            .registry
            .state(state)
            .and_then(|s| s.properties.get("facing").cloned());
        let d = facing_offset(facing.as_deref().unwrap_or("up"));
        let mut b = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        for a in 0..3 {
            match d[a] {
                1 => b[a + 3] += 0.5 * f64::from(progress),
                -1 => b[a] -= 0.5 * f64::from(progress),
                _ => {}
            }
        }
        Some(b)
    }
    /// A block's outline boxes in world space (`getShape` plus the random offset).
    fn outline_boxes(&self, pos: [i32; 3]) -> Vec<[f64; 6]> {
        self.outline_boxes_in_context(&self.world, &self.be_anims, pos, true)
    }
    fn outline_boxes_in_context(
        &self,
        world: &World,
        anims: &horizon_client::block_entity_anim::BlockEntityAnims,
        pos: [i32; 3],
        player_context: bool,
    ) -> Vec<[f64; 6]> {
        self.outline_boxes_iter(world, anims, pos, player_context)
            .collect()
    }
    /// The same outline traversal without allocating a box list.
    fn outline_boxes_iter(
        &self,
        world: &World,
        anims: &horizon_client::block_entity_anim::BlockEntityAnims,
        pos: [i32; 3],
        player_context: bool,
    ) -> impl Iterator<Item = [f64; 6]> + '_ {
        let state = world.block(pos[0], pos[1], pos[2]);
        let context = self.context_outline(pos, state, player_context, anims);
        let f = self.res.facts().get(state);
        let o = perspective::outline_offset(
            &self.pack.context.tables,
            state,
            pos[0],
            pos[2],
            f.outline_offset,
        );
        let base = [
            f64::from(pos[0]) + o[0],
            f64::from(pos[1]) + o[1],
            f64::from(pos[2]) + o[2],
        ];
        let boxes = if context.is_some() {
            &[][..]
        } else {
            &f.outline[..]
        };
        context
            .map(|b| std::array::from_fn(|i| f64::from(pos[i % 3]) + b[i]))
            .into_iter()
            .chain(
                boxes
                    .iter()
                    .map(move |b| std::array::from_fn(|i| base[i % 3] + b[i])),
            )
    }
    /// Voxel traversal against non-air, non-fluid blocks, clipped against
    /// each block's outline shape (`BlockGetter.clip` with `ClipContext.Block.OUTLINE`).
    fn pick(&self) -> Option<Hit> {
        // A guest of a hosted world acts on the block their client targeted.
        if let Some(hit) = self.forced_hit {
            return Some(hit);
        }
        if let Some(hit) = self.tick_hit {
            return hit;
        }
        let (origin, dir) = self.aim()?;
        // GameRenderer.pick: the block ray runs out to the larger of the two
        // interaction ranges; filterHitResult then drops a hit past the
        // block's.
        self.clip_ray(origin, dir, self.block_reach.max(self.entity_reach))
            .filter(|hit| {
                let distance: [f64; 3] = std::array::from_fn(|i| hit.point[i] - origin[i]);
                distance.iter().map(|v| v * v).sum::<f64>() <= self.block_reach.powi(2)
            })
    }
    /// The same traversal without `filterHitResult`: the block hit that
    /// bounds the entity ray in `GameRenderer.pick`, out to the larger
    /// interaction range — a wall between the two still hides the entity.
    fn pick_unfiltered(&self) -> Option<Hit> {
        if let Some(hit) = self.forced_hit {
            return Some(hit);
        }
        let (origin, dir) = self.aim()?;
        self.clip_ray(origin, dir, self.block_reach.max(self.entity_reach))
    }
    /// The first block outline hit within `reach` along the unit `dir`.
    fn clip_ray(&self, origin: [f64; 3], dir: [f64; 3], reach: f64) -> Option<Hit> {
        let mut pos = origin.map(|v| v.floor() as i32);
        let step = dir.map(|d| if d > 0.0 { 1 } else { -1 });
        let delta = dir.map(|d| {
            if d == 0.0 {
                f64::INFINITY
            } else {
                (1.0 / d).abs()
            }
        });
        let mut t_max = [0.0; 3];
        for i in 0..3 {
            let boundary = if dir[i] > 0.0 {
                f64::from(pos[i]) + 1.0
            } else {
                f64::from(pos[i])
            };
            t_max[i] = if dir[i] == 0.0 {
                f64::INFINITY
            } else {
                (boundary - origin[i]) / dir[i]
            };
        }
        let mut face = [0; 3];
        let mut t = 0.0;
        while t <= reach {
            let state = self.world.block(pos[0], pos[1], pos[2]);
            let f = self.res.facts().get(state);
            if !self.res.facts().is_air(state)
                && (f.fluid == Fluid::None
                    || f.waterlogged
                    || &*f.name == "minecraft:kelp"
                    || f.render_shape == horizon_client::blocks::RenderShape::Model)
                && !matches!(
                    &*f.name,
                    "minecraft:water" | "minecraft:lava" | "minecraft:bubble_column"
                )
            {
                // The nearest entry into one of the outline's boxes (slab method).
                let mut best: Option<(f64, [i32; 3])> = None;
                for b in self.outline_boxes(pos) {
                    let (mut t0, mut t1, mut axis_in, mut sign) = (0.0f64, f64::INFINITY, None, 0);
                    let mut miss = false;
                    for a in 0..3 {
                        if dir[a] == 0.0 {
                            miss |= origin[a] < b[a] || origin[a] > b[a + 3];
                            continue;
                        }
                        let (lo, hi) =
                            ((b[a] - origin[a]) / dir[a], (b[a + 3] - origin[a]) / dir[a]);
                        let (near, far) = if lo < hi { (lo, hi) } else { (hi, lo) };
                        if near > t0 {
                            t0 = near;
                            axis_in = Some(a);
                            sign = if dir[a] > 0.0 { -1 } else { 1 };
                        }
                        t1 = t1.min(far);
                    }
                    if miss || t0 > t1 || t0 > reach {
                        continue;
                    }
                    let mut normal = face;
                    if let Some(a) = axis_in {
                        normal = [0; 3];
                        normal[a] = sign;
                    }
                    if best.is_none_or(|(bt, _)| t0 < bt) {
                        best = Some((t0, normal));
                    }
                }
                if let Some((t, mut face)) = best {
                    // `BlockGetter.clipWithInteractionOverride`: the hit stays at the
                    // outline's point, with the face of the interaction shape when
                    // the ray enters that first.
                    let name = &*f.name;
                    let facing = self
                        .pack
                        .assets
                        .registry
                        .state(state)
                        .and_then(|s| s.properties.get("facing"))
                        .map(String::as_str);
                    let inside = interaction_shapes::boxes(name, facing);
                    if !inside.is_empty() {
                        let at = pos.map(f64::from);
                        let world: Vec<[f64; 6]> = inside
                            .iter()
                            .map(|b| std::array::from_fn(|i| at[i % 3] + b[i]))
                            .collect();
                        if let Some((ti, normal)) =
                            interaction_shapes::entry(origin, dir, &world, reach, face)
                            && ti < t
                        {
                            face = normal;
                        }
                    }
                    return Some(Hit {
                        pos,
                        face,
                        point: [
                            origin[0] + dir[0] * t,
                            origin[1] + dir[1] * t,
                            origin[2] + dir[2] * t,
                        ],
                        // No face entered: the eyes are in the shape.
                        inside: face == [0; 3],
                    });
                }
            }
            let axis = if t_max[0] < t_max[1] {
                if t_max[0] < t_max[2] { 0 } else { 2 }
            } else if t_max[1] < t_max[2] {
                1
            } else {
                2
            };
            t = t_max[axis];
            t_max[axis] += delta[axis];
            pos[axis] += step[axis];
            face = [0; 3];
            face[axis] = -step[axis];
        }
        None
    }
    /// The stack in the hand that acts (`getItemInHand(hand)`).
    fn held(&self) -> Option<&ItemStack> {
        self.stack_in(self.held_slot())
    }
    fn held_mut(&mut self) -> &mut Option<ItemStack> {
        self.stack_in_mut(self.held_slot())
    }
    /// The slot of the hand that acts: the selected one, or `OFFHAND_SLOT`.
    /// `LivingEntity.canGlide`: some worn piece `gliding_pieces` lets glide,
    /// or a stack in a hand that glides there (`canGlideUsing` walks every
    /// `EquipmentSlot`).
    fn glider_worn(&self) -> bool {
        let items = self.res.items();
        let hand = |stack: Option<&ItemStack>, slot: &str| {
            stack.is_some_and(|s| horizon_client::item_sync::can_glide_using(s, slot, items))
        };
        gliding_pieces(items, &self.armor).contains(&true)
            || hand(self.inventory[self.selected].as_ref(), "mainhand")
            || hand(self.offhand.as_ref(), "offhand")
    }
    /// `LivingEntity.updateFallFlying`: the glide's wear. `Util.getRandom`
    /// picks one of the gliders that will not break (from the entity's own
    /// random), and
    /// `ItemStack.hurtAndBreak` takes `points` of its durability (Unbreaking
    /// may save it). Callers skip creative.
    pub(super) fn wear_glider(&mut self, points: u32) {
        let gliding = gliding_pieces(self.res.items(), &self.armor);
        let pieces: Vec<usize> = (0..4).filter(|&i| gliding[i]).collect();
        if pieces.is_empty() {
            return;
        }
        // `Util.getRandom(slotsWithGliders, this.random)`: the entity's own
        // random, which never touches the level's stream.
        let pick = dispenser::entity_random().next_int_bound(pieces.len() as i32);
        let pick = pieces[pick.unwrap_or(0) as usize];
        if let Some(stack) = self.armor[pick].clone() {
            let loss = self.wear(&stack, points);
            if let Some(piece) = self.armor[pick].as_mut() {
                piece.damage += loss;
            }
        }
    }
    /// `LivingEntity.canFreeze`'s armor half: any worn piece in
    /// `freeze_immune_wearables`.
    pub(crate) fn wears_freeze_immune(&self) -> bool {
        let immune = self
            .res
            .items()
            .item_tags
            .get("minecraft:freeze_immune_wearables");
        self.armor
            .iter()
            .flatten()
            .any(|s| immune.is_some_and(|t| t.contains(&*s.item)))
    }
    /// `LocalPlayer.moveTowardsClosestSpace` at the body's four corners
    /// (aiStep, unless a spectator or a passenger): a corner inside a
    /// suffocating block sets a 0.1 speed towards the nearest side that is
    /// free, on that side's axis; a later corner overwrites an earlier one.
    /// Suffocating is a full collision cube that is also a full render cube,
    /// as `in_wall` has it.
    fn closest_space_nudge(&self) -> [Option<f64>; 2] {
        match self.player.as_ref() {
            Some(p) if !p.spectator && self.riding.is_none() => {
                closest_space_nudge(p, &self.world, self.res.facts())
            }
            _ => [None; 2],
        }
    }
    /// What the window decides for its player's body every tick: the look
    /// (Player.travel steers sprint-swimming by the pitch), Auto Jump (LocalPlayer.sendPosition), the slowdown of an
    /// item in use (LocalPlayer.modifyInput), the helmet's Respiration
    /// (decreaseAirSupply), the wings, leather boots on powder snow
    /// (PowderSnowBlock.canEntityWalkOnPowderSnow), freeze-immune wear
    /// (LivingEntity.canFreeze), the dimension's FAST_LAVA, the sleeping
    /// pose and blindness (Player.isMobilityRestricted, which stops a sprint).
    pub(crate) fn sync_player_body(&mut self) {
        let glider = self.glider_worn();
        let sleeping = self.sleeping.is_some();
        let item_in_use = self
            .using
            .and_then(|(slot, _)| {
                if slot == shield::OFFHAND_SLOT {
                    self.offhand.as_ref()
                } else {
                    self.inventory.get(slot)?.as_ref()
                }
            })
            .and_then(|s| self.items().get(&s.item));
        let item_use_speed = item_in_use.map(|i| i.use_speed);
        let item_use_slows_sprint = item_in_use.is_some_and(|i| !i.can_sprint);
        let respiration = self.armor[0]
            .as_ref()
            .map_or(0, |s| s.enchantment_level("minecraft:respiration") as i32);
        let leather_boots = self.armor[3]
            .as_ref()
            .is_some_and(|s| &*s.item == "minecraft:leather_boots");
        let freeze_immune = self.wears_freeze_immune();
        let burn_immune = self.wears_burn_from_stepping_immunity();
        let nudge = self.closest_space_nudge();
        let blind = self.potions.effects.get("blindness").is_some();
        let Some(player) = self.player.as_mut() else {
            return;
        };
        player.nudge = nudge;
        player.mobility_restricted = blind;
        player.yaw = self.yaw;
        player.pitch = self.pitch;
        player.auto_jump_enabled = self.settings.auto_jump;
        player.item_use_speed = item_use_speed;
        player.item_use_slows_sprint = item_use_slows_sprint;
        player.respiration = respiration;
        player.glider = glider;
        // On someone else's server the glide flag is the server's
        // (`DATA_SHARED_FLAGS_ID`, applied by the SetEntityData handler); the
        // game's own integrated server has no such round trip, so the client
        // keeps predicting the clear a tick after the landing glide.
        player.server_owns_glide = self.integrated.is_none();
        let _ = player.set_sleeping(sleeping);
        player.leather_boots = leather_boots;
        player.freeze_immune = freeze_immune;
        player.burn_from_stepping_immune = burn_immune;
        player.fluids.fast_lava = self.dim == Dim::Nether;
    }
    fn held_slot(&self) -> usize {
        match self.hand {
            Hand::Main => self.selected,
            Hand::Off => shield::OFFHAND_SLOT,
        }
    }
    /// `LivingEntity.getUsedItemHand`: the hand of the item in use.
    fn used_hand(&self) -> Option<Hand> {
        item_use::hand_of(self.using)
    }
    /// Run `f` with `hand` acting (`held`, `consume_held`, `swing_arm`).
    fn with_hand<T>(&mut self, hand: Hand, f: impl FnOnce(&mut Self) -> T) -> T {
        let prior = std::mem::replace(&mut self.hand, hand);
        let out = f(self);
        self.hand = prior;
        out
    }
    /// Inventory.add: returns the stack that did not fit.
    fn add_stack_to_inventory(&mut self, mut stack: ItemStack) -> Option<ItemStack> {
        let tutorial_item = matches!(
            self.tutorial.step,
            tutorial::Step::FindTree | tutorial::Step::PunchTree | tutorial::Step::CraftPlanks
        )
        .then(|| stack.clone());
        let max = self
            .res
            .items()
            .get(&stack.item)
            .map_or(64, |i| i.max_stack);
        let before = stack.count;
        let left = inventory_add(
            &mut self.inventory,
            &mut self.offhand,
            self.selected,
            max,
            &mut stack,
        );
        if stack.count < before {
            self.tutorial_get_item(tutorial_item.as_ref());
        }
        left.then_some(stack)
    }
    /// The block's first up face (or any face) and its fixed tint, for block
    /// models drawn as entities (falling blocks, carried blocks).
    fn falling_block_face_sprite(&self, state: BlockStateId) -> Option<(String, u32)> {
        let pack = &self.pack;
        let models = pack
            .render
            .model_catalog
            .collect(state, horizon_core::position::BlockPos::new(0, 0, 0))
            .ok()?;
        let face = models
            .iter()
            .flat_map(|(_, m)| m.faces.iter())
            .find(|f| f.quad.facing == horizon_assets::model::Direction::Up)
            .or_else(|| models.iter().flat_map(|(_, m)| m.faces.iter()).next())?;
        let name = pack.layer_names.get(&face.layer)?.clone();
        let tint = if face.quad.tint_index >= 0 {
            let n = &self.res.facts().get(state).name;
            if n.contains("leaves") || n.contains("vine") {
                0x48B518
            } else {
                0x91BD59
            }
        } else {
            0xFFFFFF
        };
        Some((name, tint))
    }
    /// `TerrainParticle` at `pos`: the model's particle sprite and tint source
    /// 0's `colorAsTerrainParticle` (white when the block has none).
    /// `HoneyBlock.showSlideParticles` (entity event 53): five honey block
    /// particles at the entity.
    fn honey_slide_particles(&mut self, at: [f64; 3]) {
        self.honey_entity_particles(at, 53);
    }
    /// `HoneyBlock.showParticles`: the crumb count of entity event 53 or 54.
    fn honey_entity_particles(&mut self, at: [f64; 3], event: i8) {
        use horizon_client::particles::{ParticleOptions, ParticleType};
        let Some(count) = horizon_client::landing::honey_particle_count(event) else {
            return;
        };
        let Some(honey) = self
            .pack
            .assets
            .registry
            .default_state("minecraft:honey_block")
        else {
            return;
        };
        let block = [
            at[0].floor() as i32,
            at[1].floor() as i32,
            at[2].floor() as i32,
        ];
        if let Some((sprite, tint)) = self.particle_sprite(honey, block) {
            for _ in 0..count {
                self.particles.add(
                    &ParticleOptions::block(ParticleType::Block, sprite.clone(), tint),
                    at,
                    [0.0; 3],
                );
            }
        }
    }
    fn particle_sprite(&self, state: BlockStateId, pos: [i32; 3]) -> Option<(String, u32)> {
        let sprite = Self::block_particle_sprite(&self.pack, state)?;
        let chunks = &self.world.chunks;
        let (cx, cz) = (pos[0] >> 4, pos[2] >> 4);
        let region: [[Option<&horizon_client::chunk::Chunk>; 3]; 3] = std::array::from_fn(|dz| {
            std::array::from_fn(|dx| {
                chunks
                    .get(&(cx + dx as i32 - 1, cz + dz as i32 - 1))
                    .map(|c| &**c)
            })
        });
        let tint = self
            .pack
            .context
            .tables
            .terrain_particle_tint(
                state,
                pos,
                &region,
                world_biomes!(self),
                self.streaming.biome_zoom,
                self.streaming.biome_blend,
                self.streaming.nether_shading,
            )
            .unwrap_or(0xFFFFFF);
        Some((sprite, tint))
    }
    /// `BlockStateModelSet.getParticleMaterial`: the sprite of the model's
    /// `particle` texture, else its first face (entity-rendered blocks may
    /// have a particle texture without faces, and faces without one).
    fn block_particle_sprite(pack: &PackResources, state: BlockStateId) -> Option<String> {
        let catalog = &pack.render.model_catalog;
        let layer = catalog.particle(state).or_else(|| {
            let models = catalog
                .collect(state, horizon_core::position::BlockPos::new(0, 0, 0))
                .ok()?;
            models
                .iter()
                .flat_map(|(_, m)| m.faces.iter())
                .next()
                .map(|f| f.layer)
        })?;
        pack.layer_names.get(&layer).cloned()
    }
    fn particle_shape(&self, pos: [i32; 3], state: BlockStateId) -> Vec<[f64; 6]> {
        let f = self.res.facts().get(state);
        let offset = if f.outline_offset {
            self.pack
                .context
                .tables
                .offset(state, pos[0], pos[2])
                .unwrap_or([0.0; 3])
        } else {
            [0.0; 3]
        };
        f.outline
            .iter()
            .map(|b| std::array::from_fn(|i| b[i] + offset[i % 3]))
            .collect()
    }
    /// ClientLevel.addDestroyBlockEffect: a burst per outline box.
    fn destroy_particles(&mut self, pos: [i32; 3], state: BlockStateId) {
        if self.settings.extras.hide_block_particles {
            return;
        }
        let Some((sprite, tint)) = self.particle_sprite(state, pos) else {
            return;
        };
        let boxes = self.particle_shape(pos, state);
        horizon_client::particles::destroy_positions(pos, &boxes, |p, v| {
            let particle = Particle::terrain(self.particles.random(), p, v, sprite.clone(), tint);
            self.particles.push(particle);
        });
    }
    /// ClientLevel.addBreakingParticles: one crumb on the face being mined.
    fn crack_particle(&mut self, pos: [i32; 3], face: [i32; 3], state: BlockStateId) {
        if self.settings.extras.hide_block_particles {
            return;
        }
        let Some((sprite, tint)) = self.particle_sprite(state, pos) else {
            return;
        };
        let boxes = self.particle_shape(pos, state);
        // Keep client effects separate from the authoritative world RNG.
        let r = self.particles.random();
        let Some(p) = horizon_client::particles::breaking_position(r, pos, face, &boxes) else {
            return;
        };
        let mut particle = Particle::terrain(r, p, [0.0; 3], sprite, tint);
        particle.set_power(0.2);
        particle.scale(0.6);
        self.particles.push(particle);
    }
    /// `Player.dropEquipment` for the player who died: unless keep_inventory
    /// is on, `destroyVanishingCursedItems`, then `Inventory.dropAll` from
    /// `at` (the eyes less 0.3): slots 0..36, armor feet to head (36..39),
    /// the off hand (40), each thrown in a random direction. The mods hear
    /// the drops (`native_living_drops`) before they join the level.
    fn drop_inventory(&mut self, at: [f64; 3], dying: &creatures::DyingDrops<'_>) {
        let mut stacks: Vec<ItemStack> = Vec::new();
        if !self
            .ui
            .rules
            .bool_at(horizon_client::gamerules::vanilla::KEEP_INVENTORY)
        {
            stacks.extend(self.inventory.iter_mut().filter_map(Option::take));
            stacks.extend(self.armor.iter_mut().rev().filter_map(Option::take));
            stacks.extend(self.offhand.take());
            stacks.extend(self.equipment.take_all());
            stacks.retain(|s| s.enchantment_level("minecraft:vanishing_curse") == 0);
            self.apply_equipment();
        }
        let stacks = stacks.into_iter().map(|stack| (stack, at)).collect();
        self.drop_death_items(dying, stacks, 40, |game| {
            let r = &mut game.world.random;
            let strength = f64::from(r.next_float()) * 0.5;
            let angle = f64::from(r.next_float()) * std::f64::consts::TAU;
            [-angle.sin() * strength, 0.2, angle.cos() * strength]
        });
    }
    /// Item entity `i` leaves the level (`Entity.discard`); mods hear it.
    pub(super) fn remove_dropped(&mut self, i: usize) -> ItemEntity {
        let e = self.dropped.remove(i);
        self.native_entity_left(e.id);
        e
    }
    /// Experience orb `i` leaves the level (`Entity.discard`); mods hear it.
    pub(super) fn remove_orb(&mut self, i: usize) -> ExperienceOrb {
        let o = self.orbs.remove(i);
        self.native_entity_left(o.id);
        o
    }
    /// The item entity `spawn_item` just added, unless a native mod kept it
    /// out of the level (`native_entity_join_level`).
    pub(super) fn just_spawned_item(&mut self) -> Option<&mut ItemEntity> {
        let id = self.next_entity;
        self.dropped.last_mut().filter(|d| d.id == id)
    }
    fn spawn_item(
        &mut self,
        stack: ItemStack,
        position: [f64; 3],
        velocity: [f64; 3],
        pickup_delay: u32,
    ) {
        let item = stack.item.clone();
        let count = stack.count as i32;
        self.next_entity += 1;
        let p = Vec3::new(position[0], position[1], position[2]);
        // ItemEntity.bobOffs comes from the entity's own random, not the level's.
        let bob = self.particles.random().next_float() * std::f32::consts::PI * 2.0;
        self.dropped.push(ItemEntity {
            attachment_lifetime: Default::default(),
            fall_distance: 0.0,
            rotation: [0.0, 0.0],
            join: Default::default(),
            no_gravity: false,
            health: 5,
            fire_ticks: 0,
            thrower: None,
            target: None,
            id: self.next_entity,
            stack,
            position: p,
            previous: p,
            velocity: Vec3::new(velocity[0], velocity[1], velocity[2]),
            on_ground: false,
            vertical_collision: false,
            horizontal_collision: false,
            age: 0,
            pickup_delay: i64::from(pickup_delay),
            bob_offset: bob,
            ticks: 0,
            needs_sync: false,
            stuck: Vec3::ZERO,
            was_touching_water: false,
        });
        // ServerLevel.addFreshEntity.
        if self.native_joins_flush(false, self.next_entity) {
            self.native_item_spawn(self.next_entity, &item, count, position);
        }
    }
    /// The block entity at `pos` saved with its id and position
    /// (`saveWithFullMetadata`), or the saved one Horizon keeps as loaded:
    /// what a broken block's loot sees as `LootContextParams.BLOCK_ENTITY`.
    fn loot_block_entity(&self, pos: [i32; 3], block: &str) -> Option<horizon_core::nbt::Tag> {
        let key = (pos[0], pos[1], pos[2]);
        match self.world.block_entities.get(&key) {
            Some(entity) => Some(horizon_client::save::encode_block_entity(
                key, entity, block,
            )),
            None => spill::saved_entity(&self.world, pos),
        }
    }
    /// Where a loot roll at `origin` happens, for a mod's own loot kinds:
    /// the block entity it may copy from, and this level as
    /// `LootContext.getLevel` gives it (`Level.isRaining`: rain above 0.2;
    /// `isThundering`: thunder above 0.9; neither where `canHaveWeather` is
    /// false: a level with a ceiling or no sky light, or the End).
    fn loot_place_at(
        &self,
        origin: [f64; 3],
        block_entity: Option<horizon_core::nbt::Tag>,
    ) -> horizon_world::loot::NativeLootPlace {
        let (rain, thunder) = self.world.weather.levels(1.0);
        // Level.canHaveWeather: sky light, no ceiling and not the End.
        let weather =
            self.world.shape.sky_light && !self.world.shape.has_ceiling && self.dim != Dim::End;
        horizon_world::loot::NativeLootPlace {
            origin: Some(origin),
            block_entity,
            level: Some(horizon_world::loot::NativeLootLevel {
                dimension: self.dim.id().to_owned(),
                game_time: self.world.game_time,
                day_time: self.world.day_time,
                raining: weather && rain > 0.2,
                thundering: weather && thunder > 0.9,
            }),
        }
    }
    /// `Block.getDrops` for a block a player breaks: the block's loot table
    /// with the player as `this`, drawing from its server random sequence
    /// (`LootContext.Builder.create`), else the level random. The roll's
    /// origin is the block's centre and `block_entity` the entity it had.
    fn block_loot_at(
        &mut self,
        pos: [i32; 3],
        state: BlockStateId,
        held: Option<&ItemStack>,
        block_entity: Option<horizon_core::nbt::Tag>,
    ) -> Vec<ItemStack> {
        let registry = self.pack.assets.registry;
        let dynamic = self.pending.dynamic(pos);
        let place = self.loot_place_at(pos.map(|v| f64::from(v) + 0.5), block_entity);
        let mut drops =
            horizon_world::loot::with_native_loot_place(place, || {
                match world_items!(self).block_loot_sequence(registry, state) {
                    Some(sequence) => {
                        let random = self.rand_sequences.get(&sequence, self.level.seed);
                        world_items!(self)
                            .block_drops_by_player(registry, state, held, &dynamic, random)
                    }
                    None => world_items!(self).block_drops_by_player(
                        registry,
                        state,
                        held,
                        &dynamic,
                        &mut self.world.random,
                    ),
                }
            });
        let player = self.native_acting_id();
        self.native_block_loot(pos, state, Some(player), held, None, &mut drops);
        drops
    }
    /// Block.popResource
    fn pop_resource(&mut self, pos: [i32; 3], mut stack: ItemStack) {
        self.pending.attach(pos, &mut stack);
        // Block.popResource: Mth.nextDouble offsets from the level random; the
        // motion is `new ItemEntity(..)`'s own, from the item's random.
        let p = spill::pop_position(&mut self.world.random, pos);
        // Block.popResource draws its offsets before the private overload checks the gamerule.
        if !self
            .ui
            .rules
            .bool_at(horizon_client::gamerules::vanilla::BLOCK_DROPS)
        {
            return;
        }
        let v = dispenser::new_item_motion();
        self.spawn_item(stack, p, v, 10);
    }
    /// Player.drop: thrown from the eyes along the view direction.
    fn drop_held(&mut self, all: bool) {
        if self.eye_position().is_none() {
            return;
        }
        let spectator = self.spectator();
        let Some(thrown) = horizon_client::items::take_held_drop(
            &mut self.inventory[self.selected],
            all,
            spectator,
        ) else {
            return;
        };
        self.player_drop(thrown);
    }
    /// `Player.drop`: the statistics and a thrown item entity in front of the player.
    pub(super) fn player_drop(&mut self, thrown: ItemStack) -> Option<u32> {
        let eye = self.eye_position()?;
        // LivingEntity.drop: every drop swings the main hand, a menu's
        // THROW and a creative tab's drop included.
        self.swing_hand(Hand::Main, SwingAnimation::DEFAULT);
        // ServerPlayer.drop: the drop statistics, only when an item entity came of it.
        let (item, count) = (thrown.item.clone(), thrown.count as i32);
        let dropped = self.throw_from_eyes(thrown, eye, false, true)?;
        self.award_stat("dropped", &item, count);
        self.award_custom_stat("drop", 1);
        Some(dropped)
    }
    /// `Player.drop(stack, false)`, as `ItemUtils.createFilledResult` drops what no longer fits
    /// in the inventory: thrown from the eyes, with no thrower, statistic or drop hook.
    pub(super) fn drop_plain(&mut self, stack: ItemStack) -> Option<u32> {
        if stack.count == 0 {
            return None;
        }
        // LivingEntity.drop swings the main hand for every drop (`PREDICTED` only keeps the
        // swing from being sent back to the player who made it).
        self.swing_hand(Hand::Main, SwingAnimation::DEFAULT);
        let eye = self.eye_position()?;
        self.throw_from_eyes(stack, eye, false, false)
    }
    /// `Player.createItemStackToDrop`: an item entity thrown from the eyes
    /// along the view direction. `thrown_from_hand` (`drop(stack, true)`, the
    /// drop key) names the player as the thrower; `drop(stack, false)`, for
    /// what no longer fits in the inventory, does not.
    fn throw_from_eyes(
        &mut self,
        thrown: ItemStack,
        eye: [f64; 3],
        randomly: bool,
        thrown_from_hand: bool,
    ) -> Option<u32> {
        let r = &mut self.world.random;
        let v = horizon_client::entities::drop_motion(r, self.yaw, self.pitch, randomly);
        let y = eye[1] - f64::from(0.3f32);
        // ItemTossEvent: a canceled toss never enters the world; what was thrown is gone.
        // Only `drop(.., false)` tosses fire it: a death's scatter (`randomly`) does not.
        if !randomly {
            let player = self.native_acting_id();
            let mut canceled = false;
            self.native_item_toss(player, &thrown.item, thrown.count, &mut canceled);
            if canceled {
                return None;
            }
        }
        self.spawn_item(thrown, [eye[0], y, eye[2]], v, 40);
        let uuid = thrown_from_hand.then(|| self.acting_player_uuid());
        let item = self.just_spawned_item()?;
        item.thrower = uuid;
        Some(item.id)
    }
    /// The UUID of the player acting: the guest whose turn it is, else the host.
    pub(super) fn acting_player_uuid(&mut self) -> [i32; 4] {
        match self.lan_acting_uuid {
            Some(guest) => horizon_client::lan::playerdata::uuid_ints(guest),
            None => self.level.ensure_player_uuid(),
        }
    }
    /// `Entity.doWaterSplashEffect` for a dropped item. The client draws the
    /// bubble and splash particles (`Level.addParticle` is empty on a server).
    /// The server plays `entity.generic.splash` (`Entity.playSound`); a
    /// client's `playSeededSound` ignores that call.
    pub(super) fn apply_item_splash(&mut self, splash: horizon_client::entities::ItemSplash) {
        use horizon_client::entities::ItemSplash;
        let client = self.options.execution_role.is_client();
        match splash {
            ItemSplash::Sound { at, volume, pitch } if !client => {
                self.audio
                    .play("minecraft:entity.generic.splash", Some(at), volume, pitch);
            }
            ItemSplash::Particle {
                bubble,
                at,
                velocity,
            } if client => {
                use horizon_client::particles::{ParticleOptions, ParticleType};
                let kind = if bubble {
                    ParticleType::Bubble
                } else {
                    ParticleType::Splash
                };
                self.particles
                    .add(&ParticleOptions::simple(kind), at, velocity);
            }
            _ => {}
        }
    }
    fn tick_items(&mut self) {
        // EntityTickEvent.Pre for the items about to tick, and the Post after them.
        let mut round = self.native_tick_round();
        if round.listening() {
            let ticking = self.entity_ticking_area();
            let merging = self.remote.is_none();
            let mut i = 0;
            while let Some(item) = self.dropped.get(i) {
                let (id, x, z) = (
                    item.id,
                    item.position.x.floor() as i32,
                    item.position.z.floor() as i32,
                );
                if !(merging && id > self.ticked_entities_through) && ticking(x, z) {
                    let kind = native_ticks::TickKind::Fixed("minecraft:item");
                    self.native_tick_pre_batch(&mut round, id, kind);
                }
                i += 1;
            }
        }
        self.tick_dropped_items(&mut round);
        self.drain_entity_inside();
        self.native_tick_post_batch(round);
    }
    /// `Block.fallOn` for item `id`, which fell `distance` onto the block at `on`
    /// (`Entity.checkFallDamage`): a mod block's `fall_on` Part hears it, through the same
    /// `block_contact::fall` as a player's and a mob's landing. An item takes no fall damage.
    /// Nothing is lent or built unless a Part of that block listens.
    fn item_fall_on(&mut self, id: u32, on: horizon_core::position::BlockPos, distance: f64) {
        let state = self.world.block(on.x, on.y, on.z);
        if !horizon_client::block_contact::fall_listens(state) {
            return;
        }
        let mut lent = horizon_client::health::LentLevel::take(&mut self.world);
        let mut level = horizon_client::health::DamageLevel::lent(
            &self.world,
            self.pack.assets.registry,
            &mut lent,
            Some(world_items!(self)),
            Some(&self.pack.block_registries),
            &self.damage_hooks,
        );
        let contact = horizon_client::block_contact::Contact {
            entity: id as i32,
            kind: "minecraft:item",
            state,
            position: on,
        };
        let _ = horizon_client::block_contact::fall(&mut level, contact, distance, 0.0);
        lent.give_back(&mut self.world);
    }
    fn tick_dropped_items(&mut self, round: &mut native_ticks::TickRound) {
        let mut items = std::mem::take(&mut self.dropped);
        // ServerLevel.tick: only entities in entity-ticking chunks tick.
        let ticking = self.entity_ticking_area();
        let mut removed = vec![false; items.len()];
        let mut cells = horizon_client::entities::ItemCells::new(&items);
        // A LAN or server guest only shows the host's items.
        let merging = self.remote.is_none();
        let obstacles = horizon_client::entities::ObstacleGrid::new(self.item_entity_collisions());
        let mut nearby = Vec::new();
        let mut bubble_tops = Vec::new();
        // `ItemUtils.onContainerDestroyed`: what a destroyed item held, and
        // where it lay, for adding to the level once its own tick is done.
        let mut destroyed: Vec<(ItemStack, Vec3)> = Vec::new();
        for i in 0..items.len() {
            if removed[i] {
                // Merged into an earlier item before its own tick: it never ticked.
                round.not_ticked(items[i].id);
                continue;
            }
            let item = &mut items[i];
            // `addFreshEntity`: an entity added this tick ticks from the next
            // one (`EntityTickList`), so a drop's `Age` stays 0 for the tick
            // function that looks for it.
            if merging && item.id > self.ticked_entities_through {
                continue;
            }
            if !ticking(
                item.position.x.floor() as i32,
                item.position.z.floor() as i32,
            ) || round.canceled(item.id)
            {
                continue;
            }
            // ItemEntity.tick's noPhysics (servers only): out of a block
            // placed on it, towards the closest open side (#1252).
            let no_physics = merging && item.inside_block(&self.world, self.collision);
            if no_physics {
                let speed = self.world.random.next_float() * 0.2 + 0.1;
                item.move_towards_closest_space(&self.world, self.res.facts(), speed);
            }
            // `getEntityCollisions(entity, bounds.expandTowards(movement))`: the boxes around
            // where the item can reach this tick (a block of slack for the next velocity).
            let reach = Vec3::new(
                item.velocity.x.abs() + 1.0,
                item.velocity.y.abs() + 1.0,
                item.velocity.z.abs() + 1.0,
            );
            obstacles.near(item.bounds().inflate(reach), &mut nearby);
            let mut entered = Vec::new();
            let (alive, discarded) = self.tick_item_with_block_contacts(
                item,
                no_physics,
                &nearby,
                false,
                &mut |splash| entered.push(splash),
            );
            // Entity.doWaterSplashEffect: the server plays the sound (broadcast
            // to clients). Its addParticle is empty.
            for splash in entered {
                self.apply_item_splash(splash);
            }
            if discarded {
                removed[i] = true;
                cells.remove(i, item.previous);
                continue;
            }
            if merging && !no_physics {
                self.item_landing_event(item.id, item.previous, item.position, item.on_ground);
                self.entity_powder_snow_particles(
                    item.previous,
                    item.position,
                    item.bounds(),
                    item.velocity,
                    false,
                );
            }
            cells.moved(i, item.previous, item.position);
            // Entity.handleOnAboveBubbleColumn: the server sends the particles of
            // the surface of a bubble column the item is at the top of.
            if merging {
                bubble_tops.extend(horizon_client::motion::bubble_column_surfaces(
                    &self.world,
                    self.pack.assets.registry,
                    self.res.facts(),
                    self.collision,
                    item.bounds(),
                ));
            }
            // ItemEntity.tick: merge every 2 ticks while moving between
            // blocks, every 40 otherwise, before the lifetime check.
            let block = |v: Vec3| (v.x.floor(), v.y.floor(), v.z.floor());
            let rate = if block(item.previous) == block(item.position) {
                40
            } else {
                2
            };
            if merging
                && (alive || item.age == 6000 && item.position.y > -128.0)
                && item.ticks.is_multiple_of(rate)
            {
                let max_stack = |stack: &ItemStack| self.stack_max_size(stack).max(1);
                horizon_client::entities::merge_with_neighbours(
                    &mut items,
                    &mut removed,
                    &mut cells,
                    i,
                    &max_stack,
                );
            }
            let item = &mut items[i];
            let mut alive = alive || (item.age < 6000 && item.position.y > -128.0);
            if merging && !removed[i] && item.age >= 6000 && item.position.y > -128.0 {
                let mut extra_life = 0;
                self.native_item_expire(item.id, &item.stack, &mut extra_life);
                if extra_life > 0 {
                    item.age = 6000 - extra_life.min(32766);
                    alive = true;
                } else if self.serving.is_some() || self.options.dedicated.is_some() {
                    self.native_item_despawn(
                        item.id,
                        &item.stack.item,
                        item.stack.count as i32,
                        [item.position.x, item.position.y, item.position.z],
                    );
                }
            }
            if !alive || removed[i] {
                removed[i] = true;
                cells.remove(i, item.position);
                continue;
            }
            let environment = item.environment_tick(
                &self.world,
                self.res.facts(),
                self.pack.assets.registry,
                world_items!(self),
            );
            if environment.should_play_lava_hurt_sound(item.ticks) {
                let pitch = 2.0 + self.world.random.next_float() * 0.4;
                self.audio.play(
                    "minecraft:entity.generic.burn",
                    Some([item.position.x, item.position.y, item.position.z]),
                    0.4,
                    pitch,
                );
            }
            if !environment.alive {
                removed[i] = true;
                cells.remove(i, item.position);
                for stack in environment.spilled {
                    destroyed.push((stack, item.position));
                }
            }
        }
        for pos in bubble_tops {
            self.bubble_column_particles(pos);
        }
        let mut removed = removed.into_iter();
        items.retain(|_| !removed.next().unwrap_or(false));
        let Some(p) = self.player.as_ref() else {
            self.put_back_dropped(items);
            self.spawn_destroyed_contents(destroyed);
            return;
        };
        if p.health.dead_or_dying() || self.spectator() {
            self.put_back_dropped(items);
            self.spawn_destroyed_contents(destroyed);
            return;
        }
        let w = f64::from(p.dimensions.width) / 2.0;
        let (min, max) = (
            [
                p.position.x - w - 1.0,
                p.position.y - 0.5,
                p.position.z - w - 1.0,
            ],
            [
                p.position.x + w + 1.0,
                p.position.y + f64::from(p.dimensions.height) + 0.5,
                p.position.z + w + 1.0,
            ],
        );
        let feet = [p.position.x, p.position.y, p.position.z];
        let own = self.level.ensure_player_uuid();
        let mut kept = horizon_client::join_seq::JoinList::with_capacity(items.len());
        for mut item in items {
            let b = item.bounds();
            let touching = b.min_position().x < max[0]
                && b.max_position().x > min[0]
                && b.min_position().y < max[1]
                && b.max_position().y > min[1]
                && b.min_position().z < max[2]
                && b.max_position().z > min[2];
            // ItemEntity.playerTouch: no pickup delay, and the target (if any) is this player.
            if touching && item.pickup_delay == 0 && item.target.is_none_or(|t| t == own) {
                if self.native_pickup_refused(&item.stack) {
                    item.pickup_delay = 10;
                    kept.push(item);
                    continue;
                }
                let before = item.stack.count;
                let kind = item.stack.item.clone();
                let thrower = item.thrower;
                match self.add_stack_to_inventory(item.stack.clone()) {
                    None => {
                        self.award_stat("picked_up", &kind, before as i32);
                        if let Some(thrower) = thrower {
                            self.trigger_thrown_picked_up_by_player(&item.stack, thrower);
                        }
                        self.add_pickup(
                            pickups::Taken::Item(Box::new(item)),
                            pickups::Collector::Player,
                        );
                        let pitch = ((self.world.random.next_float()
                            - self.world.random.next_float())
                            * 0.7
                            + 1.0)
                            * 2.0;
                        self.audio
                            .play("minecraft:entity.item.pickup", Some(feet), 0.2, pitch);
                        continue;
                    }
                    Some(rest) => {
                        if rest.count < before {
                            self.award_stat("picked_up", &kind, (before - rest.count) as i32);
                            if let Some(thrower) = thrower {
                                self.trigger_thrown_picked_up_by_player(&rest, thrower);
                            }
                            self.add_pickup(
                                pickups::Taken::Item(Box::new(item.clone())),
                                pickups::Collector::Player,
                            );
                            // handleTakeItemEntity: the random pitch of a full pickup.
                            let pitch = ((self.world.random.next_float()
                                - self.world.random.next_float())
                                * 0.7
                                + 1.0)
                                * 2.0;
                            self.audio
                                .play("minecraft:entity.item.pickup", Some(feet), 0.2, pitch);
                        }
                        item.stack = rest;
                    }
                }
            }
            kept.push(item);
        }
        self.put_back_dropped(kept);
        self.spawn_destroyed_contents(destroyed);
    }
    /// The item entities `tick_dropped_items` took out, back in the level,
    /// before those its hooks spawned meanwhile (`addFreshEntity` appends).
    /// They keep their join numbers; the spawned ones got theirs on joining.
    fn put_back_dropped(&mut self, items: horizon_client::join_seq::JoinList<ItemEntity>) {
        let spawned = std::mem::replace(&mut self.dropped, items);
        self.dropped.extend(spawned);
    }
    /// `ItemUtils.onContainerDestroyed`: each stack a destroyed item held
    /// becomes an item entity where that item lay (`new ItemEntity(level, x,
    /// y, z, stack)`), nudged by 0.2 less 0.1 per axis and pickable at once.
    fn spawn_destroyed_contents(&mut self, destroyed: Vec<(ItemStack, Vec3)>) {
        for (stack, at) in destroyed {
            let r = &mut self.world.random;
            let motion = [
                r.next_double() * 0.2 - 0.1,
                r.next_double() * 0.2 - 0.1,
                r.next_double() * 0.2 - 0.1,
            ];
            self.spawn_item(stack, [at.x, at.y, at.z], motion, 0);
        }
    }
    /// Run vanilla block behaviour against the world through `LevelAccess`.
    fn with_level<R>(
        &mut self,
        f: impl FnOnce(&Behaviours, &mut GameLevel<'_>, &mut Ticks) -> R,
    ) -> R {
        let darken = self.sky_darken();
        let (rain, thunder) = self.weather_levels(1.0);
        // Spectators ignore block triggers (Entity.isIgnoringBlockTriggers).
        let spectator = self.spectator();
        let player_box = self.player.as_ref().filter(|_| !spectator).map(|p| {
            let w = f64::from(p.dimensions.width) / 2.0;
            (
                [p.position.x - w, p.position.y, p.position.z - w],
                [
                    p.position.x + w,
                    p.position.y + f64::from(p.dimensions.height),
                    p.position.z + w,
                ],
            )
        });
        let player_collision = self.player.as_ref().filter(|_| !spectator).map(|p| {
            let main_hand = self.inventory.get(self.selected).and_then(Option::as_ref);
            PlayerCollision {
                y: p.position.y,
                descending: p.velocity.y < 0.0,
                fall_distance: p.fall_distance,
                main_hand_item: main_hand.map_or_else(
                    || Arc::from("minecraft:air"),
                    |stack| Arc::clone(&stack.item),
                ),
                main_hand_count: main_hand.map_or(0, |stack| stack.count as i32),
                leather_boots: self.armor[3]
                    .as_ref()
                    .is_some_and(|stack| stack.item.as_ref() == "minecraft:leather_boots"),
            }
        });
        let cart_boxes = self
            .carts
            .iter()
            .map(|c| {
                let p = c.position;
                (
                    [p.x - 0.49, p.y, p.z - 0.49],
                    [p.x + 0.49, p.y + 0.7, p.z + 0.49],
                )
            })
            .collect();
        let arrow_boxes = self
            .creatures
            .projectiles
            .iter()
            .filter(|p| p.arrow.is_some())
            .map(|p| p.position)
            .chain(self.tridents.iter().map(|t| t.position))
            .map(|at| {
                let b = horizon_client::arrows::arrow_bounds(at);
                (b.min_position().components(), b.max_position().components())
            })
            .collect();
        let guest_boxes = self.lan_block_trigger_boxes();
        let building_player_boxes = self
            .lan_acting
            .is_some()
            .then(|| self.lan_building_boxes.boxes(self.dim));
        let dimension = self.dim.level();
        let rules = self.level_rules();
        let fire_players: Vec<[f64; 3]> = self
            .player
            .iter()
            .filter(|_| !spectator)
            .map(|p| [p.position.x, p.position.y, p.position.z])
            .chain(self.lan.iter().flat_map(|h| {
                h.hub
                    .guests
                    .simulation_iter()
                    .filter(|g| h.hub.here(g) && !g.spectator())
                    .map(|g| g.position)
            }))
            .collect();
        let mut trigger_hooks: Vec<(Aabb, bool)> = self
            .hook
            .iter()
            .map(|h| (h.bounds(), h.applies_block_effects()))
            .collect();
        self.lan_hook_triggers(&mut trigger_hooks);
        // EntityType.LIGHTNING_BOLT: 0 x 0.
        let trigger_extras: Vec<Aabb> = self
            .bolts
            .iter()
            .map(|b| point_box(b.at, 0.0, 0.0))
            .collect();
        // `ContainerOpenersCounter.getEntitiesWithContainerOpen`: the players
        // whose menu still holds a container of the block at a position. A
        // double chest answers for both halves (`ChestBlockEntity$1` takes a
        // `CompoundContainer`), and a spectator's menu counts for nothing.
        let mut open_containers: Vec<(i32, i32, i32)> = Vec::new();
        for key in self.container.into_iter().filter(|_| !spectator).chain(
            self.lan
                .iter()
                .flat_map(|h| {
                    h.hub
                        .guests
                        .simulation_iter()
                        .filter(|g| h.hub.here(g) && !g.spectator())
                })
                .filter_map(|g| g.menu.as_ref().and_then(|m| m.pos)),
        ) {
            open_containers.push(key);
            if let Some(other) = self.chest_other_half(key) {
                open_containers.push(other);
            }
        }
        let native_authoritative = self.native_authoritative();
        let mut level = GameLevel {
            command_block_transition: None,
            redstone_experiments: self
                .level
                .enabled_features
                .iter()
                .any(|feature| feature == "minecraft:redstone_experiments"),
            cart_boxes,
            item_frames: &self.item_frames,
            carts: &self.carts,
            arrow_boxes,
            entities: entity::entity_lists_of!(self),
            crystals: &self.crystals,
            trigger_hooks,
            trigger_extras,
            world: &mut self.world,
            facts: self.res.facts(),
            collision: self.collision,
            events: &mut self.level_events,
            dimension,
            darken,
            environment: match dimension {
                LevelDimension::Overworld => self.res.environment(),
                LevelDimension::Nether => self.res.nether_environment(),
                LevelDimension::End => self.res.end_environment(),
            },
            raining: rain > 0.2,
            thundering: thunder > 0.9,
            enabled_features: self
                .remote
                .as_ref()
                .map_or(&self.level.enabled_features, |remote| &remote.features),
            player_box,
            guest_boxes,
            building_player_boxes,
            player_collision,
            biomes: world_biomes!(self),
            seed: self.level.seed,
            behaviours: self.res.behaviours(),
            items: world_items!(self),
            recipes: world_recipes!(self),
            // A client rolls no loot (LootTable.getRandomItems needs a
            // ServerLevel): it doesn't load the tables on its game thread.
            loot: if self.remote.is_some() {
                None
            } else {
                self.server_loot.get()
            },
            registry: self.pack.assets.registry,
            block_registries: &self.pack.block_registries,
            rules,
            difficulty: i32::from(self.ui.difficulty),
            fire_players,
            rand_sequences: &mut self.rand_sequences,
            pending: &mut self.pending,
            spill: &mut self.spill,
            tickets: self.tickets.levels(),
            random_tick_sections: &mut self.random_tick_sections,
            lid_openers: &mut self.lid_openers,
            open_containers: &open_containers,
            client_side: self.remote.is_some(),
            native_authoritative,
            native_player: self.lan_acting_uuid,
            native_player_entity: self.lan_acting_entity.map(|id| id as i32),
            debug: self.world_debug,
            native_sounds: &mut self.creatures.lan_sounds,
            native_particles: &mut self.creatures.lan_particles,
            sound_seeds: self.lan.as_ref().map(|host| &host.hub),
            drop_tool: None,
            drop_into: None,
        };
        let result = f(self.res.behaviours(), &mut level, &mut self.block_ticks);
        self.flush_spill();
        if self.lan.is_some() {
            self.flush_lan_particles();
            self.flush_native_sounds();
        }
        result
    }
    pub(super) fn native_block_ignite_allowed(
        &mut self,
        pos: BlockPos,
        cause: &'static str,
    ) -> bool {
        self.with_level(|_, level, _| level.allow_block_ignite(pos, cause))
    }
    pub(super) fn native_cauldron_level_change_allowed(
        &mut self,
        pos: BlockPos,
        from: i32,
        to: i32,
        reason: &'static str,
    ) -> bool {
        self.with_level(|_, level, _| level.allow_cauldron_level_change(pos, from, to, reason))
    }
    /// Level.setBlock(pos, state, UPDATE_ALL) with neighbour/shape updates.
    fn level_set(&mut self, pos: [i32; 3], state: BlockStateId) -> Result<()> {
        self.level_set_with_flags(pos, state, level_flags::UPDATE_ALL)
    }
    /// Level.setBlock with the caller's update flags.
    fn level_set_with_flags(
        &mut self,
        pos: [i32; 3],
        state: BlockStateId,
        flags: i32,
    ) -> Result<()> {
        self.with_level(|b, level, ticks| {
            b.set_block(
                level,
                ticks,
                BlockPos::new(pos[0], pos[1], pos[2]),
                state,
                flags,
            );
        });
        self.process_level_events()
    }
    /// `BlockState.canSurvive(level, pos)` for `state` at `pos` in the level in
    /// place: the rule a placement checks before `level_set` (a sapling on
    /// grass, a torch on a wall), with a mod block's own `native_can_survive`
    /// hook. False for an invalid state, an unloaded or out-of-height
    /// position, and on a client.
    ///
    /// This is the non-closure equivalent of `with_level(|b, level, ticks|
    /// b.can_survive(level, ticks, pos, state))`, for a wasm mod whose
    /// internals view cannot send a closure.
    // A wasm mod's staged internals table calls this; nothing in Horizon does.
    #[allow(dead_code)]
    fn level_can_survive(&mut self, pos: [i32; 3], state: BlockStateId) -> bool {
        if !self.is_logical_server()
            || !self.world.loaded(pos[0], pos[2])
            || !self.world.shape.contains_y(pos[1])
            || self.pack.assets.registry.state(state).is_none()
        {
            return false;
        }
        self.with_level(|b, level, ticks| {
            b.can_survive(level, ticks, BlockPos::new(pos[0], pos[1], pos[2]), state)
        })
    }
    fn attack_block(&mut self, pos: [i32; 3]) -> Result<()> {
        let note = self.with_level(|b, level, ticks| {
            b.attack_block(level, ticks, BlockPos::new(pos[0], pos[1], pos[2]))
        });
        // NoteBlock.attack awards the statistic.
        if note {
            self.award_custom_stat("play_noteblock", 1);
        }
        self.process_level_events()
    }
    /// `AbstractCandleBlock.getParticleOffsets`, in sixteenths of a block:
    /// `CandleBlock.PARTICLE_OFFSETS` by its `candles` (one to four wicks),
    /// then `CandleCakeBlock.PARTICLE_OFFSETS`, whose one wick sits a block up.
    fn candle_offsets(name: &str, candles: Option<usize>) -> &'static [[u8; 3]] {
        const OFFSETS: [&[[u8; 3]]; 5] = [
            &[[8, 8, 8]],
            &[[6, 7, 8], [10, 8, 7]],
            &[[8, 5, 10], [6, 7, 8], [9, 8, 7]],
            &[[7, 5, 9], [10, 7, 9], [6, 7, 6], [9, 8, 6]],
            &[[8, 16, 8]],
        ];
        if name.ends_with("candle_cake") {
            OFFSETS[4]
        } else {
            OFFSETS[candles.unwrap_or(1).clamp(1, 4) - 1]
        }
    }
    /// `AbstractCandleBlock.extinguish`: the smoke from each wick, at the
    /// block's `getParticleOffsets` (sixteenths of a block), rising 0.1.
    fn candle_smoke(&mut self, pos: [i32; 3], state: BlockStateId) {
        use horizon_client::particles::{ParticleOptions, ParticleType};
        let candles = self
            .pack
            .assets
            .registry
            .state(state)
            .and_then(|s| s.properties.get("candles")?.parse::<usize>().ok());
        let offsets = Self::candle_offsets(&self.res.facts().get(state).name, candles);
        for o in offsets {
            self.particles.add(
                &ParticleOptions::simple(ParticleType::Smoke),
                [
                    f64::from(pos[0]) + f64::from(o[0]) / 16.0,
                    f64::from(pos[1]) + f64::from(o[1]) / 16.0,
                    f64::from(pos[2]) + f64::from(o[2]) / 16.0,
                ],
                [0.0, 0.1, 0.0],
            );
        }
    }
    /// `LeverBlock.makeParticle` at full scale: a powered lever puffs one red
    /// dust on the clicking client (`LeverBlock.useWithoutItem`).
    fn lever_puff(&mut self, pos: [i32; 3]) {
        // The server's side of a guest's pull draws nothing.
        if self.lan_acting.is_some() {
            return;
        }
        let state = self.world.block(pos[0], pos[1], pos[2]);
        let Some(registry_state) = self.pack.assets.registry.state(state) else {
            return;
        };
        let prop = |key: &str| registry_state.properties.get(key).map(String::as_str);
        if prop("powered") != Some("true") {
            return;
        }
        // FACING is horizontal; getConnectedDirection is the face's direction.
        let facing = match prop("facing") {
            Some("north") => [0, 0, -1],
            Some("south") => [0, 0, 1],
            Some("west") => [-1, 0, 0],
            Some("east") => [1, 0, 0],
            _ => return,
        };
        let connected = match prop("face") {
            Some("ceiling") => [0, -1, 0],
            Some("floor") => [0, 1, 0],
            _ => facing,
        };
        let at = |base: i32, i: usize| {
            f64::from(base) + 0.5 + 0.1 * f64::from(-facing[i]) + 0.2 * f64::from(-connected[i])
        };
        let p = [at(pos[0], 0), at(pos[1], 1), at(pos[2], 2)];
        self.particles.add(
            &horizon_client::particles::ParticleOptions::dust([1.0, 0.0, 0.0], 1.0),
            p,
            [0.0; 3],
        );
    }
    /// Track which chunks are ticking (Ticks needs chunk load/unload).
    fn sync_tick_chunks(&mut self) {
        if !self.tick_chunks.synchronize(&self.world.chunks) {
            return;
        }
        // Move only the added keys out so registration can borrow all of Game.
        let added = self.tick_chunks.take_added();
        for &key in &added {
            self.register_chunk_ticks(key);
        }
        for &key in self.tick_chunks.removed() {
            self.random_tick_sections.remove(&key);
            self.portals.sections.remove(&key);
            self.block_ticks
                .remove_chunk(horizon_core::position::ChunkPos::new(key.0, key.1));
            // Its moving pistons were saved with it and load back with it.
            self.block_ticks
                .moving_pistons
                .retain(|(p, _)| (p.x >> 4, p.z >> 4) != key);
        }
        self.tick_chunks.recycle_added(added);
    }
    /// `DistanceManager`'s tickets of the level in place, from every player in
    /// it (the local player, LAN guests, a server's players; a spectator the
    /// rule skips holds none), its forced chunks and its portal tickets. A game
    /// with neither a server nor a player (the headless fixtures) ticks
    /// everywhere.
    fn update_chunk_tickets(&mut self) {
        let rule = self
            .ui
            .rules
            .bool_at(horizon_client::gamerules::vanilla::SPECTATORS_GENERATE_CHUNKS);
        let mut players: Vec<((i32, i32), bool)> = self.lan.as_ref().map_or_else(Vec::new, |h| {
            h.hub
                .guests
                .iter()
                .filter(|g| h.hub.here(g))
                .map(|g| (g.chunk(), g.spectator()))
                .collect()
        });
        if let Some(p) = &self.player {
            players.push((
                (
                    (p.position.x.floor() as i32) >> 4,
                    (p.position.z.floor() as i32) >> 4,
                ),
                self.spectator(),
            ));
        }
        // A spectator the rule skips is still a player: a game that has one
        // must not take the headless fixtures' tick-everywhere case below.
        let any_player = !players.is_empty();
        let sources = (!self.options.execution_role.is_client() || any_player).then(|| {
            horizon_client::ticket_levels::Sources {
                players: players
                    .into_iter()
                    .filter(|&(_, spectator)| game_mode::loads_chunks(rule, spectator))
                    .map(|(chunk, _)| chunk)
                    .collect(),
                forced: self.streaming.forced.iter().copied().collect(),
                // Filled from the `PORTAL` tickets by `update`.
                portal: Vec::new(),
                // EnderDragonFight.tick: while a player watches the fight, the
                // arena holds a `DRAGON` ticket of radius 9 at (0, 0), so its
                // island ticks out to 7 chunks whatever the watcher's
                // simulation distance. The fight lives in the End only.
                dragon: (self.dim == Dim::End && self.fight.arena_watched()).then_some((0, 0)),
                view_distance: self.settings.render_distance.clamp(2, 32),
                simulation_distance: self.settings.simulation_distance.max(2),
            }
        });
        self.tickets.update(sources);
    }
    /// `ServerLevel.startTickingChunk`: a loaded chunk's saved ticks are
    /// scheduled from the game time at which it becomes a ticking chunk
    /// (block ticking, with its neighbours loaded). Chunks become ticking
    /// between ticks in vanilla: before this tick's clock advanced.
    fn start_ticking_chunks(&mut self) {
        let levels = self.tickets.levels();
        let world = &self.world;
        let started = self
            .tick_chunks
            .start_ticking(|(x, z)| levels.loaded_ticking(x, z) && ticking_chunk(world, x, z));
        let time = self.world.game_time - 1;
        for (x, z) in started {
            let pos = horizon_core::position::ChunkPos::new(x, z);
            self.block_ticks.blocks.unpack_chunk(pos, time);
            self.block_ticks.fluids.unpack_chunk(pos, time);
        }
    }
    /// ServerLevel.tick block phases: scheduled ticks, random ticks in the
    /// chunks the tickets simulate, block events and moving pistons.
    /// The length the block log had after the changed blocks went out
    /// (`ServerChunkCache.tick`'s broadcast), before the block events and the
    /// block entities ran.
    fn tick_blocks(&mut self) -> Result<Option<usize>> {
        let mut laps = horizon_client::spans::laps();
        // ServerChunkCache.tick: the tickets that time out.
        self.tickets.tick_portal();
        self.sync_tick_chunks();
        self.start_ticking_chunks();
        laps.lap("tb_tickets");
        let levels = self.tickets.levels();
        let bounds = levels.random_tick_bounds();
        // Keep the world map's chunk order, including empty chunks
        // (tick_chunk_random still draws for precipitation in those).
        let ticking: Vec<_> = self
            .world
            .chunks
            .keys()
            .copied()
            .filter(|&(x, z)| {
                if let Some((min_x, min_z, max_x, max_z)) = bounds
                    && (x < min_x || x > max_x || z < min_z || z > max_z)
                {
                    return false;
                }
                levels.random_ticking(x, z) && ticking_chunk(&self.world, x, z)
            })
            .collect();
        laps.lap("tb_chunks");
        // Entity.checkInsideBlocks for the plates, tripwires, detector rails and
        // buttons under the players (the host and the LAN guests), the minecarts
        // and the arrows (the mobs and other entities are added in `with_level`
        // below).
        let mut inside = Vec::new();
        let guests = self.lan_block_trigger_boxes();
        let boxes = self
            .player
            .iter()
            .filter(|_| !self.spectator())
            .map(|p| {
                let w = f64::from(p.dimensions.width) / 2.0;
                let h = f64::from(p.dimensions.height);
                (
                    [p.position.x - w, p.position.y, p.position.z - w],
                    [p.position.x + w, p.position.y + h, p.position.z + w],
                )
            })
            .chain(self.carts.iter().map(|c| {
                let p = c.position;
                (
                    [p.x - 0.49, p.y, p.z - 0.49],
                    [p.x + 0.49, p.y + 0.7, p.z + 0.49],
                )
            }))
            .chain(guests);
        for (lo, hi) in boxes {
            for x in (lo[0] + 1e-7).floor() as i32..=(hi[0] - 1e-7).floor() as i32 {
                for y in (lo[1] + 1e-7).floor() as i32..=(hi[1] - 1e-7).floor() as i32 {
                    for z in (lo[2] + 1e-7).floor() as i32..=(hi[2] - 1e-7).floor() as i32 {
                        inside.push(BlockPos::new(x, y, z));
                    }
                }
            }
        }
        // AbstractArrow.applyEffectsFromBlocks: the blocks every arrow passed
        // through this tick (a wooden button is pressed by one inside it), and
        // the ones a thrown trident is in.
        for p in self
            .creatures
            .projectiles
            .iter()
            .filter(|p| p.arrow.is_some())
        {
            inside.extend(horizon_client::arrows::blocks_inside(
                p.previous, p.position,
            ));
        }
        for t in &self.tridents {
            inside.extend(horizon_client::arrows::blocks_inside(
                t.position, t.position,
            ));
        }
        // BigDripleafBlock.entityInside also asks whether the player stands on
        // the ground and how high its feet are: the leaves its box's feet
        // layer is inside (the host and the LAN guests).
        let mut standing = Vec::new();
        let feet = self
            .player
            .iter()
            .filter(|_| !self.spectator())
            .map(|p| {
                (
                    [p.position.x, p.position.y, p.position.z],
                    p.ground.on_ground,
                    f64::from(p.dimensions.width) / 2.0,
                )
            })
            .chain(
                self.lan_guest_feet()
                    .into_iter()
                    .map(|(at, on_ground)| (at, on_ground, 0.3)),
            );
        for (at, on_ground, w) in feet {
            let y = (at[1] + 1e-7).floor() as i32;
            for x in (at[0] - w + 1e-7).floor() as i32..=(at[0] + w - 1e-7).floor() as i32 {
                for z in (at[2] - w + 1e-7).floor() as i32..=(at[2] + w - 1e-7).floor() as i32 {
                    standing.push((BlockPos::new(x, y, z), on_ground, at[1]));
                }
            }
        }
        self.with_level(|b, level, ticks| {
            // LilyPadBlock.entityInside: a boat inside a lily pad breaks it.
            let mut boat_cells: Vec<BlockPos> = Vec::new();
            for bounds in level.boat_trigger_boxes() {
                let (lo, hi) = (bounds.min_position(), bounds.max_position());
                for x in (lo.x + 1e-7).floor() as i32..=(hi.x - 1e-7).floor() as i32 {
                    for y in (lo.y + 1e-7).floor() as i32..=(hi.y - 1e-7).floor() as i32 {
                        for z in (lo.z + 1e-7).floor() as i32..=(hi.z - 1e-7).floor() as i32 {
                            boat_cells.push(BlockPos::new(x, y, z));
                        }
                    }
                }
            }
            for pos in boat_cells {
                b.boat_inside(level, ticks, pos);
            }
            // The mobs and the other entities (items, orbs, projectiles...) too.
            let mut touched: Vec<BlockPos> = Vec::new();
            use entity::{Entity as _, for_each_entity};
            let mut touch = |bounds: Aabb| {
                let (lo, hi) = (bounds.min_position(), bounds.max_position());
                for x in (lo.x + 1e-7).floor() as i32..=(hi.x - 1e-7).floor() as i32 {
                    for y in (lo.y + 1e-7).floor() as i32..=(hi.y - 1e-7).floor() as i32 {
                        for z in (lo.z + 1e-7).floor() as i32..=(hi.z - 1e-7).floor() as i32 {
                            touched.push(BlockPos::new(x, y, z));
                        }
                    }
                }
            };
            for_each_entity!(level.entities, entity => {
                if entity.triggers_blocks() && entity.sweeps_blocks_inside() {
                    touch(entity.bounds());
                }
            });
            for crystal in level.crystals {
                touch(crystal.bounds());
            }
            for &(bounds, applies) in &level.trigger_hooks {
                if applies {
                    touch(bounds);
                }
            }
            for &bounds in &level.trigger_extras {
                touch(bounds);
            }
            for pos in inside {
                b.entity_inside(level, ticks, pos);
            }
            // The touched blocks go once each, in (x, y, z) order. Only an
            // entity-sensing block does anything there, so the others are
            // passed over while no block has changed since they were looked
            // at (they are still not sensing), and only the few sensing ones
            // are sorted. Once a block changes, every later one goes, as the
            // whole sorted list would.
            let key = |p: &BlockPos| (p.x, p.y, p.z);
            let (mut sensing, rest): (Vec<_>, Vec<_>) = touched
                .into_iter()
                .partition(|p| b.senses_entities(level.world.block(p.x, p.y, p.z)));
            sensing.sort_unstable_by_key(key);
            sensing.dedup();
            let looked = (
                level.world.revision,
                level.world.chunks.membership_revision(),
            );
            let mut i = 0;
            while i < sensing.len() {
                let pos = sensing[i];
                b.entity_inside(level, ticks, pos);
                i += 1;
                let now = (
                    level.world.revision,
                    level.world.chunks.membership_revision(),
                );
                if now != looked {
                    let mut later: Vec<_> = rest
                        .into_iter()
                        .chain(sensing.drain(i..))
                        .filter(|p| key(p) > key(&pos))
                        .collect();
                    later.sort_unstable_by_key(key);
                    later.dedup();
                    for pos in later {
                        b.entity_inside(level, ticks, pos);
                    }
                    break;
                }
            }
            for (pos, on_ground, feet_y) in standing {
                b.entity_on_big_dripleaf(level, ticks, pos, on_ground, feet_y);
            }
        });
        self.process_level_events()?;
        laps.lap("tb_entity_inside");
        // ServerLevel.tick: a Debug world runs no scheduled block or fluid ticks, and
        // ServerChunkCache.tickChunks no random ticks.
        let debug = self.world_debug;
        if !debug {
            self.tick_scheduled()?;
        }
        self.with_level(|b, level, ticks| {
            horizon_client::spans::time("random_ticks", || {
                for (x, z) in ticking.into_iter().filter(|_| !debug) {
                    // The last answer, asked from Y `from` at a world revision:
                    // the next ticking section and its mask. It stands for a
                    // later Y up to that section's until a block changes.
                    let mut last: Option<(u64, i32, Option<(i32, std::rc::Rc<_>)>)> = None;
                    let pos = horizon_core::position::ChunkPos::new(x, z);
                    b.tick_chunk_random_masked(level, ticks, pos, |level, from| {
                        let revision = level.world.revision;
                        let stands = last.as_ref().is_some_and(|(r, f, next)| {
                            *r == revision
                                && *f <= from
                                && next.as_ref().is_none_or(|(y, _)| from <= *y)
                        });
                        if !stands {
                            let next = level.world.chunks.get(&(x, z)).and_then(|chunk| {
                                level.random_tick_sections.next_masked(
                                    (x, z),
                                    chunk,
                                    level.facts,
                                    |id| b.random_tick_candidate(BlockStateId(id)),
                                    from,
                                )
                            });
                            last = Some((revision, from, next));
                        }
                        last.as_ref()
                            .and_then(|(_, _, next)| next.as_ref())
                            .map(|(y, mask)| (*y, Some(mask.clone())))
                    });
                }
            });
        });
        laps = horizon_client::spans::laps();
        let sent = self.lan_broadcast_changes();
        laps.lap("tb_broadcast");
        self.with_level(|b, level, ticks| {
            b.run_block_events(level, ticks);
        });
        laps.lap("tb_block_events");
        self.process_level_events()?;
        laps.lap("tb_level_events");
        self.sync_moving_pistons();
        laps.lap("tb_pistons");
        Ok(sent)
    }
    /// `ServerLevel.tick`: moving block entities push after entity travel.
    fn tick_moving_pistons(&mut self) -> Result<()> {
        self.with_level(|b, level, ticks| b.tick_block_entities(level, ticks));
        let pushes = std::mem::take(&mut self.block_ticks.piston_pushes);
        self.push_piston_entities(pushes);
        self.process_level_events()?;
        self.sync_moving_pistons();
        Ok(())
    }
    /// Copies the moving-piston block entities into the world, where the
    /// collision queries and the renderer read them.
    fn sync_moving_pistons(&mut self) {
        let previous = std::mem::take(&mut self.world.moving_pistons);
        let current: Vec<_> = self
            .block_ticks
            .moving_pistons
            .iter()
            .map(|(p, piston)| ([p.x, p.y, p.z], piston.shape_state()))
            .collect();
        let mut fallback: Vec<_> = previous
            .into_iter()
            .filter(|(pos, _)| !current.iter().any(|(next, _)| next == pos))
            .collect();
        // A finished piston stays drawn until the chunk mesh shows the block
        // it placed, however many ticks that takes.
        let older = std::mem::take(&mut self.world.moving_piston_fallback);
        for (pos, piston) in older {
            let replaced = current.iter().chain(&fallback).any(|(p, _)| *p == pos);
            if !replaced && self.section_mesh_pending(pos) {
                fallback.push((pos, piston));
            }
        }
        self.world.moving_piston_fallback = fallback;
        self.world.moving_pistons = current;
    }
    /// `ServerLevel.tick`'s scheduled block then fluid ticks, one at a time:
    /// the side effects of each (a shrieker summoning a warden, drops, game
    /// events) happen before the next tick runs, as they do inline in vanilla.
    pub(super) fn tick_scheduled(&mut self) -> Result<()> {
        for fluids in [false, true] {
            let mut laps = horizon_client::spans::laps();
            let name = if fluids { "tb_fluids" } else { "tb_scheduled" };
            if !self.with_level(|b, level, ticks| b.scheduled_begin(level, ticks, fluids)) {
                laps.lap(name);
                continue;
            }
            loop {
                // Due ticks run against one level while each leaves nothing for
                // the game to apply (level events, spilled items or orbs): the
                // next one would see the same level built again.
                let more = self.with_level(|b, level, ticks| {
                    loop {
                        let more = b.scheduled_step(level, ticks, fluids);
                        if !more || !level.events.is_empty() || !level.spill.is_empty() {
                            break more;
                        }
                    }
                });
                if !self.level_events.is_empty() {
                    self.process_level_events()?;
                }
                if !more {
                    break;
                }
            }
            self.with_level(|b, _, ticks| b.scheduled_finish(ticks, fluids));
            laps.lap(name);
        }
        Ok(())
    }
    /// Side effects reported by the behaviour library, in vanilla order.
    fn process_level_events(&mut self) -> Result<()> {
        let events = std::mem::take(&mut self.level_events);
        self.lan_record_events(&events);
        dispense_fixture::record_events(&events);
        for event in events {
            match event {
                LevelEvent::LevelEvent {
                    id: 2001,
                    pos,
                    data,
                    ..
                } => {
                    // LevelEventHandler: destroy particles and the block's
                    // break sound (none for air).
                    let state = BlockStateId(data as u32);
                    let p = [pos.x, pos.y, pos.z];
                    self.destroy_particles(p, state);
                    if self.res.facts().is_air(state) {
                        continue;
                    }
                    let sounds = self.res.facts().get(state).sounds.clone();
                    let c = [
                        f64::from(pos.x) + 0.5,
                        f64::from(pos.y) + 0.5,
                        f64::from(pos.z) + 0.5,
                    ];
                    self.audio.play(
                        &sounds.break_sound,
                        Some(c),
                        (sounds.volume + 1.0) / 2.0,
                        sounds.pitch * 0.8,
                    );
                }
                LevelEvent::LevelEvent {
                    id: 3006,
                    pos,
                    data,
                    ..
                } => self.sculk_charge_event(pos, data),
                LevelEvent::LevelEvent { id: 3007, pos, .. } => self.shriek_event(pos),
                // LevelEventHandler 1010 and 1011: a jukebox's song starts
                // (`playJukeboxSong`) or stops, and the parrots nearby dance or stop.
                LevelEvent::LevelEvent {
                    id: 1010,
                    pos,
                    data,
                    ..
                } => self.jukebox_song_event([pos.x, pos.y, pos.z], data),
                LevelEvent::LevelEvent { id: 1011, pos, .. } => {
                    self.jukebox_stop_event([pos.x, pos.y, pos.z])
                }
                // LevelEventHandler 2019 and 2020: the crack on a face of the
                // block being mined, with its hit sound for 2020.
                LevelEvent::LevelEvent {
                    id: id @ (2019 | 2020),
                    pos,
                    data,
                    ..
                } => {
                    let face = horizon_core::geometry::Direction::ALL[data.clamp(0, 5) as usize];
                    self.breaking_block_effects([pos.x, pos.y, pos.z], face.normal(), id == 2020);
                }
                LevelEvent::LevelEvent {
                    id: 3008,
                    pos,
                    data,
                    ..
                } => self.brush_complete_event([pos.x, pos.y, pos.z], data),
                LevelEvent::LevelEvent { id: 2004, pos, .. } => {
                    self.spawner_burst([pos.x, pos.y, pos.z])
                }
                // LevelEventHandler 1505: the growth particles, then the sound.
                LevelEvent::LevelEvent {
                    id: 1505,
                    pos,
                    data,
                    ..
                } => {
                    self.growth_particles(pos, data);
                    let at = [pos.x, pos.y, pos.z].map(|v| f64::from(v) + 0.5);
                    self.audio
                        .play("minecraft:item.bone_meal.use", Some(at), 1.0, 1.0);
                }
                LevelEvent::LevelEvent {
                    id: 2015,
                    pos,
                    data,
                    ..
                } => self.dragon_egg_particles([pos.x, pos.y, pos.z], data),
                LevelEvent::LevelEvent {
                    id: 2016,
                    pos,
                    data,
                    ..
                } => self.shulker_teleport_particles([pos.x, pos.y, pos.z], data),
                LevelEvent::LevelEvent {
                    id: 2017,
                    pos,
                    data,
                    ..
                } => self.consume_teleport_particles([pos.x, pos.y, pos.z], data),
                LevelEvent::LevelEvent {
                    id: 2018,
                    pos,
                    data,
                    ..
                } => self.enderman_teleport_particles([pos.x, pos.y, pos.z], data),
                LevelEvent::LevelEvent { id: 2003, pos, .. } => {
                    self.eye_break_particles([pos.x, pos.y, pos.z]);
                }
                LevelEvent::LevelEvent {
                    id: 2013,
                    pos,
                    data,
                    ..
                } => self.smash_dust(pos, data),
                // LevelEventHandler 2002 and 2007: a potion's splash.
                LevelEvent::LevelEvent {
                    id: 2002,
                    pos,
                    data,
                    ..
                } => self.potion_splash_particles(pos, data, false),
                LevelEvent::LevelEvent {
                    id: 2007,
                    pos,
                    data,
                    ..
                } => self.potion_splash_particles(pos, data, true),
                LevelEvent::LevelEvent {
                    id: id @ 3002..=3005,
                    pos,
                    data,
                    ..
                } => self.block_particle_event(id, pos, data),
                LevelEvent::LevelEvent {
                    id: 1500,
                    pos,
                    data,
                    ..
                } => self.composter_fill_event(pos, data == 1),
                // LevelEventHandler 1504: the stalactite's drip particle.
                LevelEvent::LevelEvent { id: 1504, pos, .. } => self.dripstone_drip_particle(pos),
                // LevelEventHandler 2006: the dragon fireball's burst.
                LevelEvent::LevelEvent {
                    id: 2006,
                    pos,
                    data,
                    ..
                } => self.dragon_fireball_burst(pos, data),
                // LevelEventHandler 2009: a wet sponge drying.
                LevelEvent::LevelEvent { id: 2009, pos, .. } => {
                    self.puffs_above_block(pos, horizon_client::particles::ParticleType::Cloud)
                }
                // LevelEventHandler 2011 and 2012: a bee growing a crop, a
                // turtle egg laid.
                LevelEvent::LevelEvent {
                    id: 2011 | 2012,
                    pos,
                    data,
                    ..
                } => self.turtle_egg_particles(pos, data),
                // LevelEventHandler 2014: a straw bed destroyed, with no sound.
                LevelEvent::LevelEvent {
                    id: 2014,
                    pos,
                    data,
                    ..
                } => self.destroy_particles([pos.x, pos.y, pos.z], BlockStateId(data as u32)),
                // EnderDragonFight.spawnNewGateway: an always-visible burst
                // and a sound at the gateway for every nearby player.
                LevelEvent::LevelEvent { id: 3000, pos, .. } => {
                    let at = [
                        f64::from(pos.x) + 0.5,
                        f64::from(pos.y) + 0.5,
                        f64::from(pos.z) + 0.5,
                    ];
                    self.particles.add_with(
                        &horizon_client::particles::ParticleOptions::simple(
                            horizon_client::particles::ParticleType::ExplosionEmitter,
                        ),
                        at,
                        [0.0; 3],
                        true,
                        true,
                    );
                    let random = self.particles.level_random();
                    let pitch = (1.0 + (random.next_float() - random.next_float()) * 0.2) * 0.7;
                    self.audio.play_unshared(
                        "minecraft:block.end_gateway.spawn",
                        Some(at),
                        10.0,
                        pitch,
                    );
                }
                // LevelEventHandler 3009: a sniffer egg cracking.
                LevelEvent::LevelEvent { id: 3009, pos, .. } => {
                    self.sniffer_egg_crack_particles(pos)
                }
                LevelEvent::LevelEvent {
                    id: id @ 3011..=3021,
                    pos,
                    data,
                    ..
                } => self.trial_level_event(id, [pos.x, pos.y, pos.z], data),
                // LevelEvent.SOUND_PORTAL_TRAVEL: LevelEventHandler.levelEvent
                // plays SimpleSoundInstance.forLocalAmbience (ambient, no
                // position). EndPortalBlock.entityInside travels through
                // TeleportTransition.PLAY_PORTAL_SOUND, which sends 1032 to
                // the traveling player. The packet's position is BlockPos.ZERO,
                // so a positional play would not be heard; at None is the
                // relative sound, and it is not captured for other players.
                LevelEvent::LevelEvent { id: 1032, .. } => {
                    let pitch = self.particles.level_random().next_float() * 0.4 + 0.8;
                    self.audio.play_on(
                        horizon_client::sound_engine::AMBIENT,
                        "minecraft:block.portal.travel",
                        None,
                        0.25,
                        pitch,
                    );
                }
                LevelEvent::LevelEvent { id, pos, .. } => {
                    let c = Some([
                        f64::from(pos.x) + 0.5,
                        f64::from(pos.y) + 0.5,
                        f64::from(pos.z) + 0.5,
                    ]);
                    let sound = match id {
                        1503 => Some(("minecraft:block.end_portal_frame.fill", 1.0)),
                        1501 => Some(("minecraft:block.lava.extinguish", 0.5)),
                        1009 => Some(("minecraft:block.fire.extinguish", 0.5)),
                        1502 => Some(("minecraft:block.redstone_torch.burnout", 0.5)),
                        1033 => Some(("minecraft:block.chorus_flower.grow", 1.0)),
                        1034 => Some(("minecraft:block.chorus_flower.death", 1.0)),
                        1045 => Some(("minecraft:block.pointed_dripstone.land", 2.0)),
                        1052 => Some(("minecraft:block.sulfur_spike.land", 2.0)),
                        1046 => Some((
                            "minecraft:block.pointed_dripstone.drip_lava_into_cauldron",
                            2.0,
                        )),
                        1047 => Some((
                            "minecraft:block.pointed_dripstone.drip_water_into_cauldron",
                            2.0,
                        )),
                        // LevelEvent.SOUND_ZOMBIE_WOODEN_DOOR (attack, break).
                        1019 => Some(("minecraft:entity.zombie.attack_wooden_door", 2.0)),
                        1021 => Some(("minecraft:entity.zombie.break_wooden_door", 2.0)),
                        1026 => Some(("minecraft:entity.zombie.infect", 2.0)),
                        1027 => Some(("minecraft:entity.zombie_villager.converted", 2.0)),
                        1030 => Some(("minecraft:block.anvil.use", 1.0)),
                        1029 => Some(("minecraft:block.anvil.destroy", 1.0)),
                        1031 => Some(("minecraft:block.anvil.land", 0.3)),
                        1042 => Some(("minecraft:block.grindstone.use", 1.0)),
                        1044 => Some(("minecraft:block.smithing_table.use", 1.0)),
                        // LevelEvent.SOUND_PAGE_TURN (a lectern's page).
                        1043 => Some(("minecraft:item.book.page_turn", 1.0)),
                        // LevelEvent.SOUND_SPLASH_POTION and its instant twin.
                        1053 | 1054 => Some(("minecraft:entity.splash_potion.break", 1.0)),
                        _ => None,
                    };
                    if let Some((name, volume)) = sound {
                        let pitch = match id {
                            1009 => {
                                2.6 + (self.world.random.next_float()
                                    - self.world.random.next_float())
                                    * 0.8
                            }
                            // LevelEventHandler draws these from the client
                            // level's random, before 1501's and 1502's smoke.
                            1501 | 1502 => {
                                let r = self.particles.level_random();
                                2.6 + (r.next_float() - r.next_float()) * 0.8
                            }
                            1045 | 1046 | 1047 | 1052 => {
                                self.particles.level_random().next_float() * 0.1 + 0.9
                            }
                            1029 | 1030 | 1042 | 1043 | 1044 | 1053 | 1054 => {
                                self.world.random.next_float() * 0.1 + 0.9
                            }
                            1026 | 1027 => {
                                let r = self.particles.level_random();
                                (r.next_float() - r.next_float()) * 0.2 + 1.0
                            }
                            1019 | 1021 => {
                                (self.world.random.next_float() - self.world.random.next_float())
                                    * 0.2
                                    + 1.0
                            }
                            _ => 1.0,
                        };
                        if matches!(
                            id,
                            1019 | 1021
                                | 1026
                                | 1027
                                | 1029
                                | 1030
                                | 1045
                                | 1052
                                | 1053
                                | 1054
                                | 1501
                                | 1503
                        ) {
                            // Clients play these sounds from their level events;
                            // capturing it also sends a duplicate sound packet.
                            self.audio.play_unshared(name, c, volume, pitch);
                        } else {
                            self.audio.play(name, c, volume, pitch);
                        }
                    }
                    // LevelEventHandler draws smoke after the sound.
                    match id {
                        1501 => self.puffs_above_block(
                            pos,
                            horizon_client::particles::ParticleType::LargeSmoke,
                        ),
                        1502 => self.torch_burnout_smoke(pos),
                        1503 => self.end_portal_frame_smoke(pos),
                        _ => {}
                    }
                }
                LevelEvent::ContainerLootTable { pos, tag } => {
                    let key = (pos.x, pos.y, pos.z);
                    if let Some(mut entity) = self.world.block_entities.remove(&key) {
                        self.take_loot_table(key, &tag, &mut entity);
                        self.world.block_entities.insert(key, entity);
                        self.world.modified.insert((pos.x >> 4, pos.z >> 4));
                    }
                }
                // A prime the mods refused is silent: no fuse sound, no game event.
                LevelEvent::Sound { pos, ref sound, .. }
                    if self.tnt_refused == Some(pos)
                        && &**sound == "minecraft:entity.tnt.primed" => {}
                LevelEvent::GameEvent {
                    pos,
                    event: "minecraft:prime_fuse",
                    ..
                } if self.tnt_refused == Some(pos) => self.tnt_refused = None,
                LevelEvent::Sound {
                    pos,
                    sound,
                    volume,
                    pitch,
                } => {
                    let c = [
                        f64::from(pos.x) + 0.5,
                        f64::from(pos.y) + 0.5,
                        f64::from(pos.z) + 0.5,
                    ];
                    let name = if sound.contains(':') {
                        sound.to_string()
                    } else {
                        format!("minecraft:{sound}")
                    };
                    self.audio.play(&name, Some(c), volume, pitch);
                }
                // Handled as the behaviour emits it (GameLevel::emit).
                LevelEvent::DropResources { pos, state } => {
                    self.with_level(|_, level, _| level.drop_resources_now(pos, state));
                }
                LevelEvent::FallingBlock {
                    pos,
                    state,
                    hurt_per_distance,
                } => {
                    let mut damage = horizon_client::falling_block::Damage::falling(
                        &self.res.facts().get(state).name,
                    );
                    if let Some(per_distance) = hurt_per_distance {
                        damage.hurt_entities = true;
                        damage.per_distance = per_distance;
                    }
                    let p = Vec3::new(
                        f64::from(pos.x) + 0.5,
                        f64::from(pos.y),
                        f64::from(pos.z) + 0.5,
                    );
                    self.next_entity += 1;
                    self.falling.push(FallingBlock {
                        rotation: [0.0, 0.0],
                        join: Default::default(),
                        id: self.next_entity,
                        state,
                        start_pos: pos,
                        position: p,
                        previous: p,
                        velocity: Vec3::ZERO,
                        age: 0,
                        no_gravity: false,
                        portal: Default::default(),
                        drop_item: true,
                        fall_distance: 0.0,
                        stuck: Vec3::ZERO,
                        damage,
                        saved: None,
                    });
                }
                LevelEvent::Player { pos, effect, .. } => match effect {
                    PlayerEffect::OpenMenu => self.open_block_effect_menu([pos.x, pos.y, pos.z]),
                    PlayerEffect::Sleep if self.beds_destroy_on_use() => {
                        self.destroy_bed_on_use([pos.x, pos.y, pos.z])?
                    }
                    PlayerEffect::Sleep => self.try_sleep([pos.x, pos.y, pos.z]),
                    PlayerEffect::GiveItem(item) => {
                        if let Some(rest) = self.add_stack_to_inventory(ItemStack {
                            item,
                            count: 1,
                            damage: 0,
                            extra: None,
                        }) {
                            self.pop_resource([pos.x, pos.y, pos.z], rest);
                        }
                    }
                    // Popped as the behaviour emits it (GameLevel::emit).
                    PlayerEffect::DropItem(_) => {}
                    PlayerEffect::EatCake => {
                        // CakeBlock.eat
                        self.award_custom_stat("eat_cake_slice", 1);
                        if let Some(p) = self.player.as_mut() {
                            let food = &mut p.health.food;
                            food.food = (food.food + 2).min(20);
                            food.saturation = (food.saturation + 0.4).min(food.food as f32);
                        }
                    }
                    PlayerEffect::RespawnAnchor => {
                        self.respawn_anchor_used([pos.x, pos.y, pos.z])?
                    }
                    PlayerEffect::InteractLoot(_) => {}
                },
                LevelEvent::PrimeTnt { pos, rot } => self.prime_tnt_event(pos, rot),
                LevelEvent::BlockEvent {
                    pos,
                    state,
                    kind: "dispense",
                } => self.dispense_from(pos, state)?,
                LevelEvent::BlockEvent {
                    pos, kind: "golem", ..
                } => self.check_golem_spawn([pos.x, pos.y, pos.z])?,
                LevelEvent::BlockEvent {
                    pos,
                    kind: "command_block",
                    ..
                } => self.command_block_tick(pos),
                LevelEvent::BlockEvent {
                    pos,
                    kind: "structure_block",
                    ..
                } => self.structure_block_neighbor_changed((pos.x, pos.y, pos.z)),
                LevelEvent::BlockEvent {
                    pos,
                    kind: "test_block",
                    ..
                } => self.test_block_neighbor_changed((pos.x, pos.y, pos.z)),
                LevelEvent::BlockEvent {
                    pos,
                    kind: "test_block_tick",
                    ..
                } => self.reset_test_block((pos.x, pos.y, pos.z)),
                LevelEvent::BlockEvent {
                    pos,
                    kind: "brushable",
                    ..
                } => self.brushable_tick([pos.x, pos.y, pos.z]),
                LevelEvent::BlockEvent {
                    pos, kind: "hatch", ..
                } => {
                    // FrogspawnBlock.hatchFrogspawn: 2-5 tadpoles.
                    let at = [
                        f64::from(pos.x) + 0.5,
                        f64::from(pos.y) + 0.5,
                        f64::from(pos.z) + 0.5,
                    ];
                    self.audio
                        .play("minecraft:block.frogspawn.hatch", Some(at), 1.0, 1.0);
                    let count = 2 + self.world.random.next_int_bound(4).unwrap_or(0);
                    for _ in 0..count {
                        // getRandomTadpolePositionOffset: 0.2..0.8 inside the block.
                        let r = &mut self.world.random;
                        let off = |r: &mut horizon_core::random::LegacyRandom| {
                            f64::from(r.next_float()) * 0.6 + 0.2
                        };
                        let (dx, dz) = (off(r), off(r));
                        let spawn = [
                            f64::from(pos.x) + dx,
                            f64::from(pos.y) - 0.5,
                            f64::from(pos.z) + dz,
                        ];
                        let _ = self.spawn_mob(
                            "minecraft:tadpole",
                            spawn,
                            true,
                            creatures::SpawnSite::Plain,
                        );
                    }
                }
                LevelEvent::BlockEvent {
                    pos,
                    kind: "hatch_sniffer",
                    ..
                } => self.hatch_sniffer([pos.x, pos.y, pos.z]),
                LevelEvent::BlockEvent {
                    pos,
                    state,
                    kind: "hatch_turtles",
                } => self.hatch_turtles([pos.x, pos.y, pos.z], state),
                LevelEvent::BlockEvent {
                    pos,
                    kind: "portal_spawn",
                    ..
                } => self.portal_spawn([pos.x, pos.y, pos.z]),
                LevelEvent::BlockEvent {
                    pos,
                    state,
                    kind: "spawn_ghastling",
                } => self.spawn_ghastling([pos.x, pos.y, pos.z], state),
                LevelEvent::GameEvent { pos, event, state } => {
                    self.sculk_level_event(pos, event, state)
                }
                LevelEvent::BlockEvent {
                    pos,
                    state,
                    kind: "shrieker_respond",
                } => self.shrieker_block_event(pos, state),
                LevelEvent::BlockEntityEvent { pos, a, b, .. } => {
                    self.block_entity_event([pos.x, pos.y, pos.z], a, b)
                }
                // NoteBlock.triggerEvent: a NOTE particle above, coloured by
                // the note (`note / 24.0` in the x speed).
                LevelEvent::BlockEvent {
                    pos,
                    state,
                    kind: "note_particle",
                } => {
                    use horizon_client::particles::{ParticleOptions, ParticleType};
                    let note = self
                        .pack
                        .assets
                        .registry
                        .state(state)
                        .and_then(|s| s.properties.get("note")?.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    self.particles.add(
                        &ParticleOptions::simple(ParticleType::Note),
                        [
                            f64::from(pos.x) + 0.5,
                            f64::from(pos.y) + 1.2,
                            f64::from(pos.z) + 0.5,
                        ],
                        [note / 24.0, 0.0, 0.0],
                    );
                }
                // NoteBlock.getCustomSoundId: the skull above's note block sound,
                // played at volume 3 (vanilla's pitch for a head is 1).
                LevelEvent::BlockEvent {
                    pos,
                    kind: "note_custom_sound",
                    ..
                } => {
                    if let Some(BlockEntity::Skull(skull)) =
                        self.world.block_entities.get(&(pos.x, pos.y + 1, pos.z))
                        && let Some(sound) = skull.note_block_sound.as_deref()
                    {
                        let at = [
                            f64::from(pos.x) + 0.5,
                            f64::from(pos.y) + 0.5,
                            f64::from(pos.z) + 0.5,
                        ];
                        self.audio.play(sound, Some(at), 3.0, 1.0);
                    }
                }
                // `level.addParticle`: only the local player's own click draws it.
                LevelEvent::BlockEvent {
                    kind: "candle_smoke",
                    ..
                } if self.lan_acting.is_some() => {}
                LevelEvent::BlockEvent {
                    pos,
                    state,
                    kind: "candle_smoke",
                } => self.candle_smoke([pos.x, pos.y, pos.z], state),
                // The block at `pos` has just replaced `state`: Block.pushEntitiesUp.
                LevelEvent::BlockEvent {
                    pos,
                    state,
                    kind: "push_entities_up",
                } => self.push_entities_up(pos, state),
                LevelEvent::GrowTree { .. }
                | LevelEvent::PlaceFeature { .. }
                | LevelEvent::BlockEvent { .. } => {}
            }
        }
        Ok(())
    }
    /// Menus opened by PlayerEffect::OpenMenu.
    fn open_block_effect_menu(&mut self, pos: [i32; 3]) {
        if self.fake_blocks(mod_api::FakeEffect::Menu) {
            return;
        }
        let name = self
            .res
            .facts()
            .get(self.world.block(pos[0], pos[1], pos[2]))
            .name
            .clone();
        let key = (pos[0], pos[1], pos[2]);
        if self.chest_menu_blocked(key, &name) {
            // The use still succeeds, but no menu, stat or piglin anger.
            return;
        }
        if &*name == "minecraft:lectern" {
            self.open_lectern(pos);
            return;
        }
        if horizon_client::command_block::Mode::of_block(&name).is_some() {
            self.open_command_block(key);
            return;
        }
        // The game master blocks' screens open on the player's own client
        // (`BlockEntity.usedBy`); the server has nothing to open.
        if matches!(
            &*name,
            "minecraft:structure_block"
                | "minecraft:jigsaw"
                | "minecraft:test_block"
                | "minecraft:test_instance_block"
        ) {
            return;
        }
        // SignBlock.useWithoutItem: the text editor, not a container.
        let block = self.world.block(pos[0], pos[1], pos[2]);
        if self.res.facts().get(block).sign_form().is_some() {
            self.use_sign_empty_hand(pos);
            return;
        }
        if &*name == "minecraft:crafting_table" {
            self.craft_size = 3;
            self.craft_table = Some(pos);
        } else if &*name == "minecraft:ender_chest" {
            self.open_ender_chest(key);
            // EnderChestBlock.useWithoutItem
            self.award_custom_stat("open_enderchest", 1);
            self.anger_piglins(true);
            // EnderChestBlockEntity.startOpen: the first opener's CONTAINER_OPEN
            // (ContainerOpenersCounter.incrementOpeners), as for chests below.
            if self.container_opener_event(key, true) {
                self.post_block_game_event(
                    "minecraft:container_open",
                    [key.0, key.1, key.2],
                    Some(horizon_client::mobs::Who::Player),
                    None,
                );
                self.container_edge_sound(key, true);
            }
        } else if &*name == "minecraft:beacon" && self.open_station(pos, &name) {
            self.world.block_entities.get_or_insert_with(key, || {
                BlockEntity::for_block(&name).unwrap_or(BlockEntity::Chest(Vec::new()))
            });
        } else if storage::is_shulker_box(&name) && !self.shulker_can_open(pos) {
            // ShulkerBoxBlock.canOpen: the lid has no room; nothing opens.
        } else if let Some(entity) = BlockEntity::for_block(&name) {
            self.world.block_entities.get_or_insert_with(key, || entity);
            horizon_client::spans::time("open_loot", || self.unpack_opened_loot(key, &name));
            let centre = [
                f64::from(key.0) + 0.5,
                f64::from(key.1) + 0.5,
                f64::from(key.2) + 0.5,
            ];
            horizon_client::spans::time("open_maps", || {
                self.resolve_container_maps(key, centre);
            });
            self.container = Some(key);
            let first = self.container_opener_event(key, true);
            if first {
                self.post_block_game_event(
                    "minecraft:container_open",
                    [key.0, key.1, key.2],
                    Some(horizon_client::mobs::Who::Player),
                    None,
                );
            }
            let stat = match &*name {
                "minecraft:chest" => "open_chest",
                "minecraft:trapped_chest" => "trigger_trapped_chest",
                "minecraft:barrel" => "open_barrel",
                "minecraft:hopper" => "inspect_hopper",
                "minecraft:dropper" => "inspect_dropper",
                "minecraft:dispenser" => "inspect_dispenser",
                n if storage::is_shulker_box(n) => "open_shulker_box",
                _ => "",
            };
            if !stat.is_empty() {
                self.award_custom_stat(stat, 1);
            }
            // Opening a piglin-guarded container angers the piglins who see it.
            if self.guarded_by_piglins(&name) {
                self.anger_piglins(true);
            }
            if first {
                self.container_edge_sound(key, true);
            }
        } else if self.open_mod_menu(key) {
            // A mod block's container (`native_block_entity_container`).
            self.container = Some(key);
        } else if self.open_station(pos, &name) {
            // Workstation menus: game/blocks/stations.rs.
        } else {
            self.message = Some((
                format!(
                    "{} is not available yet",
                    name.trim_start_matches("minecraft:").replace('_', " ")
                ),
                Instant::now(),
            ));
            return;
        }
        self.set_screen(Screen::Inventory);
        self.capture(false);
        self.right = false;
    }
    /// FallingBlockEntity.tick: fall with 0.04 gravity and 0.98 drag, then land.
    /// One falling block's `tick`: places or drops its block in its own tick,
    /// so a block falling onto one that just landed lands on top of it. `i` is
    /// its place in the list; the caller has checked it is ticking.
    /// `entities` are the boxes that stop its move.
    fn tick_one_falling(&mut self, i: usize, entities: &[Aabb]) -> Result<()> {
        let entity_drops = self
            .ui
            .rules
            .bool_at(horizon_client::gamerules::vanilla::ENTITY_DROPS);
        let mut f = self.falling.remove(i);
        let original = f.clone();
        match self.fall_step(&mut f, entity_drops, entities) {
            Ok(FallStep::Falling) => self.falling.insert(i.min(self.falling.len()), f),
            Ok(FallStep::Gone) => {}
            Ok(FallStep::Dropped(at)) => self.drop_falling_block(f.state, at),
            Ok(FallStep::Landed(pos)) => self.finish_falling_block(f, pos),
            Err(error) => {
                self.falling.insert(i.min(self.falling.len()), original);
                return Err(error);
            }
        }
        Ok(())
    }
    /// One `FallingBlockEntity.tick` move of `f`.
    fn fall_step(
        &mut self,
        f: &mut FallingBlock,
        entity_drops: bool,
        entities: &[Aabb],
    ) -> Result<FallStep> {
        f.previous = f.position;
        f.age = f.age.wrapping_add(1);
        f.velocity.y -= horizon_client::entity_rules::EntityRules::get_gravity(f);
        let mut wanted = f.velocity;
        // Entity.move: a cobweb it was in scales this move, which then
        // starts from rest.
        if f.stuck.length_squared() > 1.0e-7 {
            wanted = wanted.multiply(f.stuck);
            f.stuck = Vec3::ZERO;
            f.velocity = Vec3::ZERO;
        }
        let scene = collisions::SceneCollisions {
            world: &self.world,
            binding: self.collision,
            subject: Some(collisions::PowderSnowSubject {
                fall_distance: f.fall_distance,
                falling_block: true,
                ..Default::default()
            }),
        };
        // Entity.isDescending: only sneaking entities descend through
        // scaffolding. Falling blocks do not sneak, so descending is always
        // false (Entity.isDescending returns isShiftKeyDown, fixes #7553).
        let context = CollisionContext::for_entity(
            CollisionEntity {
                identity: EntityIdentity(0),
                y: f.position.y,
                descending: false,
                main_hand: None,
            },
            false,
        );
        // On an error the caller keeps the block as it was before this tick.
        // Entity.collide: the world border's walls and the entities that
        // `canBeCollidedWith` it (a boat, a shulker) stop it too.
        let movement = collisions::EntityCollisions {
            blocks: scene,
            entities,
            border: Some((self.border.geometry(), [f.position.x, f.position.z])),
        }
        .resolve_with_context(
            horizon_client::falling_block::bounds(f.position),
            wanted,
            0.0,
            false,
            context,
        )?;
        let next = Vec3::new(
            f.position.x + movement.x,
            f.position.y + movement.y,
            f.position.z + movement.z,
        );
        let on_ground = wanted.y < 0.0 && movement.y != wanted.y;
        if next.y < f.position.y {
            f.fall_distance += f.position.y - next.y;
        }
        let from = f.position;
        f.position = next;
        // Entity.move stops it on the axes it collided on.
        if movement.x != wanted.x {
            f.velocity.x = 0.0;
        }
        if movement.y != wanted.y {
            f.velocity.y = 0.0;
        }
        if movement.z != wanted.z {
            f.velocity.z = 0.0;
        }
        if on_ground {
            self.falling_block_impact(f);
            f.fall_distance = 0.0;
        }
        // applyEffectsFromBlocks: WebBlock.entityInside makes it stuck
        // (Entity.makeStuckInBlock), BubbleColumnBlock.entityInside lifts or
        // drags it. Entity.move records the move unless it was blocked short
        // of a tiny one.
        let moved = movement.length_squared();
        let recorded = moved > 1.0e-7 || wanted.length_squared() - moved < 1.0e-7;
        let world = &self.world;
        let facts = self.res.facts();
        let (registry, collision) = (self.pack.assets.registry, self.collision);
        let mut tops = Vec::new();
        let stuck = horizon_client::falling_block::apply_effects_from_blocks(
            from,
            f.position,
            recorded.then_some(wanted),
            &mut f.velocity,
            &mut f.fall_distance,
            &mut tops,
            |p| {
                use horizon_client::falling_block::Inside;
                if &*facts.get(world.block(p.x, p.y, p.z)).name == "minecraft:cobweb" {
                    Inside::Cobweb
                } else if let Some(column) =
                    horizon_client::motion::bubble_column_at(world, registry, facts, collision, p)
                {
                    Inside::BubbleColumn(column)
                } else {
                    Inside::Nothing
                }
            },
        );
        if stuck {
            f.stuck = horizon_client::falling_block::web_multiplier();
        }
        for pos in tops {
            self.bubble_column_particles(pos);
        }
        self.entity_powder_snow_particles(
            from,
            f.position,
            horizon_client::falling_block::bounds(f.position),
            f.velocity,
            false,
        );
        // FallingBlockEntity.tick handles its portal after the move's block
        // effects, before placing/dropping on the floor under an End portal.
        let teleported = self.falling_through_portal(f);
        // FallingBlockEntity.forceTickAfterTeleportToDuplicate: crossing
        // either way through the End deliberately keeps the source's final
        // placement/drop branch, even though the source entity was removed.
        if let Some(target) = teleported
            && self.dim != Dim::End
            && target != Dim::End
        {
            return Ok(FallStep::Gone);
        }
        let y = next.y.floor() as i32;
        let outside = y <= self.world.bottom_y() || y >= self.world.top_y();
        if !on_ground && (f.age > 100 && outside || f.age > 600) {
            return Ok(if f.drop_item && entity_drops {
                FallStep::Dropped(next)
            } else {
                FallStep::Gone
            });
        }
        if on_ground {
            return Ok(FallStep::Landed([
                f.position.x.floor() as i32,
                f.position.y.floor() as i32,
                f.position.z.floor() as i32,
            ]));
        }
        f.velocity = Vec3::new(
            f.velocity.x * 0.98,
            f.velocity.y * 0.98,
            f.velocity.z * 0.98,
        );
        Ok(if teleported.is_some() {
            FallStep::Gone
        } else {
            FallStep::Falling
        })
    }
    /// `Entity.spawnAtLocation`: an item entity where the block was.
    fn drop_falling_block(&mut self, state: BlockStateId, at: Vec3) {
        // Entity.spawnAtLocation: an item entity where the block was.
        let item = self.res.facts().get(state).item.clone();
        if &*item != "minecraft:air" {
            // new ItemEntity(level, x, y, z, stack): the motion is from the
            // item's own random, not the level's.
            let v = dispenser::new_item_motion();
            let stack = ItemStack {
                item,
                count: 1,
                damage: 0,
                extra: None,
            };
            self.spawn_item(stack, [at.x, at.y, at.z], v, 10);
        }
    }
    /// `DecoratedPotBlock.playerWillDestroy`: a pot mined with an item in
    /// `breaks_decorated_pots`, unless an enchantment prevents shattering, is
    /// cracked first (flags 260), so it shatters into its sherds.
    fn crack_mined_pot(&mut self, pos: [i32; 3], state: BlockStateId) -> BlockStateId {
        let registry = &self.pack.assets.registry;
        let Some(mut pot) = registry
            .state(state)
            .filter(|s| s.name == "minecraft:decorated_pot")
            .cloned()
        else {
            return state;
        };
        let Some(held) = self.held() else {
            return state;
        };
        let breaks = self
            .res
            .items()
            .item_tags
            .get("minecraft:breaks_decorated_pots")
            .is_some_and(|t| t.contains(&*held.item));
        let enchantments = &self.stations().enchantments;
        // `EnchantmentHelper.hasTag`, which NeoForge asks through the mods.
        let kept = held
            .all_enchantments()
            .iter()
            .any(|(e, _)| enchantments.in_tag("minecraft:prevents_decorated_pot_shattering", e));
        if !breaks || kept {
            return state;
        }
        pot.properties.insert("cracked".into(), "true".into());
        let Ok(cracked) = registry.resolve(&pot) else {
            return state;
        };
        let at = BlockPos::new(pos[0], pos[1], pos[2]);
        self.with_level(|b, level, ticks| b.set_block(level, ticks, at, cracked, 260));
        cracked
    }
    /// `ItemStack.mineBlock` for the held stack once a survival player has
    /// mined the block `name`: an item with a `tool` component counts as used
    /// and loses its `damage_per_block` on a block that doesn't break
    /// instantly (`ShearsItem`: on any block but fire).
    fn mine_block_with_held(&mut self, name: &str, hardness: f32) {
        let Some(stack) = self.inventory[self.selected].clone() else {
            return;
        };
        let Some((per_block, max)) = self
            .res
            .items()
            .get(&stack.item)
            .and_then(|i| Some((i.tool_damage_per_block?, i.max_damage)))
        else {
            return;
        };
        let wears = if &*stack.item == "minecraft:shears" {
            !self.items().block_in_tag("minecraft:fire", name)
        } else {
            hardness != 0.0
        };
        if wears && per_block > 0 && max > 0 {
            self.damage_held(per_block);
        }
        self.award_stat("used", &stack.item, 1);
    }
    fn break_block(&mut self, pos: [i32; 3]) -> Result<()> {
        self.with_player_actor(|game| game.break_block_as_player(pos))
    }
    fn break_block_as_player(&mut self, pos: [i32; 3]) -> Result<()> {
        // DebugStickItem.canDestroyBlock: a left click selects a property and
        // never destroys the block.
        if !self.debug_stick_can_destroy(pos)? {
            return Ok(());
        }
        // ServerPlayerGameMode.destroyBlock.
        if self.block_action_restricted(pos) {
            return Ok(());
        }
        if !self
            .res
            .items()
            .can_destroy_block(self.held(), self.creative)
        {
            return Ok(());
        }
        // ServerPlayerGameMode.destroyBlock (and MultiPlayerGameMode.destroyBlock
        // on the client): a GameMasterBlock stays when the player cannot use
        // game master blocks (Player.canUseGameMasterBlocks).
        let state = self.world.block(pos[0], pos[1], pos[2]);
        if !self.can_use_game_master_blocks()
            && is_game_master_block(&self.res.facts().get(state).name)
        {
            return Ok(());
        }
        // Native mods, once the game allowed the break: a canceled one keeps
        // the block, and a guest's acknowledgement puts it back in its world.
        let mut canceled = false;
        let mut exp_to_drop = None;
        self.native_break_block(pos, state, &mut canceled, &mut exp_to_drop);
        if canceled {
            return Ok(());
        }
        self.pad_rumble(horizon_client::gamepad::Rumble::BlockBroken);
        let broken = state;
        let facts = self.res.facts().get(state).clone();
        let state = self.crack_mined_pot(pos, state);
        // Block.playerWillDestroy: mining their gold angers all nearby piglins.
        if self.guarded_by_piglins(&facts.name) {
            self.anger_piglins(false);
        }
        // The block entity as it was, for what the drops copy from it.
        let key = (pos[0], pos[1], pos[2]);
        let loot_entity = if self.creative {
            None
        } else {
            self.loot_block_entity(pos, &facts.name)
        };
        if let Some(entity) = self.world.block_entities.get(&key).cloned() {
            let name = self.world.block_names.get(&key).cloned();
            self.pending.record(pos, &entity, &facts.name, name);
        } else if let Some(tag) = spill::saved_entity(&self.world, pos) {
            self.pending.record_saved(pos, &tag);
        }
        self.creaking_heart_broken(pos, true);
        let hive = match self.world.block_entities.get(&key) {
            Some(BlockEntity::Beehive(h)) => self.hive_state(pos).map(|s| ((**h).clone(), s)),
            _ => None,
        };
        // playerWillDestroy and Level.removeBlock through the vanilla behaviour
        // library: the block entity's side effects (containers spill), neighbour
        // and shape updates, and the destroy event (particles, sound).
        let creative = self.creative;
        // TripWireBlock.playerWillDestroy: shears disarm the string first (flags
        // 260), so taking it down does not trip the hook.
        if &*facts.name == "minecraft:tripwire"
            && self.held().is_some_and(|s| &*s.item == "minecraft:shears")
            && let Some(disarmed) = self.with_props(state, &[("disarmed", "true")])
        {
            let at = BlockPos::new(pos[0], pos[1], pos[2]);
            self.with_level(|b, level, ticks| b.set_block(level, ticks, at, disarmed, 260));
            self.post_block_game_event("minecraft:shear", pos, self.sculk_actor(), None);
        }
        if creative && storage::is_shulker_box(&facts.name) {
            self.drop_creative_shulker(pos);
        }
        // BeehiveBlock.playerWillDestroy: a creative break still drops the hive
        // with its bees and honey level.
        if creative
            && self
                .ui
                .rules
                .bool_at(horizon_client::gamerules::vanilla::BLOCK_DROPS)
            && let Some((h, (honey, _))) = &hive
            && (!h.bees.is_empty() || honey.is_some_and(|l| l > 0))
            && let Some(stack) = self.picked_stack(pos, true)
        {
            let at = pos.map(f64::from);
            self.spawn_item(stack, at, dispenser::new_item_motion(), 10);
        }
        let mut correct_tool =
            !facts.requires_tool || self.res.items().correct_for_drops(self.held(), &facts.name);
        if !creative
            && self
                .res
                .behaviours()
                .data(state)
                .is(horizon_world::level::classes::Class::Door)
            && mod_api::__private::listened(mod_api::HookId::PlayerHarvestCheck)
        {
            self.native_player_harvest_check(
                self.own_entity_id() as u32,
                pos,
                state.0,
                &mut correct_tool,
            );
        }
        let source = self
            .lan_acting_uuid
            .map(horizon_world::level::LevelEventPlayer);
        let removed = self.with_level(|b, level, ticks| {
            b.player_destroy_block(
                level,
                ticks,
                BlockPos::new(pos[0], pos[1], pos[2]),
                creative,
                correct_tool,
                source,
            )
        });
        if !creative {
            let held = self.held().cloned();
            let mut harvest = !facts.requires_tool
                || self
                    .res
                    .items()
                    .correct_for_drops(held.as_ref(), &facts.name);
            if mod_api::__private::listened(mod_api::HookId::PlayerHarvestCheck) {
                self.native_player_harvest_check(
                    self.own_entity_id() as u32,
                    pos,
                    state.0,
                    &mut harvest,
                );
            }
            if removed {
                self.mine_block_with_held(&facts.name, facts.hardness);
            }
            if removed && harvest {
                // Block.playerDestroy: the mined statistic, the exhaustion and the drops.
                self.award_stat("mined", &facts.name, 1);
                if let Some(p) = self.player.as_mut() {
                    p.cause_food_exhaustion(0.005);
                }
                let stacks = self.block_loot_at(pos, state, held.as_ref(), loot_entity);
                let silk = held
                    .as_ref()
                    .is_some_and(|h| h.enchantment_level("minecraft:silk_touch") > 0);
                // Block.dropResources: the item entities, then spawnAfterBreak's
                // experience, with the mods' say between the two.
                let breaker = Some(self.native_acting_id());
                let dropped = self.with_level(|_, level, _| {
                    level.block_dropped(
                        BlockPos::new(pos[0], pos[1], pos[2]),
                        state,
                        breaker,
                        held.as_ref(),
                        stacks,
                        exp_to_drop,
                    )
                });
                let canceled = dropped.canceled;
                self.spawn_block_drops(pos, dropped);
                // BeehiveBlock.playerDestroy: silk touch keeps the bees in, so
                // the trigger counts them; otherwise they are out already.
                if let Some((hive, (honey, facing))) = hive {
                    let bees = if silk { hive.bees.len() as i64 } else { 0 };
                    if !silk {
                        self.hive_broken_by_player(pos, hive, honey, facing);
                    }
                    self.trigger_bee_nest_destroyed(broken, held.as_ref(), bees);
                }
                // InfestedBlock.spawnAfterBreak: its silverfish comes out unless the
                // tool has silk touch (#prevents_infested_spawns) or blocks don't drop.
                if facts.name.starts_with("minecraft:infested_")
                    && !silk
                    && !canceled
                    && self
                        .ui
                        .rules
                        .bool_at(horizon_client::gamerules::vanilla::BLOCK_DROPS)
                {
                    let at = [
                        f64::from(pos[0]) + 0.5,
                        f64::from(pos[1]),
                        f64::from(pos[2]) + 0.5,
                    ];
                    if let Some(id) = self.spawn_mob(
                        "minecraft:silverfish",
                        at,
                        false,
                        creatures::SpawnSite::Plain,
                    ) {
                        self.mob_poof(id);
                    }
                }
                // TurtleEggBlock.playerDestroy: after the drops, one egg of a
                // mined stack is taken and the rest stay.
                self.with_level(|b, level, ticks| {
                    b.turtle_egg_destroyed_by_player(
                        level,
                        ticks,
                        BlockPos::new(pos[0], pos[1], pos[2]),
                        broken,
                    );
                });
            }
            // IceBlock.playerDestroy: the ice melts back into water (the
            // #prevents_ice_melting tool tag holds only silk touch).
            let prevents_melting = held
                .as_ref()
                .is_some_and(|h| h.enchantment_level("minecraft:silk_touch") > 0);
            self.with_level(|b, level, ticks| {
                b.ice_destroyed_by_player(
                    level,
                    ticks,
                    BlockPos::new(pos[0], pos[1], pos[2]),
                    broken,
                    prevents_melting,
                );
            });
        }
        self.pending.clear(pos);
        self.process_level_events()?;
        Ok(())
    }
    /// The hand whose use ended the click, if any.
    fn use_item(&mut self) -> Result<Option<Hand>> {
        self.with_player_actor(|game| game.use_item_as_player())
    }
    /// `Minecraft.startUseItem`: the main hand, then the off hand.
    fn use_item_as_player(&mut self) -> Result<Option<Hand>> {
        // Spectators only look (MultiPlayerGameMode.useItemOn/useItem pass),
        // but on a server a spectator's use on a block opens the block's
        // menu (ServerPlayerGameMode.useItemOn).
        if self.spectator() {
            if let Some(GuestUse::OnBlock(hand)) = self.guest_use
                && let Some(hit) = self.pick()
            {
                self.with_hand(hand, |game| {
                    let held = game.held().cloned();
                    if matches!(
                        game.right_click_block(&hit, hand, held.as_ref()),
                        native_hooks::BlockClickAnswer::Go { .. }
                    ) {
                        game.spectator_open_menu(hit.pos);
                    }
                });
            }
            return Ok(None);
        }
        // A carrot or warped fungus on a stick boosts the ridden pig or strider.
        // A guest's use packets name one hand and touch no entity: the client
        // sends one per hand and part of its click, and Interact for entities.
        let guest = self.guest_use;
        if let Some(GuestUse::Entity(hand, target)) = guest {
            return Ok(self
                .with_hand(hand, |g| g.interact_entity(Some(target)))?
                .then_some(hand));
        }
        let on_block = matches!(guest, Some(GuestUse::OnBlock(_)));
        if !on_block
            && self.riding.is_some()
            && let Some(item) = self.held().map(|s| s.item.clone())
            && self.use_steering_stick(&item)
        {
            return Ok(Some(Hand::Main));
        }
        if guest.is_none() && self.interact_entity(None)? {
            return Ok(Some(Hand::Main));
        }
        // An EntityHitResult may fall through to Item.use, never to useItemOn.
        let block_hit = match guest {
            Some(GuestUse::OnBlock(_)) => self.pick(),
            Some(GuestUse::Item(_)) => None,
            _ if self.entity_targeted() => None,
            _ => self.pick(),
        };
        let hands = match guest {
            Some(GuestUse::OnBlock(hand) | GuestUse::Item(hand) | GuestUse::Entity(hand, _)) => {
                vec![hand]
            }
            None => Hand::BOTH.to_vec(),
        };
        let entity = guest.is_none() && self.entity_targeted();
        for hand in hands {
            // The off hand's turn starts on the entity too (Minecraft.startUseItem).
            if entity && hand == Hand::Off && self.with_hand(hand, |g| g.interact_entity(None))? {
                return Ok(Some(hand));
            }
            if self.with_hand(hand, |game| game.use_hand(block_hit.as_ref()))? {
                return Ok(Some(hand));
            }
        }
        Ok(None)
    }
    /// The index in `list` of the entity an interact packet's `target` names,
    /// by its server id (`Level.getEntity(id)`), or None for an id this level
    /// does not hold: `handleInteract` finds nothing then either.
    fn named_entity<T>(list: &[T], target: i32, id: impl Fn(&T) -> u32) -> Option<usize> {
        list.iter().position(|e| id(e) as i32 == target)
    }
    /// `Player.interactOn` with the acting hand: whether it took the click. A
    /// guest's interact packet names its entity (`target`), which the helpers
    /// use as it is; the local player picks from the crosshair instead, the
    /// way `Minecraft.hitResult` does. `native_entity_interact_specific` and
    /// `native_interact_entity` hear it first; a claim swings the hand and
    /// ends the click.
    fn interact_entity(&mut self, target: Option<InteractTarget>) -> Result<bool> {
        // Native mods, where Player.interactOn starts: before the entity's
        // own interact and the item's interactLivingEntity. The entity is the
        // one the packet named, or the one the crosshair picked.
        let (named, distance) = match target {
            Some(t) => (Some(t.id), None),
            None => match self.pick_entity_hit_at() {
                Some((distance, id)) => (id, Some(distance)),
                None => (None, None),
            },
        };
        if let Some(id) = named
            && let Some(entity) = self.level_entity(id)
        {
            // The point clicked, measured from the entity's position: a
            // guest's packet carries it, and the local player's is where the
            // crosshair's ray met the entity (`EntityHitResult.getLocation()`).
            let location = match target {
                Some(t) => Some(t.location),
                None => distance
                    .zip(self.aim())
                    .zip(self.target_position(entity))
                    .map(|((d, (eye, dir)), at)| {
                        std::array::from_fn(|k| eye[k] + dir[k] * d - at[k])
                    }),
            };
            let kind = self.target_type(entity);
            let stack = self.held().cloned();
            let mut claimed = false;
            if let Some(location) = location {
                self.native_entity_interact_specific(
                    id,
                    &kind,
                    stack.as_ref(),
                    location,
                    &mut claimed,
                );
            }
            if !claimed {
                self.native_entity_interact(id, &kind, stack.as_ref(), &mut claimed);
            }
            if claimed {
                self.swing_arm();
                return Ok(true);
            }
        }
        Ok(self.interact_frame(target)
            || self.interact_armor_stand(target)
            || self.interact_cushion(target)
            || self.interact_boat(target)
            || self.interact_cart(target)
            || self.interact_mob(target)?
            || self.item_interact_mob(target))
    }
    /// The `ItemStack.interactLivingEntity` fallback in `Player.interactOn`,
    /// currently for native Mob entries after the entity's own action passed.
    fn item_interact_mob(&mut self, target: Option<InteractTarget>) -> bool {
        if self.spectator() {
            return false;
        }
        let Some(index) = self.mob_interaction_target(target) else {
            return false;
        };
        let mob = &self.mobs[index];
        if mob.removed || !mob.alive() {
            return false;
        }
        let (entity, kind) = (
            mob.id as i32,
            horizon_client::command::args::normalize(&mob.kind.id),
        );
        let Some(stack) = self.held().filter(|stack| stack.count > 0).cloned() else {
            return false;
        };
        let mut swing = None;
        self.native_item_interact_mob(entity, &kind, &stack, &mut swing);
        if swing == Some(true) {
            self.swing_arm();
        }
        swing.is_some()
    }
    /// `ItemStack.applyAfterUseComponentSideEffects`: a used stack with a
    /// `use_cooldown` component starts its entry (`UseCooldown.apply`). A
    /// server's guest also gets `ClientboundCooldownPacket`
    /// (`ServerItemCooldowns.onCooldownStarted`), queued for their turn's end.
    pub(super) fn apply_use_cooldown(&mut self, stack: &ItemStack) {
        let default = self
            .res
            .items()
            .get(&stack.item)
            .and_then(|i| i.use_cooldown.clone());
        let Some(component) = horizon_client::cooldowns::component(stack, default.as_ref()) else {
            return;
        };
        let group = horizon_client::cooldowns::group(stack, default.as_ref());
        let mut cancel = false;
        if let (Some(player), Some(name)) = (self.lan_acting_uuid, self.lan_acting_name.clone()) {
            self.native_player_item_cooldown(
                player,
                &name,
                &stack.item,
                component.ticks(),
                &mut cancel,
            );
        }
        if !cancel {
            self.add_cooldown(group, component.ticks());
        } else if self.lan_acting.is_some() {
            // The guest's client started the cooldown itself; 0 ends it there.
            self.guest_cooldowns.push((group, 0));
        }
    }
    /// `ItemCooldowns.addCooldown` (a guest also gets the packet).
    pub(super) fn add_cooldown(&mut self, group: Arc<str>, ticks: i32) {
        self.cooldowns.add(Arc::clone(&group), ticks);
        if self.lan_acting.is_some() {
            self.guest_cooldowns.push((group, ticks));
        }
    }
    /// One hand's turn in `startUseItem`: `useItemOn` the block aimed at,
    /// then `useItem`. Whether the click ends here (it acted or failed).
    fn use_hand(&mut self, block_hit: Option<&Hit>) -> Result<bool> {
        self.use_failed = false;
        // The click's target as it was decided, before the adventure gate
        // below may drop the block for the item's own use.
        let aimed_at_block = block_hit.is_some();
        // isSecondaryUseActive with something in either hand keeps blocks
        // from reacting.
        let sneaking = self.keys.held(Mapping::Sneak);
        let hands_full = self.stack_in(self.selected).is_some() || self.offhand.is_some();
        let mut anchor_passed = false;
        // Native mods, where ServerPlayerGameMode.useItemOn starts
        // (RightClickBlock): a cancel answers for the block and the item, and
        // its PASS goes on to Item.use, which a guest's UseItem asks for.
        // A slow tick's line names the phase of a use that took it (#8669): the
        // mods' hook, the block's own use, then the item's.
        use mod_api::{NativeInteractionResult as Answer, NativeTriState};
        let on_block = matches!(self.guest_use, Some(GuestUse::OnBlock(_)));
        let mut block_hit = block_hit;
        let (mut use_block, mut use_item) = (NativeTriState::Default, NativeTriState::Default);
        let block_phase = horizon_client::spans::guard("use_block");
        if let Some(hit) = block_hit {
            let held = self.held().cloned();
            let answer = horizon_client::spans::time("use_mods", || {
                self.right_click_block(hit, self.hand, held.as_ref())
            });
            match answer {
                native_hooks::BlockClickAnswer::Go {
                    use_block: block,
                    use_item: item,
                } => (use_block, use_item) = (block, item),
                native_hooks::BlockClickAnswer::Canceled(answer)
                    if answer.consumes_action() || answer == Answer::Fail =>
                {
                    if matches!(answer, Answer::Success | Answer::SuccessServer) {
                        self.swing_arm();
                    }
                    self.use_failed = answer == Answer::Fail;
                    return Ok(true);
                }
                native_hooks::BlockClickAnswer::Canceled(_) => {
                    (use_block, use_item) = (NativeTriState::False, NativeTriState::False);
                }
            }
        }
        // ServerPlayerGameMode.useItemOn: the held item's onItemUseFirst, when
        // RightClickBlock didn't deny the item, ahead of the sneak gate and
        // every block behaviour. Its answer ends the click for this hand.
        if use_item.or(true)
            && let Some(hit) = block_hit
            && let Some(held) = self.held().cloned()
            && let Some(answer) = self.item_use_first(hit, self.hand, &held)
        {
            if matches!(answer, Answer::Success | Answer::SuccessServer) {
                self.swing_arm();
            }
            self.use_failed = answer == Answer::Fail;
            return Ok(answer.consumes_action() || answer == Answer::Fail);
        }
        // ServerPlayerGameMode.useItemOn: after the spectator and sneak gates,
        // BlockState.useItemOn runs before every block-specific behaviour.
        if use_block.or(!(sneaking && hands_full))
            && let Some(hit) = block_hit
        {
            let held = self.held().cloned();
            let state = self.world.block(hit.pos[0], hit.pos[1], hit.pos[2]);
            let mut block_answer =
                self.use_item_on_block(hit, self.hand, held.as_ref(), mod_api::UsePhase::Block);
            if block_answer.is_none()
                && mod_api::__private::has_part_hook(mod_api::HookId::UseItemOn)
                && mod_api::__private::parts(mod_api::HookId::UseItemOn, "use_item_on")
                    .for_hook(mod_api::HookId::UseItemOn)
                    .is_some_and(|parts| !parts.block_state(state).is_empty())
            {
                let click = self.block_click(hit, self.hand, held.as_ref());
                self.native_use_item_on(state, &click, &mut block_answer);
            }
            let mut chains_sign = false;
            if block_answer.is_none() {
                if self.use_on_cauldron(hit.pos)? {
                    self.swing_arm();
                    return Ok(true);
                }
                if let Some(placed) = self.use_on_campfire(hit.pos)? {
                    if placed {
                        self.swing_arm();
                    }
                    return Ok(true);
                }
                if self.use_lectern(hit.pos, held.as_ref()) {
                    self.swing_arm();
                    return Ok(true);
                }
                if self.use_sign_applicator(hit.pos, held.as_ref()) {
                    self.swing_arm();
                    return Ok(true);
                }
                if self.use_golem_statue(hit.pos, held.as_ref()) {
                    self.swing_arm();
                    return Ok(true);
                }
                if let Some(swings) = self.use_shelf(hit) {
                    let used = held.unwrap_or_else(|| ItemStack::new("minecraft:air", 0));
                    self.trigger_block("minecraft:item_used_on_block", hit.pos, &used);
                    if swings {
                        self.swing_arm();
                    }
                    return Ok(true);
                }
                if self.use_composter(hit.pos, held.as_ref()) {
                    self.swing_arm();
                    return Ok(true);
                }
                if self.use_vault(hit.pos, held.as_ref())? {
                    self.swing_arm();
                    return Ok(true);
                }
                if self.use_pumpkin(hit.pos, hit.face, held.as_ref())? {
                    self.swing_arm();
                    return Ok(true);
                }
                if self.use_beehive(hit.pos, held.as_ref())? {
                    self.swing_arm();
                    return Ok(true);
                }
                if self.use_campfire_douse(hit, held.as_ref())? {
                    self.swing_arm();
                    return Ok(true);
                }
                block_answer = self.use_redstone_ore(hit.pos, held.as_ref())?;
                match self.use_respawn_anchor(hit.pos, held.as_ref())? {
                    respawn_anchor::AnchorUse::Charged => {
                        self.swing_arm();
                        return Ok(true);
                    }
                    // The main hand passes; its item is used, then the off hand's glowstone.
                    respawn_anchor::AnchorUse::Pass => anchor_passed = true,
                    respawn_anchor::AnchorUse::Other => {}
                }
                // CeilingHangingSignBlock.useItemOn: a hanging sign aimed at the
                // underside passes, so the next sign chains below instead of
                // opening the editor, unless the side has click commands to run
                // (`shouldTryToChainAnotherHangingSign`).
                chains_sign = hit.face == [0, -1, 0]
                // `instanceof HangingSignItem`, independent of the item's name.
                && held.as_ref().is_some_and(|s| {
                    self.res.behaviours().item_binding(&s.item)
                        .is_some_and(|i| i.kind == horizon_world::level::ItemKind::HangingSign)
                })
                && self
                    .res
                    .facts()
                    .get(self.world.block(hit.pos[0], hit.pos[1], hit.pos[2]))
                    .has_class("CeilingHangingSignBlock")
                && matches!(
                    self.world
                        .block_entities
                        .get(&(hit.pos[0], hit.pos[1], hit.pos[2])),
                    Some(BlockEntity::Sign(_))
                )
                && !self.sign_can_execute_click_commands((hit.pos[0], hit.pos[1], hit.pos[2]));
            }
            if block_answer.is_none() {
                use horizon_world::level::UseBranch::ItemOn;
                block_answer = self
                    .use_jukebox(hit.pos, state, held.as_ref(), ItemOn)
                    .or_else(|| self.use_decorated_pot(hit.pos, state, held.as_ref(), ItemOn))
                    .or_else(|| self.use_chiseled_bookshelf(hit, state, held.as_ref(), ItemOn));
            }
            // TRY_WITH_EMPTY_HAND: only the main hand's turn uses the block
            // itself (useWithoutItem). The block's item behaviour hears both hands.
            if !anchor_passed && !chains_sign && (self.hand == Hand::Main || block_answer.is_some())
            {
                let face = match hit.face {
                    [0, 1, 0] => Direction::Up,
                    [0, -1, 0] => Direction::Down,
                    [0, 0, -1] => Direction::North,
                    [0, 0, 1] => Direction::South,
                    [-1, 0, 0] => Direction::West,
                    _ => Direction::East,
                };
                let player = self.use_context();
                let block_hit = horizon_core::geometry::BlockHit {
                    block: BlockPos::new(hit.pos[0], hit.pos[1], hit.pos[2]),
                    direction: face,
                    location: Vec3::new(hit.point[0], hit.point[1], hit.point[2]),
                    inside: hit.inside,
                };
                let item = held.as_ref().map(|s| s.item.to_string());
                let adds_candle = self
                    .with_level(|b, _, _| b.cake_candle_state(state, item.as_deref()).is_some());
                let pots_flower = self
                    .with_level(|b, _, _| b.potted_flower_state(state, item.as_deref()).is_some());
                use horizon_world::level::{InteractionResult, UseBranch};
                let item_result = match block_answer {
                    Some(answer) => native_hooks::block_use_result(answer),
                    None => self.with_level(|b, level, ticks| {
                        b.block_use_item_on(level, ticks, state, block_hit, player, item.as_deref())
                    }),
                };
                let mut empty_template_ran = false;
                let (result, branch) = if let Some(result) = item_result {
                    (result, UseBranch::ItemOn)
                } else if self.hand == Hand::Main {
                    let mut empty_answer = None;
                    if !mod_api::__private::mods().is_empty() {
                        let empty_held = self.held().cloned();
                        let click = self.block_click(hit, self.hand, empty_held.as_ref());
                        self.native_use_without_item(state, &click, &mut empty_answer);
                    }
                    if empty_answer.is_none()
                        && self
                            .res
                            .behaviours()
                            .data(state)
                            .is(horizon_world::level::classes::Class::Fence)
                        && self.use_knot(hit.pos, false)
                    {
                        self.trigger_default_block_use(hit.pos);
                        return Ok(true);
                    }
                    let answer = empty_answer.or_else(|| {
                        self.use_jukebox(hit.pos, state, None, UseBranch::WithoutItem)
                            .or_else(|| {
                                self.use_decorated_pot(hit.pos, state, None, UseBranch::WithoutItem)
                            })
                            .or_else(|| {
                                self.use_chiseled_bookshelf(
                                    hit,
                                    state,
                                    None,
                                    UseBranch::WithoutItem,
                                )
                            })
                    });
                    let result = if let Some(answer) = answer {
                        native_hooks::block_use_result(answer).unwrap_or(InteractionResult::Pass)
                    } else {
                        empty_template_ran = true;
                        let player = self.use_context();
                        self.with_level(|b, level, ticks| {
                            b.use_without_item(level, ticks, state, block_hit, player)
                        })
                    };
                    (result, UseBranch::WithoutItem)
                } else {
                    (InteractionResult::Pass, UseBranch::ItemOn)
                };
                self.process_level_events()?;
                // CakeBlock.useItemOn: the candle is used up and counted.
                if block_answer.is_none()
                    && adds_candle
                    && result == horizon_world::level::InteractionResult::Success
                    && let Some(used) = held.as_ref()
                {
                    self.award_stat("used", &used.item, 1);
                    self.consume_held();
                }
                // FlowerPotBlock.useItemOn: the plant is potted, used up, and counted.
                if block_answer.is_none()
                    && pots_flower
                    && result == horizon_world::level::InteractionResult::Success
                    && let Some(used) = held.as_ref()
                {
                    self.award_stat("used", &used.item, 1);
                    self.award_custom_stat("pot_flower", 1);
                    self.consume_held();
                    if let (Some(player), Some(name)) =
                        (self.lan_acting_uuid, self.lan_acting_name.clone())
                    {
                        self.native_player_flower_pot_manipulate(
                            player, &name, hit.pos, &used.item,
                        );
                    }
                }
                // These effects belong to the vanilla method, not its Part replacement.
                if empty_template_ran && result == horizon_world::level::InteractionResult::Success
                {
                    let name = self
                        .res
                        .facts()
                        .get(self.world.block(hit.pos[0], hit.pos[1], hit.pos[2]))
                        .name
                        .clone();
                    match &*name {
                        "minecraft:note_block" => self.award_custom_stat("tune_noteblock", 1),
                        "minecraft:bell" => self.award_custom_stat("bell_ring", 1),
                        "minecraft:lever" => self.lever_puff(hit.pos),
                        _ => {}
                    }
                }
                if result.consumes_action() {
                    // ServerPlayerGameMode.useItemOn: ITEM_USED_ON_BLOCK with
                    // the stack before the use, or DEFAULT_BLOCK_USE.
                    match branch {
                        horizon_world::level::UseBranch::ItemOn => {
                            let used = held.unwrap_or_else(|| ItemStack::new("minecraft:air", 0));
                            self.trigger_block("minecraft:item_used_on_block", hit.pos, &used);
                        }
                        horizon_world::level::UseBranch::WithoutItem => {
                            self.trigger_default_block_use(hit.pos);
                        }
                    }
                    if result.should_swing() {
                        // SUCCESS swings on the client too; the beds, light,
                        // respawn anchor and sign's own uses are SUCCESS_SERVER.
                        if server_only_block_use(self.res.facts().get(state), branch) {
                            self.swing_arm();
                        } else {
                            self.swing_arm_predicted();
                        }
                    }
                    return Ok(true);
                }
            }
        }
        // RightClickBlock's useItem FALSE: no ItemStack.useOn, so only
        // Item.use is left (a guest's UseItem asks for it).
        if !use_item.or(true) {
            if on_block {
                return Ok(false);
            }
            block_hit = None;
        }
        drop(block_phase);
        let _item_phase = horizon_client::spans::guard("use_item");
        let Some(stack) = self.held().cloned() else {
            return Ok(false);
        };
        let Some(info) = self.items().get(&stack.item).cloned() else {
            return Ok(false);
        };
        // ServerPlayerGameMode.useItem and useItemOn: an item on cooldown is
        // not used (PASS, so the other hand may still act), unless
        // RightClickBlock's useItem TRUE forces a block click's useOn.
        let forced = on_block && use_item == NativeTriState::True;
        if !forced
            && self
                .cooldowns
                .is_on_cooldown(&horizon_client::cooldowns::group(
                    &stack,
                    info.use_cooldown.as_ref(),
                ))
        {
            return Ok(false);
        }
        // ItemStack.useOn: adventure's predicate selects Item.useOn before
        // its event and Part, not just before the eventual block placement.
        if let Some(hit) = block_hit
            && !self.may_build()
            && !self.can_place_on_in_adventure(hit.pos)
        {
            if on_block {
                return Ok(false);
            }
            block_hit = None;
        }
        // Native mods, where ItemStack.useOn and Item.use start: the block
        // aimed at had its turn, and the item is off cooldown. A guest's
        // UseItemOn reaches only useOn, its UseItem only use.
        let mut claimed = false;
        if let Some(hit) = block_hit {
            if let Some(answer) = self.use_item_on_block(
                hit,
                self.hand,
                Some(&stack),
                mod_api::UsePhase::ItemAfterBlock,
            ) {
                if answer.consumes_action() {
                    self.trigger_block("minecraft:item_used_on_block", hit.pos, &stack);
                    if matches!(answer, Answer::Success | Answer::SuccessServer) {
                        self.swing_arm();
                    }
                    return Ok(true);
                }
                if answer == Answer::Fail {
                    self.use_failed = true;
                    return Ok(true);
                }
                if on_block {
                    return Ok(false);
                }
                block_hit = None;
            } else {
                let (player, player_entity_id) = self.native_clicker();
                let (hand, at) = (self.hand, hit.native());
                self.native_use_on(mod_api::__private::UseOnCall {
                    stack: &stack,
                    pos: hit.pos,
                    face: hit.face,
                    hand,
                    hit: at,
                    player,
                    player_entity_id,
                    claimed: &mut claimed,
                });
            }
        }
        if !claimed && !on_block {
            if let Some(answer) = self.right_click_item(self.hand, &stack, aimed_at_block) {
                if matches!(answer, Answer::Success | Answer::SuccessServer) {
                    self.swing_arm();
                }
                return Ok(answer.consumes_action());
            }
            self.native_use_item(&stack, &mut claimed);
        }
        if claimed {
            self.swing_arm();
            return Ok(true);
        }
        // LeadItem.useOn follows the block and the item's override, including
        // when sneaking bypassed the block's useWithoutItem.
        if block_hit.is_some() && &*stack.item == "minecraft:lead" && self.interact_knot() {
            return Ok(true);
        }
        // ItemStack.useOn: after the block's own useItemOn/useWithoutItem
        // passed. DebugStickItem.useOn cycles the selected property.
        if let Some(hit) = block_hit
            && let Some(success) = self.debug_stick_use_on(hit.pos)?
        {
            // `Minecraft.startUseItem`: the stick's client-side `useOn` is
            // SUCCESS, so the actor's hand swings; a remote guest swings in
            // its own client, not on the server.
            if self.lan_acting.is_none() {
                self.swing_arm();
            }
            if success {
                // `ServerPlayerGameMode.useItemOn`: a consuming `useOn`
                // triggers ITEM_USED_ON_BLOCK with the stack before the use.
                self.trigger_block("minecraft:item_used_on_block", hit.pos, &stack);
            } else {
                // DebugStickItem.useOn's FAIL.
                self.use_failed = true;
            }
            return Ok(true);
        }
        // TntBlock.useItemOn is the block's own: a sneak with something in hand skips it, and
        // the flint and steel or fire charge lights a fire on the TNT instead.
        if block_hit.is_some()
            && (self.use_bone_meal(&stack)?
                || (!(sneaking && hands_full) && self.ignite_tnt(&stack)?))
        {
            return Ok(true);
        }
        // ShearsItem.useOn: shears crop a growing plant's head to its top age.
        if let Some(hit) = block_hit
            && self.shears_crop(hit.pos, &stack)?
        {
            return Ok(true);
        }
        // A guest's UseItemOn stops at ItemStack.useOn: what Item.use does
        // (throw, drink, eat, raise, equip) waits for its UseItem.
        let spawn_egg = stack.item.ends_with("_spawn_egg");
        use horizon_client::first_person::UseResult;
        let swings = self.swings_started;
        match self.use_tool(&stack, block_hit.is_some())? {
            UseResult::Success { swing } => {
                if (UseResult::Success { swing }).should_swing(swings, self.swings_started) {
                    self.swing_arm();
                }
                return Ok(true);
            }
            UseResult::Fail => {
                self.use_failed = true;
                return Ok(true);
            }
            UseResult::Pass => {}
        }
        // PotionItem.useOn comes before PotionItem.use's drink.
        if let Some(hit) = block_hit
            && self.use_water_bottle_on_block(hit.pos, hit.face, &stack)?
        {
            return Ok(true);
        }
        // a mod's item used over time: its Item.use, once a block item's
        // useOn had its turn.
        if !on_block && (info.block.is_none() || block_hit.is_none()) && self.start_mod_use(&stack)
        {
            return Ok(true);
        }
        if (!on_block && self.use_potion_item(&stack)?)
            || ((!on_block || (spawn_egg && block_hit.is_some())) && self.throw_item(&stack)?)
        {
            return Ok(true);
        }
        // BrushItem.useOn: brushing starts on the block aimed at (CONSUME).
        if block_hit.is_some() && &*stack.item == "minecraft:brush" {
            self.start_using_item();
            return Ok(true);
        }
        if !on_block
            && (self.raise_shield(self.held_slot())
                || self.use_fishing_rod(&stack)
            || self.use_crossbow(&stack)
            || self.use_trident(&stack)
            || self.use_spear(&stack)
            || self.boost_firework(&stack)
            || self.use_empty_map(&stack)
                || self.use_book(&stack)
                // KnowledgeBookItem.use: the recipes it names become known.
                || self.use_knowledge_book(&stack)
                // Block items (heads, carved pumpkins) aimed at a block are
                // placed first (useItemOn before useItem).
                || (!(info.block.is_some() && block_hit.is_some())
                    && self.equip_from_hand(&stack)))
        {
            return Ok(true);
        }
        if !on_block && &*stack.item == "minecraft:spyglass" {
            self.use_spyglass();
            return Ok(true);
        }
        // FoodOnAStickItem.use with no boost (not riding its mount, or the
        // mount refused): PASS, but counted as a use of the stick.
        if !on_block
            && matches!(
                &*stack.item,
                "minecraft:carrot_on_a_stick" | "minecraft:warped_fungus_on_a_stick"
            )
        {
            self.award_stat("used", &stack.item, 1);
        }
        // BundleItem.use: startUsingItem, then onUseTick empties it.
        if !on_block && info.bundle {
            self.start_using_item();
            self.swing_arm();
            return Ok(true);
        }
        if !on_block && &*stack.item == "minecraft:goat_horn" {
            self.use_instrument(&stack);
            return Ok(true);
        }
        if !on_block && &*stack.item == "minecraft:bow" {
            // BowItem.use: ArrowNockEvent, then FAIL without ammunition.
            let found_projectile = self.get_projectile(&stack).is_some();
            let has_ammo = found_projectile || self.creative;
            let player = horizon_client::lan::playerdata::ints_uuid(self.acting_player_uuid());
            let name = self.acting_player_name().to_owned();
            let (hand, mut cancel) = (self.hand, false);
            self.native_arrow_nock(player, &name, &stack, hand, has_ammo, &mut cancel);
            if has_ammo && !cancel {
                self.start_using_item();
            }
            return Ok(true);
        }
        // Block foods attempt placement before starting consumption.
        if !on_block && (info.block.is_none() || block_hit.is_none()) && self.start_eating(&info) {
            return Ok(true);
        }
        // BoatItem.use also performs its own fluid/block raycast.
        if !on_block && self.place_boat(&stack) {
            return Ok(true);
        }
        if let Some(hit) = block_hit
            && (self.launch_firework(hit, &stack)
                || self.place_minecart(hit, &stack)
                || self.place_painting(hit, &stack)
                || self.place_item_frame(hit, &stack)
                || self.use_lodestone_compass(hit, &stack)
                || self.use_map_on_banner(hit, &stack)
                || self.place_armor_stand(hit, &stack)
                || self.place_cushion(hit, &stack)
                || self.place_end_crystal(hit, &stack))
        {
            return Ok(true);
        }
        if self.use_ender_eye(&stack, block_hit.is_some())? {
            return Ok(true);
        }
        // PlaceOnWaterBlockItem.use: the lily pad and frogspawn aim at the block
        // or water source the SOURCE_ONLY ray meets and are placed on the
        // position above it.
        let water_placement = matches!(&*stack.item, "minecraft:lily_pad" | "minecraft:frogspawn");
        let water_hit;
        let hit = if water_placement {
            let Some(h) = self.pick_fluid() else {
                return Ok(false);
            };
            water_hit = Hit {
                pos: [h.pos[0], h.pos[1] + 1, h.pos[2]],
                ..h
            };
            &water_hit
        } else {
            let Some(hit) = block_hit else {
                return Ok(false);
            };
            hit
        };
        if info.block.is_none() {
            return Ok(false);
        }
        if water_placement && !self.may_build() && !self.can_place_on_in_adventure(hit.pos) {
            // The fluid-ray placement has its own target, selected by Item.use.
            return Ok(!on_block && self.start_eating(&info));
        }
        let face = match hit.face {
            [0, 1, 0] => Direction::Up,
            [0, -1, 0] => Direction::Down,
            [0, 0, -1] => Direction::North,
            [0, 0, 1] => Direction::South,
            [-1, 0, 0] => Direction::West,
            _ => Direction::East,
        };
        // BlockItem.place through the vanilla placement rules.
        let ctx = PlaceContext {
            clicked_pos: BlockPos::new(hit.pos[0], hit.pos[1], hit.pos[2]),
            face,
            hit: Vec3::new(hit.point[0], hit.point[1], hit.point[2]),
            inside: false,
            yaw: self.yaw,
            pitch: self.pitch,
            sneaking: self.keys.held(Mapping::Sneak),
            item: stack.item.to_string(),
            can_use_game_master_blocks: self.can_use_game_master_blocks(),
        };
        let outcome = if self.player_place_canceled(&ctx) {
            PlaceOutcome::Failed
        } else {
            self.with_level(|b, level, ticks| b.place_block(level, ticks, &ctx))
        };
        let PlaceOutcome::Placed { pos, state } = outcome else {
            self.process_level_events()?;
            // BlockItem.useOn: a consumable that could not be placed is used,
            // which ItemStack.useOn counts as a use of the item.
            if self.start_eating(&info) {
                self.award_stat("used", &stack.item, 1);
            } else {
                // BlockItem.place's FAIL.
                self.use_failed = true;
            }
            return Ok(true);
        };
        let target = [pos.x, pos.y, pos.z];
        let state = self.apply_item_block_state(target, state, &stack)?;
        let name = self.res.facts().get(state).name.clone();
        if let Some(entity) = self.new_block_entity(&name) {
            let key = (target[0], target[1], target[2]);
            self.world.block_entities.insert(key, entity);
            let game_master = self.can_use_game_master_blocks();
            self.apply_placed_item(target, &stack, game_master);
            // Block.setPlacedBy.
            match self.world.block_entities.get(&key) {
                Some(BlockEntity::CommandBlock(_)) => {
                    let data = stack.data().is_some_and(|d| {
                        d.other(horizon_client::block_entity_item::BLOCK_ENTITY_DATA)
                            .is_some()
                    });
                    self.command_block_placed(key, !data);
                }
                Some(BlockEntity::StructureBlock(_)) => self.structure_block_placed(key),
                Some(BlockEntity::JigsawBlock(_)) => self.jigsaw_block_placed(key),
                Some(BlockEntity::TestBlock(_)) => self.test_block_placed(key),
                Some(BlockEntity::Jukebox(_)) => self.jukebox_placed(target, &stack),
                Some(BlockEntity::Sign(sign)) if !sign.waxed => self.open_sign_editor(key, true),
                _ => {}
            }
        }
        // BlockItem.place's SUCCESS: the client swings with its own prediction.
        self.swing_arm_predicted();
        // BlockItem.place's sound is among the level events;
        // SolidBucketItem.getPlaceSound replaces it with the bucket's own.
        let solid_bucket = &*stack.item == "minecraft:powder_snow_bucket";
        if solid_bucket {
            for event in &mut self.level_events {
                if let LevelEvent::Sound { pos: at, sound, .. } = event
                    && *at == pos
                {
                    *sound = "minecraft:item.bucket.empty_powder_snow".into();
                }
            }
        }
        self.process_level_events()?;
        // BlockItem.place: the used statistic and the placed_block trigger.
        self.award_stat("used", &stack.item, 1);
        self.trigger_block("minecraft:placed_block", target, &stack);
        self.check_wither_spawn(target)?;
        self.check_golem_spawn(target)?;
        self.consume_held();
        // SolidBucketItem.useOn: BucketItem.getEmptySuccessItem in hand.
        if solid_bucket && !self.creative {
            *self.held_mut() = Some(ItemStack::new("minecraft:bucket", 1));
        }
        Ok(true)
    }
    /// Asks the mods whether the acting guest may empty `stack` at `pos`
    /// (`native_bucket_empty`): true when one cancelled it. A game's own
    /// player is not asked.
    fn bucket_empty_cancelled(&mut self, stack: &ItemStack, pos: [i32; 3]) -> bool {
        let (Some(uuid), Some(name)) = (self.lan_acting_uuid, self.lan_acting_name.clone()) else {
            return false;
        };
        let fluid = match &*stack.item {
            "minecraft:lava_bucket" => "minecraft:lava",
            "minecraft:powder_snow_bucket" => "minecraft:powder_snow",
            _ => "minecraft:water",
        };
        let mut cancel = false;
        self.native_bucket_empty(uuid, &name, pos, fluid, &mut cancel);
        cancel
    }
    /// The player as a block's `useItemOn` / `useWithoutItem` sees them.
    fn use_context(&self) -> PlayerContext {
        PlayerContext {
            yaw: self.yaw,
            pitch: self.pitch,
            may_build: self.may_build(),
            can_eat: self.creative
                || self
                    .player
                    .as_ref()
                    .is_some_and(|p| p.health.food.food < 20),
            game_master: self.can_use_game_master_blocks(),
        }
    }
    /// `Consumable.startConsuming` for a held consumable: food needs
    /// `Player.canEat` (`Consumable.canConsume`), anything else is always
    /// consumed. The right click is held for `consume_seconds`.
    fn start_eating(&mut self, info: &horizon_client::items::ItemInfo) -> bool {
        let Some(stack) = self.held().cloned() else {
            return false;
        };
        let Some(consumable) = stack.consumable(Some(info)) else {
            return false;
        };
        if info.food.is_some()
            && (self.player.is_none() || !self.can_eat(&stack, info.can_always_eat))
        {
            return false;
        }
        // as Consumable.startConsuming does, no time is eaten at once, without a use
        // (the server's eating reaches a client through the inventory).
        if item_use::consumes_at_once(consumable.ticks()) {
            if self.remote.is_none() {
                // the cooldown of ItemStack.use's applyAfterUseComponentSideEffects.
                self.apply_use_cooldown(&stack);
                self.finish_consuming(info, consumable, &stack);
            }
            return true;
        }
        self.start_using_item();
        true
    }
    /// `InstrumentItem.use` for a goat horn: the player holds it up for the
    /// instrument's `use_duration`, its sound plays at the player
    /// (`SoundSource.RECORDS`, volume `range / 16`), `INSTRUMENT_PLAY` is
    /// posted, the used statistic counts and the horn is on cooldown as long.
    /// A horn whose instrument is unknown fails, which still ends the click.
    fn use_instrument(&mut self, stack: &ItemStack) {
        let Some(instrument) = self.res.items().instrument(stack) else {
            return;
        };
        self.start_using_item();
        self.play_player_bound_sound(&instrument.sound, 2, instrument.range / 16.0, 1.0, true);
        self.post_entity_event(
            "minecraft:instrument_play",
            horizon_client::mobs::Who::Player,
        );
        if instrument.use_duration > 0.0 {
            let group = horizon_client::cooldowns::group(stack, None);
            self.add_cooldown(group, instrument.use_ticks());
        }
        self.award_stat("used", &stack.item, 1);
    }
    fn apply_consume_effect(
        &mut self,
        effect: &horizon_client::items::ConsumeEffect,
        mob: Option<mod_api::LivingRef>,
        allow_native: bool,
    ) -> Option<bool> {
        use horizon_client::items::ConsumeEffect;
        Some(match effect {
            ConsumeEffect::ApplyEffects {
                effects,
                probability,
            } => {
                // ApplyStatusEffectsConsumeEffect draws from the user's random,
                // even at probability 1. The level random stands in for a player's.
                let draw = match mob {
                    Some(living) => {
                        let commands::Target::Mob(id) = self.native_living_target(living)? else {
                            return None;
                        };
                        self.mobs
                            .iter_mut()
                            .find(|m| m.id == id)?
                            .random
                            .next_float()
                    }
                    None => self.world.random.next_float(),
                };
                if draw >= *probability {
                    return Some(false);
                }
                let mut changed = false;
                for eff in effects {
                    let Some(def) = horizon_client::brewing::effect(&eff.id) else {
                        continue;
                    };
                    if let Some(living) = mob {
                        let effect = mod_api::NativeActiveEffect::new(
                            mod_api::__private::effect_key(def.index)
                                .expect("effects resolved at load"),
                            eff.duration,
                            eff.amplifier,
                            eff.ambient,
                            eff.show_particles,
                        );
                        changed |= self.living_add_effect(living, &effect);
                    } else {
                        let mut instance = horizon_client::effects::EffectInstance::new(
                            def.id,
                            eff.duration,
                            eff.amplifier,
                        );
                        instance.ambient = eff.ambient;
                        instance.visible = eff.show_particles;
                        instance.show_icon = eff.show_icon;
                        changed |= self.add_player_effect(instance);
                    }
                }
                changed
            }
            ConsumeEffect::ClearAllEffects => match mob {
                Some(living) => self.living_remove_all_effects(living),
                None => self.remove_player_effects(None),
            },
            ConsumeEffect::RemoveEffects(effects) => {
                let mut changed = false;
                for effect in effects {
                    changed |= match mob {
                        Some(living) => horizon_client::brewing::effect(effect)
                            .and_then(|def| mod_api::__private::effect_key(def.index))
                            .is_some_and(|key| self.living_remove_effect(living, key)),
                        None => horizon_client::brewing::effect(effect)
                            .is_some_and(|def| self.remove_player_effects(Some(def.index))),
                    };
                }
                changed
            }
            ConsumeEffect::TeleportRandomly(teleport) => {
                if mob.is_some() {
                    return None;
                }
                self.teleport_randomly(teleport)
            }
            ConsumeEffect::PlaySound(sound) => {
                let at = match mob {
                    Some(living) => {
                        let commands::Target::Mob(id) = self.native_living_target(living)? else {
                            return None;
                        };
                        self.mobs.iter().find(|m| m.id == id).map(|m| m.position)?
                    }
                    None => self.player.as_ref()?.position,
                };
                let b = horizon_core::position::BlockPos::containing(at);
                let at = [b.x, b.y, b.z].map(|v| f64::from(v) + 0.5);
                let level = self.level(self.dim.id())?;
                let source = match mob {
                    Some(living) => self.entity_sound_source(living.entity())?,
                    None => mod_api::SoundSource::Players,
                };
                self.native_queue_sound(level, sound, at, source, 1.0, 1.0, None)?;
                true
            }
            ConsumeEffect::Native { kind, data } => {
                if !allow_native {
                    return None;
                }
                self.native_consume_effect(kind, data);
                return None;
            }
        })
    }
    fn consume_held(&mut self) {
        if self.creative {
            return;
        }
        let cell = self.held_mut();
        if let Some(s) = cell.as_mut() {
            s.count -= 1;
            if s.count == 0 {
                *cell = None;
            }
        }
    }
    fn tick_using(&mut self) {
        let Some(hand) = self.used_hand() else {
            return;
        };
        self.with_hand(hand, Self::tick_used_item);
    }
    /// `LivingEntity.updatingUsingItem` for the item in use, its hand acting.
    fn tick_used_item(&mut self) {
        let Some((slot, ticks)) = self.using else {
            return;
        };
        let Some(stack) = self.stack_in(slot).cloned() else {
            self.using = None;
            return;
        };
        let hand_changed = self.use_hand_changed(slot);
        if !self.right || hand_changed || self.screen != Screen::Playing {
            self.using = None;
            self.release_used_item(&stack, ticks, hand_changed);
            return;
        }
        // ServerPlayer.updateUsingItem: USING_ITEM every tick of the use.
        self.trigger_using_item(&stack);
        let Some(info) = self.items().get(&stack.item).cloned() else {
            self.using = None;
            return;
        };
        let going_on = self.use_item_ticked(&stack, ticks);
        // A mod's tick released it: nothing of this use is left to run.
        if self.using != Some((slot, ticks)) {
            return;
        }
        // a mod's tick ran the use out: finished now, or frozen until let go
        // for an item used on release (CrossbowItem.useOnRelease).
        if !going_on {
            if &*stack.item != "minecraft:crossbow" {
                self.complete_using_item(slot, &stack, &info);
            }
            return;
        }
        if self.blocks_attacks(&stack).is_some() {
            // ShieldItem: used for 72000 ticks, i.e. until released.
            if self.item_on_use_tick(&stack, ticks) {
                self.using = Some((slot, ticks + 1));
            }
            return;
        }
        if self.tick_drinking(slot, ticks, &stack) {
            return;
        }
        if &*stack.item == "minecraft:bow" {
            self.using = Some((slot, ticks + 1));
            return;
        }
        if &*stack.item == "minecraft:crossbow" {
            self.tick_crossbow(slot, ticks, &stack);
            return;
        }
        // TridentItem and EnderEyeItem: used until released (an eye's
        // getUseDuration 0 never counts down to finishUsingItem).
        if matches!(&*stack.item, "minecraft:trident" | "minecraft:ender_eye") {
            self.using = Some((slot, ticks + 1));
            return;
        }
        // ItemStack.onUseTick: a spear stabs what its speed allows, until released.
        if horizon_client::spear::Kinetic::of(&stack.item).is_some() {
            self.spear_tick(&stack, ticks);
            self.using = Some((slot, ticks + 1));
            return;
        }
        if &*stack.item == "minecraft:spyglass" {
            // SpyglassItem.getUseDuration (1200, as a mod left it), then
            // completeUsingItem: finishUsingItem lowers it, and the finish hook.
            match item_use::count_down(ticks, self.use_duration) {
                Some(ticks) => self.using = Some((slot, ticks)),
                None => self.complete_using_item(slot, &stack, &info),
            }
            return;
        }
        if let Some(duration) = self
            .res
            .items()
            .instrument(&stack)
            .map(|i| i.use_ticks().max(0) as u32)
        {
            // InstrumentItem.getUseDuration: held up that long. When it runs
            // out, LivingEntity.stopUsingItem ends the use and causes the
            // interaction vibration; letting go of the button already does.
            if ticks + 1 < duration {
                self.using = Some((slot, ticks + 1));
            } else {
                self.stop_using_item(&stack);
            }
            return;
        }
        if &*stack.item == "minecraft:brush" {
            self.tick_brush(&stack, slot, ticks);
            return;
        }
        if info.bundle {
            self.tick_bundle(slot, ticks);
            return;
        }
        if self.mod_use(&stack) {
            self.tick_mod_use(slot, ticks, &stack);
            return;
        }
        let Some(consumable) = stack.consumable(Some(&info)) else {
            self.using = None;
            return;
        };
        // ItemStack.onUseTick: Consumable.shouldEmitParticlesAndSounds with
        // the remaining ticks before this tick counts them down.
        // the duration is getUseDuration's when the use started, as a mod's
        // Start and Tick left it.
        let remaining = self.use_duration - ticks as i32;
        if consumable.should_emit(remaining) {
            self.emit_consume(consumable, &stack, 5);
        }
        if !self.item_on_use_tick(&stack, ticks) {
            return;
        }
        match item_use::count_down(ticks, self.use_duration) {
            Some(ticks) => self.using = Some((slot, ticks)),
            None => self.complete_using_item(slot, &stack, &info),
        }
    }
    /// `BundleItem.onUseTick` and `getUseDuration` (200): the first tick and
    /// then every second tick from the 11th on drop one stack out of the
    /// bundle in `slot`; the use ends after 200 ticks.
    fn tick_bundle(&mut self, slot: usize, ticks: u32) {
        const USE_DURATION: u32 = 200;
        let remaining = USE_DURATION - ticks.min(USE_DURATION);
        if ticks == 0 || (remaining < USE_DURATION - 10 && remaining.is_multiple_of(2)) {
            self.drop_bundle_content(slot);
        }
        self.using = (ticks + 1 < USE_DURATION).then_some((slot, ticks + 1));
    }
    /// `BundleItem.dropContent`: `removeOneItemFromBundle` and `Player.drop`,
    /// then the drop sound and the used statistic.
    fn drop_bundle_content(&mut self, slot: usize) {
        let items = self.items();
        let Some(mut bundle) = self.stack_in(slot).cloned() else {
            return;
        };
        let Some(mut contents) = horizon_client::bundle::Contents::of(items, &bundle) else {
            return;
        };
        let Some(removed) = contents.remove_one(items) else {
            return;
        };
        contents.store(&mut bundle);
        let item = Arc::clone(&bundle.item);
        *self.stack_in_mut(slot) = Some(bundle);
        let Some(position) = self.player.as_ref().map(|p| p.position) else {
            return;
        };
        let at = [position.x, position.y, position.z];
        // playRemoveOneSound, then Player.drop and playDropContentsSound at
        // the player's block.
        let pitch = 0.8 + self.world.random.next_float() * 0.4;
        self.audio
            .play(horizon_client::bundle::REMOVE_ONE, Some(at), 0.8, pitch);
        self.player_drop(removed);
        let block = BlockPos::containing(position);
        let pitch = 0.8 + self.world.random.next_float() * 0.4;
        self.audio.play(
            horizon_client::bundle::DROP_CONTENTS,
            Some([block.x, block.y, block.z].map(|v| f64::from(v) + 0.5)),
            0.8,
            pitch,
        );
        self.award_stat("used", &item, 1);
    }
    /// `LivingEntity.releaseUsingItem` (`Item.releaseUsing`) for `stack`,
    /// used for `ticks`, its hand acting: a bow or a trident lets go (not
    /// when its hand changed), a spyglass stops scoping, and a mod's own
    /// release follows. A mod's `Stop` may skip the release. `stopUsingItem`
    /// ends the use after it.
    pub(super) fn release_used_item(&mut self, stack: &ItemStack, ticks: u32, hand_changed: bool) {
        let canceled = !hand_changed && !self.use_item_stopped(stack, ticks);
        let (release, spyglass) = item_use::release_parts(canceled, hand_changed);
        match &*stack.item {
            "minecraft:bow" if release => self.release_bow(stack, self.ticks_held(stack, ticks)),
            "minecraft:trident" if release => self.release_trident(self.ticks_held(stack, ticks)),
            "minecraft:spyglass" if spyglass => self.stop_spyglass(),
            _ => {}
        }
        if release {
            self.mod_release_using(stack, ticks);
        }
        self.stop_using_item(stack);
    }
    /// `LivingEntity.stopUsingItem`: the use of `stack` is over (`useItem` and
    /// its remaining ticks are cleared), and when the use effects of the stack
    /// ask for it, `ItemStack.causeUseVibration` sends an `ITEM_INTERACT_FINISH`
    /// game event from the player, at its feet.
    pub(super) fn stop_using_item(&mut self, stack: &ItemStack) {
        self.using = None;
        self.use_vibration(stack, "minecraft:item_interact_finish");
    }
    /// Whether the use effects of `stack` ask for interaction vibrations.
    pub(super) fn use_vibrates(&self, stack: &ItemStack) -> bool {
        item_use::interact_vibrations(
            stack,
            self.items().get(&stack.item).map(|i| i.interact_vibrations),
        )
    }
    /// `ItemStack.causeUseVibration` for `stack` used by the player: the
    /// `event` game event (`ITEM_INTERACT_START` when the use begins,
    /// `ITEM_INTERACT_FINISH` when it ends) from the player, at its feet, when
    /// the stack's use effects ask for interaction vibrations (the default).
    pub(super) fn use_vibration(&mut self, stack: &ItemStack, event: &'static str) {
        if !self.use_vibrates(stack) {
            return;
        }
        let Some(p) = self.player.as_ref().map(|p| p.position) else {
            return;
        };
        let ctx = self.player_context(None);
        self.post_game_event(event, [p.x, p.y, p.z], ctx);
    }
    /// `ServerPlayerGameMode.tick` while a player digs: level event 2020 every
    /// fourth tick since the START (the hit sound), else 2019 (the crack only),
    /// for everyone around, the digger included (`levelEvent(null, ...)`).
    fn destroy_progress_event(&mut self, pos: [i32; 3], face: [i32; 3], ticks: u32) {
        let id = if ticks.is_multiple_of(4) { 2020 } else { 2019 };
        let direction = horizon_core::geometry::Direction::ALL
            .iter()
            .position(|d| d.normal() == face)
            .unwrap_or(1);
        self.level_event_at(id, pos, direction as i32);
    }
    fn tick_mining(&mut self) -> Result<()> {
        if self.destroy_delay > 0 {
            self.destroy_delay -= 1;
        }
        if !self.left || self.screen != Screen::Playing || self.dialog_open() {
            self.mining = None;
            return Ok(());
        }
        let Some(hit) = self.pick() else {
            self.mining = None;
            return Ok(());
        };
        // Player.blockActionRestricted: spectators, and adventure players
        // without a matching can_break item.
        if self.block_action_restricted(hit.pos)
            || !self
                .res
                .items()
                .can_destroy_block(self.held(), self.creative)
        {
            self.mining = None;
            return Ok(());
        }
        if self.creative {
            if self.destroy_delay == 0 {
                self.break_block(hit.pos)?;
                self.destroy_delay = 5;
            }
            return Ok(());
        }
        if self.destroy_delay > 0 {
            return Ok(());
        }
        if self.mining.as_ref().is_none_or(|m| m.pos != hit.pos) {
            self.attack_block(hit.pos)?;
            self.mining = Some(Mining {
                pos: hit.pos,
                progress: 0.0,
                ticks: 0,
            });
        }
        let state = self.world.block(hit.pos[0], hit.pos[1], hit.pos[2]);
        if self.res.facts().get(state).hardness < 0.0 {
            // The server still counts its destroy ticks on an unbreakable block.
            let m = self.mining.as_mut().ok_or("mining")?;
            m.ticks += 1;
            let (pos, ticks) = (m.pos, m.ticks);
            self.destroy_progress_event(pos, hit.face, ticks);
            return Ok(());
        }
        let progress = self.destroy_progress(hit.pos);
        self.swing_arm();
        let m = self.mining.as_mut().ok_or("mining")?;
        m.progress += progress;
        m.ticks += 1;
        // An instant break at START sends no destroy events.
        if m.progress < 1.0 || m.ticks > 1 {
            let (pos, ticks) = (m.pos, m.ticks);
            self.destroy_progress_event(pos, hit.face, ticks);
        }
        let m = self.mining.as_mut().ok_or("mining")?;
        // ServerPlayerGameMode breaks a player's block at once only when it is instant (START's
        // insta-mine); otherwise it waits for the client's STOP_DESTROY_BLOCK
        // (`guest_stop_digging`), never breaking it on its own count. Breaking it here would
        // take the block from under the player's crack before the player's client predicts the
        // break, so it gets neither particles nor sound.
        if m.progress >= 1.0 && (self.lan_acting.is_none() || m.ticks == 1) {
            let pos = m.pos;
            self.mining = None;
            self.break_block(pos)?;
            self.destroy_delay = 5;
        }
        Ok(())
    }
    /// A column whose surface is a full solid, non-fluid, non-leaf block open to the sky.
    fn land_column(&self, x: i32, z: i32) -> Option<i32> {
        if self.dim == Dim::Nether {
            // Below the bedrock ceiling: a solid floor with two air blocks above.
            for y in (32..120).rev() {
                let floor = self.res.facts().get(self.world.block(x, y - 1, z));
                if floor.full_collision
                    && floor.fluid == Fluid::None
                    && self.res.facts().is_air(self.world.block(x, y, z))
                    && self.res.facts().is_air(self.world.block(x, y + 1, z))
                {
                    return Some(y);
                }
            }
            return None;
        }
        let mut y = self.world.top_y() - 1;
        while y > self.world.bottom_y() {
            let s = self.world.block(x, y, z);
            if !self.res.facts().is_air(s) {
                let f = self.res.facts().get(s);
                let ok = f.full_collision
                    && f.fluid == Fluid::None
                    && !f.name.contains("leaves")
                    && f.hardness >= 0.0;
                return ok.then_some(y + 1);
            }
            y -= 1;
        }
        None
    }
    fn spawn_player(&mut self) -> Result<()> {
        if self.player.is_some() || self.remote.is_some() {
            return Ok(());
        }
        let saved = self.level.position.map(|p| Vec3::new(p[0], p[1], p[2]));
        if self.spawn.is_none() {
            self.spawn = self
                .level
                .spawn
                .map(|s| Vec3::new(s[0] + 0.5, s[1], s[2] + 0.5));
        }
        let feet = if let Some(p) = saved {
            let (cx, cz) = ((p.x.floor() as i32) >> 4, (p.z.floor() as i32) >> 4);
            self.streaming.center = (cx, cz);
            if !(-1..=1).all(|dz| (-1..=1).all(|dx| self.streaming.is_lit(cx + dx, cz + dz))) {
                return Ok(());
            }
            self.level.position = None;
            p
        } else if self.spawn.is_none() || self.player_spawn.is_some() {
            // A new world: its spawn, then the first player around it.
            if self.spawn.is_none() {
                let Some([x, y, z]) = self.find_world_spawn()? else {
                    return Ok(());
                };
                self.spawn = Some(Vec3::new(
                    f64::from(x) + 0.5,
                    f64::from(y),
                    f64::from(z) + 0.5,
                ));
                self.level.spawn = Some([f64::from(x), f64::from(y), f64::from(z)]);
                // The world's spawn from the start, not only once saved: a bed set
                // before the first save must not become it.
                self.level.world_spawn.get_or_insert([x, y, z]);
                // The "Bonus Chest" create-world option, placed once with the spawn.
                if !self.spawn_hook_cancelled
                    && horizon_client::save::WorldGen::read(&self.world_root)
                        .is_some_and(|g| g.bonus_chest)
                {
                    self.place_bonus_chest([x, y, z])?;
                }
            }
            let s = self
                .level
                .spawn
                .unwrap_or_default()
                .map(|v| v.floor() as i32);
            let Some(feet) = self.find_player_spawn(s)? else {
                return Ok(());
            };
            feet
        } else {
            let s = self.spawn.unwrap_or_default();
            let (cx, cz) = ((s.x.floor() as i32) >> 4, (s.z.floor() as i32) >> 4);
            self.streaming.center = (cx, cz);
            if !(-1..=1).all(|dz| (-1..=1).all(|dx| self.streaming.is_lit(cx + dx, cz + dz))) {
                return Ok(());
            }
            let column = [s.x.floor() as i32, s.y.floor() as i32, s.z.floor() as i32];
            if self.level.world_spawn == Some(column) {
                // TeleportTransition.missingRespawnBlock: PlayerSpawnFinder
                // around the world spawn, not its exact column; its candidates'
                // chunks load first.
                let Some(feet) = self.find_player_spawn(column)? else {
                    return Ok(());
                };
                feet
            } else {
                // A spawnpoint: the stored column, lifted onto its current surface.
                let y = self
                    .land_column(column[0], column[2])
                    .map_or(s.y, f64::from);
                Vec3::new(s.x, y.max(s.y), s.z)
            }
        };
        let mut player = PlayerMotion::new(self.motion, feet)?;
        player.yaw = self.yaw;
        let entering = self.restore_player_state;
        player.load_saved_state_once(&mut self.restore_player_state, &self.level)?;
        let spectator = self.mode == horizon_client::command::args::GameMode::Spectator;
        player.may_fly = self.creative || spectator;
        player.invulnerable = self.creative || spectator;
        player.spectator = spectator;
        if spectator {
            player.flying = true;
        } else if !player.may_fly {
            player.flying = false;
        }
        player.respiration = self.armor[0]
            .as_ref()
            .map_or(0, |s| s.enchantment_level("minecraft:respiration") as i32);
        player.pitch = self.pitch;
        player.glider = self.glider_worn();
        // A saved death already dropped inventory/XP and counted the death.
        if player.health.dead_or_dying() {
            self.set_screen(Screen::Dead);
            self.death_focus = None;
            self.ui.died_at = Some(Instant::now());
            self.ui.death_text = None;
            self.ui.death_confirm = None;
            self.capture(false);
        }
        self.previous_position = feet;
        if entering {
            // The world's first player: a server's client gets it with its
            // level (Minecraft.setLevel), which stops every sound.
            self.stop_sounds_for_new_level();
        }
        self.player = Some(player);
        self.pad.player_replaced();
        self.sway = Default::default();
        self.apply_equipment();
        self.eye = 1.62;
        self.previous_eye = 1.62;
        self.clock.reset();
        self.capture_loaded_world();
        Ok(())
    }
    /// Landing rules (farmland trampling, turtle eggs) wrote or removed the block
    /// raw: finish `Level.setBlock`'s neighbour updates, so a crop on trampled
    /// farmland breaks. The local player's and every server player's landings.
    /// The level events they send go out too: a turtle egg's break particles
    /// and sound (`decreaseEggs`' 2001, `Level.destroyBlock`'s for the last).
    ///
    /// A mod hears the change first (`native_entity_change_block_pre`, Bukkit's
    /// `EntityChangeBlockEvent`): cancelling puts the block back and skips the
    /// updates and the events, so nothing the change would have caused happens -
    /// the crop on trampled farmland stays, because breaking it is the
    /// neighbour update below and not a write of its own.
    fn landing_block_updates(
        &mut self,
        player: Option<u128>,
        event: &horizon_client::landing::FallEvent,
    ) {
        use horizon_client::landing::FallEvent;
        // The block the landing changed, what it replaced, and what it became.
        let (position, from, to, flags) = match *event {
            FallEvent::Write {
                position,
                from,
                state,
                flags,
            } => (position, from, state, flags as i32),
            FallEvent::Destroy {
                position, state, ..
            } => (position, state, self.air_state(), level_flags::UPDATE_ALL),
            FallEvent::LevelEvent {
                event,
                position,
                data,
            } => {
                self.level_event_at(event, [position.x, position.y, position.z], data);
                return;
            }
            _ => return,
        };
        if self.landing_change_cancelled(player, position, from, to) {
            self.world
                .set(position.x, position.y, position.z, from)
                .ok();
            return;
        }
        if matches!(event, FallEvent::Destroy { .. }) {
            self.level_event_at(2001, [position.x, position.y, position.z], from.0 as i32);
        }
        self.with_level(|b, level, ticks| b.notify_written(level, ticks, position, flags));
    }
    /// `Blocks.AIR.defaultBlockState`: what a landing's destroy leaves.
    fn air_state(&self) -> BlockStateId {
        self.pack
            .assets
            .registry
            .default_state("minecraft:air")
            .unwrap_or(BlockStateId(0))
    }
    /// Whether a mod cancelled a landing's block change
    /// (`native_entity_change_block_pre`): asked with the two blocks' names and
    /// the player whose landing it was (`None` when no player did it).
    fn landing_change_cancelled(
        &mut self,
        player: Option<u128>,
        position: BlockPos,
        from: BlockStateId,
        to: BlockStateId,
    ) -> bool {
        if !mod_api::__private::listened(mod_api::HookId::EntityChangeBlockPre) {
            return false;
        }
        let name = |s: BlockStateId| self.res.facts().get(s).name.to_string();
        let (from, to) = (name(from), name(to));
        let mut cancel = false;
        self.native_entity_change_block_pre(mod_api::__private::EntityChangeBlockPreCall {
            player,
            entity: None,
            kind: "minecraft:player",
            pos: [position.x, position.y, position.z],
            from: &from,
            to: &to,
            cancel: &mut cancel,
        });
        cancel
    }
    /// The clock of the animated textures: `Minecraft.runTick` ticks the `TextureManager`
    /// only while the level runs normally. Under `HORIZON_STILL` it is the world's ticks.
    pub(super) fn texture_clock(&self) -> u64 {
        if self.clock.still() {
            self.visual_ticks()
        } else {
            self.texture_ticks
        }
    }
    fn tick(&mut self) -> Result<()> {
        if self.agent.hold_server {
            return Ok(());
        }
        self.abort_open_transactions();
        // `Player.entityInteractionRange`/`blockInteractionRange` for the tick's
        // picks: the attribute carries creative's and a server's modifiers.
        self.entity_reach = self.interaction_range("minecraft:entity_interaction_range", 3.0, 5.0);
        self.block_reach = self.interaction_range("minecraft:block_interaction_range", 4.5, 5.0);
        self.tick_debug_keys()?;
        self.tick_debug_charts();
        self.tick_tutorial();
        if self.screen == Screen::Creative {
            self.refresh_creative_tabs();
            // CreativeModeInventoryScreen.containerTick: a player without
            // infinite materials gets the survival inventory instead.
            if !self.creative {
                if let Some(c) = self.carried.take() {
                    let _ = self.add_stack_to_inventory(c);
                }
                self.set_screen(Screen::Inventory);
            }
        }
        if self.remote.is_some() {
            return self.tick_remote();
        }
        // Only a server ticks its own world from here: its players are guests, swapped in for
        // their turns (`as_guest`), and the window's player is a client of it.
        self.ticks += 1;
        self.repro_tick();
        self.track_experience_display();
        self.tick_enchanting_book();
        // `/tick freeze` stops the world but not the player.
        let running = match self.level_pass.running {
            Some(running) => running,
            None => self.clock.world_runs(),
        };
        // GameRenderer.tick: a wither's bar darkens the world, then fades;
        // the activated item counts down.
        self.boss_overlay.tick_darken(running);
        self.tick_item_activation();
        self.native_inventory_ticks();
        if running {
            self.world.game_time += 1;
            if self
                .ui
                .rules
                .bool_at(horizon_client::gamerules::vanilla::ADVANCE_WEATHER)
            {
                self.world.weather.tick_timers(&mut self.world.random);
            }
            self.world.weather.tick_levels();
        }
        // ClientLevel.tick: the sky flash fades in a frozen level too.
        self.tick_weather_effects();
        let _ = self.finish_player_autosaves(false);
        // MinecraftServer.tickServer's countdown, then autoSave: three
        // hundred seconds of ticks at the rate the server runs at now.
        self.ticks_until_autosave -= 1;
        self.autosaving = self.ticks_until_autosave <= 0;
        if let Some(serving) = &self.serving {
            serving.set_saving(self.autosaving);
        }
        if self.autosaving {
            self.save_game_with(false)?;
            self.ticks_until_autosave = self.next_autosave_interval();
            if let Some(serving) = &self.serving {
                serving.set_saving(false);
            }
        }
        if running {
            self.advance_clock();
            // `MinecraftServer.tickChildren`: `commandFunctions` runs before
            // any level ticks, so a tick function that polls for the drop a
            // block just made still finds it with `Age: 0`
            // (`EntityTickList.ensureActiveIsNotIterated` gives an entity
            // added during the iteration its first tick on the next one).
            // Vanilla's SavedData is loaded long before this point; the lazy
            // scoreboard/storage read has to be done by then too, or a load
            // tag writes over what it restores.
            self.load_scoreboard();
            self.tick_functions();
            // What the functions summoned ticks this tick, as it would in
            // vanilla; what the levels add from here waits for the next.
            self.ticked_entities_through = self.next_entity;
        }
        if self.use_delay > 0 {
            self.use_delay -= 1;
        }
        self.tick_totem();
        self.update_totem_ready();
        self.tick_command_extras();
        self.tick_world(running)
    }
    /// What the tick runs after the local player's own update: effects, the
    /// world's entities and blocks, and the network hub.
    fn tick_world(&mut self, running: bool) -> Result<()> {
        // Entities added since the last tick join the level before it ticks.
        self.native_joins_flush(false, 0);
        self.share_spawn_conditions();
        horizon_client::mobs::sulfur_cube::set_tnt_explodes(
            self.ui
                .rules
                .bool_at(horizon_client::gamerules::vanilla::TNT_EXPLODES),
        );
        self.update_chunk_tickets();
        self.score_destroyed_entities();
        horizon_client::spans::time("tick_effects", || self.tick_effects(running));
        self.tick_sign_editor();
        // Player.tick's ItemCooldowns.tick for the window's player (a
        // server's players count theirs down in `tick_guests`).
        self.shield_cooldown = self.shield_cooldown.saturating_sub(1);
        self.cooldowns.tick();
        horizon_client::spans::time("tick_using", || self.tick_using());
        self.tick_bundle_hover();
        horizon_client::spans::time("tick_hud", || self.tick_hud());
        // The player's digging comes in with its packets, before the level
        // ticks; ServerLevel.tick then runs the scheduled and random block
        // ticks before any entity, so what they drop or spawn (dispensed
        // items, popped crops) moves on the tick it appears, as in vanilla.
        // On the server thread the window is a hub guest: drain its packets
        // here too, before falling-block schedules run (#4510).
        horizon_client::spans::time("tick_local_hand", || self.tick_local_hand())?;
        horizon_client::spans::time("postprocess_chunks", || self.postprocess_chunks())?;
        let mut sent = None;
        if running {
            horizon_client::spans::time("tick_lan_packets", || self.tick_lan_packets());
            sent = horizon_client::spans::time("tick_blocks", || self.tick_blocks())?;
            // What the blocks spawned (a dispensed minecart, a lit TNT, a
            // placed boat) joins before the entities tick.
            self.native_joins_flush(false, 0);
            self.lan_mark_synced();
        }
        self.begin_ordered_ticks();
        if running {
            horizon_client::spans::time("tick_paintings", || self.tick_paintings());
            horizon_client::spans::time("tick_item_frames", || self.tick_item_frames());
            self.ordered_before(ordered_tick::Batch::ArmorStands)?;
            horizon_client::spans::time("tick_armor_stands", || self.tick_armor_stands())?;
            horizon_client::spans::time("tick_cushions", || self.tick_cushions());
            // ServerLevel.tick: the arrivals of this server tick are in the level
            // (`addDuringTeleport`), and each vehicle handles its portal as its
            // own tick opens.
            self.land_portal_arrivals();
            self.handle_vehicle_portals();
            self.ordered_before(ordered_tick::Batch::Boats)?;
            horizon_client::spans::time("tick_boats", || self.tick_boats());
            self.ordered_before(ordered_tick::Batch::Carts)?;
            horizon_client::spans::time("tick_carts", || self.tick_carts());
            self.tick_vehicle_watch();
            let from = self.lan_synced_from();
            horizon_client::spans::time("tick_thunder", || self.tick_thunder());
            self.lan_synced_since(from);
            horizon_client::spans::time("tick_lecterns", || self.tick_lecterns());
            horizon_client::spans::time("tick_maps", || self.tick_maps());
            self.tick_lodestone_compasses();
            horizon_client::spans::time("tick_bolts", || self.tick_bolts());
            horizon_client::spans::time("tick_fishing", || self.tick_fishing());
            self.ordered_before(ordered_tick::Batch::Tridents)?;
            horizon_client::spans::time("tick_tridents", || self.tick_tridents());
            self.ordered_before(ordered_tick::Batch::Rockets)?;
            horizon_client::spans::time("tick_rockets", || self.tick_rockets());
            horizon_client::spans::time("tick_ominous_item_spawners", || {
                self.tick_ominous_item_spawners()
            });
        }
        if running {
            horizon_client::spans::time("entity_portals", || {
                self.handle_entity_portals();
                self.handle_end_gateway_portals();
            });
            self.ordered_before(ordered_tick::Batch::Items)?;
            horizon_client::spans::time("tick_items", || self.tick_items());
            if self.remote.is_none() {
                self.hoppers_catch_items();
            }
            // Merged, burnt, despawned or picked up.
            self.native_leaves_flush();
            self.native_joins_flush(false, 0);
            horizon_client::spans::time("touch_arrows", || self.touch_arrows());
        }
        horizon_client::spans::time("spawn_loaded_entities", || self.spawn_loaded_entities());
        // The ride the player's file names, once the vehicle is in the world.
        self.restore_root_vehicle();
        if running {
            self.ordered_before(ordered_tick::Batch::Mobs)?;
            horizon_client::spans::time("tick_mobs", || self.tick_mobs())?;
            // Dead, despawned or gone into a hive.
            self.native_leaves_flush();
            horizon_client::spans::time("recheck_golem_openers", || self.recheck_golem_openers());
            self.tick_mod_container_viewers();
            // Orbs, primed TNT and falling blocks still to tick, in join order.
            self.run_ordered_ticks(horizon_client::join_seq::JoinSeq::MAX, false)?;
            horizon_client::spans::time("tick_experience", || self.tick_experience());
            // Merged, despawned or picked up.
            self.native_leaves_flush();
            horizon_client::spans::time("entity_portals", || {
                self.find_entity_portals();
                self.resolve_arrivals();
            });
        }
        // The rest of the tick's spawns join, and what left is told of.
        self.native_joins_flush(false, 0);
        self.native_leaves_flush();
        // ServerLevel.tick broadcasts the changed blocks after the scheduled and
        // random ticks (`tick_blocks`), before the block events, the entities
        // and the block entities: what those change goes out with the next
        // tick's. A piston's moved block reaches the clients once their own
        // copy of it finished moving, so a slow tick rate shows it slide
        // (#8430).
        if let (Some(sent), Some(log)) = (sent, self.world.block_log.as_mut()) {
            self.world.block_log_held = log.split_off(sent.min(log.len()));
        }
        horizon_client::spans::time("tick_lan", || self.tick_lan(false));
        let held = std::mem::take(&mut self.world.block_log_held);
        if let Some(log) = self.world.block_log.as_mut() {
            log.extend(held);
        }
        self.tick_friends();
        horizon_client::spans::time("tick_hand", || self.tick_hand());
        horizon_client::spans::time("tick_sleep", || self.tick_sleep());
        self.announce_sleep_status();
        self.free_left_beds();
        if running {
            horizon_client::spans::time("tick_end", || self.tick_end())?;
            horizon_client::spans::time("end_gateways", || {
                self.find_end_gateway_portals();
                self.resolve_end_gateway_arrivals()
            })?;
            self.keep_gateway_destinations();
        }
        if running {
            // Primed TNT and falling blocks that joined since the spawn packets.
            self.run_ordered_ticks(horizon_client::join_seq::JoinSeq::MAX, true)?;
            self.apply_late_wind_pushes();
        }
        // Everything that has joined has had its spawn packet and, if it joined
        // before the entity passes (TNT a piston or repeater primed), its first
        // tick in the late pass above (#10063).
        self.ordered.announced = horizon_client::join_seq::JoinSeq::upcoming();
        if running {
            horizon_client::spans::time("tick_moving_pistons", || self.tick_moving_pistons())?;
            horizon_client::spans::time("tick_block_entities", || self.tick_block_entities())?;
            horizon_client::spans::time("tick_sculk_entities", || self.tick_sculk_entities());
            horizon_client::spans::time("tick_sculk_blocks", || self.tick_sculk_blocks())?;
        }
        if running {
            // ClientLevel.tickBlockEntities (not while paused or frozen):
            // chest lids, books, shulker boxes, bells, conduits, heads.
            horizon_client::spans::time("tick_block_entity_anims", || {
                self.tick_block_entity_anims()
            });
            horizon_client::spans::time("tick_particles", || self.tick_particles());
            self.tick_client_ambient();
        }
        self.tick_frame_visuals(running);
        if running {
            // MinecraftServer.tickServer: GameTestTicker.SINGLETON.tick().
            self.tick_game_tests();
        }
        // ServerBossEvent broadcasts, after this level's entities have ticked.
        self.publish_boss_bars();
        // Minecraft.handleKeybinds: held use repeats only while no item is in
        // use (a raised shield or a drawn bow places and uses nothing).
        if self.right
            && self.use_delay == 0
            && self.using.is_none()
            && self.screen == Screen::Playing
            && !self.dialog_open()
            && self.sleeping.is_none()
        {
            self.use_with_hand()?;
            self.use_delay = 4;
        }
        if running {
            horizon_client::spans::time("lan_note_joined", || self.lan_note_joined());
        }
        Ok(())
    }
    /// Keep the held-key state of toggled sneak and sprint in step with their mapping.
    /// `KeyMapping.resetToggleKeys`: every toggle mapping up, and the latched sneak and sprint
    /// keys with it.
    fn reset_toggle_keys(&mut self) {
        for key in &mut self.toggles.0 {
            key.reset();
        }
        self.sync_toggle(0);
        self.sync_toggle(1);
        self.left = false;
        self.right = false;
    }
    /// key.smoothCamera from any device toggles the cinematic camera in play.
    fn smooth_camera_action(&mut self) {
        if self.screen == Screen::Playing {
            self.smooth_camera = !self.smooth_camera;
        }
    }
    fn sync_toggle(&mut self, slot: usize) {
        let key = [KeyCode::ShiftLeft, KeyCode::ControlLeft][slot];
        if self.toggles.is_down(slot) {
            self.keys.insert(key);
        } else {
            self.keys.remove(&key);
        }
    }
    /// Let a held sneak, sprint or modifier key go when the keyboard reports
    /// its modifier up. A release does not always reach the mapping: a screen
    /// reads presses only, and an unfocused window is given no release at all,
    /// so the latch would keep the player sneaking or sprinting until the key
    /// is tapped again. The modifier state is what the keyboard itself
    /// asserts, so it settles the question. A mapping that toggles, or one a
    /// pad latched, keeps its state: `set_down(false, needs_toggle)` leaves
    /// those alone, and a toggle the player asked for is meant to outlive the
    /// key. A mapping rebound away from its modifier is left alone too: the
    /// modifier says nothing about it, and so is one a controller holds.
    ///
    /// Only `physical_keys` is cleared directly here; the logical `self.keys`
    /// entries for sneak and sprint are left to the toggle loop below, because
    /// they stand for the mapping and not the physical Shift or Control.
    /// `Screen.hasShiftDown` and `hasControlDown` read the physical state
    /// (vanilla's `InputConstants.isKeyDown`), so a Shift no one is holding
    /// makes every inventory click a shift-click and every throw a whole-stack
    /// drop. Those do not care what sneak and sprint are bound to. A modifier
    /// says nothing about which side is down, so one being up means both
    /// Shifts, or both Controls, are.
    fn heal_hold_latches(&mut self) {
        let (shift, control) = (self.modifiers.shift_key(), self.modifiers.control_key());
        if !shift {
            for key in [KeyCode::ShiftLeft, KeyCode::ShiftRight] {
                self.physical_keys.remove(&key);
            }
        }
        if !control {
            for key in [KeyCode::ControlLeft, KeyCode::ControlRight] {
                self.physical_keys.remove(&key);
            }
        }
        for (slot, mapping, needs_toggle) in [
            (0, "key.sneak", self.settings.toggle_crouch),
            (1, "key.sprint", self.settings.toggle_sprint),
        ] {
            let held = match horizon_client::keybinds::key_code(self.settings.keys.get(mapping)) {
                Some(KeyCode::ShiftLeft | KeyCode::ShiftRight) => shift,
                Some(KeyCode::ControlLeft | KeyCode::ControlRight) => control,
                _ => continue,
            };
            // A controller's stick click acts as the key without setting the modifier: it is held
            // while the pad holds it (or its release waits for a tick), not when the keyboard says.
            let pad = self.pad.holds_action(&self.settings.controller, mapping);
            if !held && !pad {
                self.toggles.0[slot].set_down(false, needs_toggle);
                self.sync_toggle(slot);
            }
        }
    }
    /// Asks the mods about each gameplay mapping a physical input (a key's or a mouse button's
    /// name) is bound to, before the game acts on it: true when one cancels a press. Only while
    /// playing, and nothing is asked with no mod listening.
    fn key_mapping_canceled(&mut self, input: Option<&str>, pressed: bool) -> bool {
        let Some(input) = input else {
            return false;
        };
        if !mod_api::__private::listened(mod_api::HookId::KeyMappingPre)
            || !gameplay_has_keyboard(self.screen, self.title.is_some(), self.dialog_open())
        {
            return false;
        }
        let names: Vec<&'static str> = self.settings.keys.translated_mappings(input).collect();
        let mut canceled = false;
        for name in names {
            let mut this = false;
            self.native_key_mapping_pre(name, pressed, &mut this);
            canceled |= this;
        }
        canceled && pressed
    }
    /// `key_mapping_canceled` for a key event, and what a cancel leaves of the key's hold: a first
    /// press that is cancelled never happened, so `restore_held_keys` must not hold it (nor its
    /// repeats); a cancelled repeat only drops its own action and keeps the hold an accepted
    /// press made.
    fn key_press_canceled(&mut self, key: KeyCode, pressed: bool, repeat: bool) -> bool {
        if !repeat {
            self.rejected_keys.remove(&key);
        }
        let name = horizon_client::keybinds::key_name(key);
        if !self.key_mapping_canceled(name, pressed) {
            return false;
        }
        if !repeat {
            self.rejected_keys.insert(key);
        }
        if self.rejected_keys.contains(&key) {
            self.physical_keys.remove(&key);
        }
        true
    }
    /// Apply a physical key to every mapping bound to it.
    fn mapped_key_event(&mut self, key: KeyCode, pressed: bool, repeat: bool) -> Result<()> {
        for mapped in self.settings.keys.translate_all(key) {
            match mapped {
                Ok(k) => {
                    self.mapped_key(k, pressed, repeat, key, Source::Keyboard)?;
                }
                Err(button) => {
                    let action = match (pressed, repeat) {
                        (false, _) => KeyAction::Release,
                        (_, false) => KeyAction::Press,
                        (_, true) => KeyAction::Repeat,
                    };
                    if let Some(pressed) = keyboard_button(
                        gameplay_has_keyboard(
                            self.screen,
                            self.title.is_some(),
                            self.screen_over_playing(),
                        ),
                        action,
                    ) {
                        if self.shared_default_acts(button, horizon_client::keybinds::key_name(key))
                        {
                            self.mouse_mapping(button, pressed, Source::Keyboard)?;
                        }
                    } else if container_key_clones(button, action) {
                        self.inventory_pick_key();
                    }
                }
            }
        }
        Ok(())
    }
    /// A key event after rebinding, as the default key of its mapping (the
    /// keyboard and controllers both end here). `raw` is the physical key
    /// the event started from (`key` where the caller started from no window
    /// key): the screens act in that keyspace (`Screen.keyPressed`), where
    /// `key`, the vanilla default the mapping translated to, may itself be
    /// another mapping's rebound key. `source` is the device the event came
    /// from: a key stays held while any device holds it.
    fn mapped_key(
        &mut self,
        key: KeyCode,
        pressed: bool,
        repeat: bool,
        raw: KeyCode,
        source: Source,
    ) -> Result<()> {
        // Sneak and sprint are ToggleKeyMappings: a press (or a key
        // repeat) flips them when toggling; only releases reach them
        // behind a screen (KeyboardHandler.keyPress).
        let toggle = match key {
            KeyCode::ShiftLeft => Some((0, self.settings.toggle_crouch)),
            KeyCode::ControlLeft => Some((1, self.settings.toggle_sprint)),
            _ => None,
        };
        if let Some((slot, option)) = toggle {
            let action = match (pressed, repeat) {
                (false, _) => KeyAction::Release,
                (_, false) => KeyAction::Press,
                (_, true) => KeyAction::Repeat,
            };
            let screen_open = !gameplay_has_keyboard(
                self.screen,
                self.title.is_some(),
                self.screen_over_playing(),
            );
            self.toggles.key(slot, action, screen_open, option);
            self.sync_toggle(slot);
            if !pressed {
                return Ok(());
            }
            if self.keys.raw(&key) {
                self.keys.press(key, source);
            }
        } else if !pressed {
            self.keys.release(&key, source);
            return Ok(());
        } else {
            self.keys.press(key, source);
        }
        // KeyboardHandler.keyPress: a repeat (InputConstants.REPEAT) reaches
        // Screen.keyPressed and KeyMapping.click like a press, so holding
        // key.drop keeps throwing, in game and over a menu slot.
        if repeat && key != KeyCode::KeyQ {
            return Ok(());
        }
        self.key_pressed(key, raw)
    }
    /// `KeyMapping.releaseAll` when a screen opens and
    /// `restoreToggleStatesOnScreenClosed` when it closes.
    fn track_toggle_screens(&mut self) {
        let dialog = self.dialog_open();
        drop_held_behind_dialog(dialog, &mut self.keys, &mut self.left, &mut self.right);
        let playing = gameplay_has_keyboard(
            self.screen,
            self.title.is_some(),
            self.screen_over_playing(),
        );
        if playing == self.was_playing {
            return;
        }
        self.was_playing = playing;
        let options = [
            self.settings.toggle_crouch,
            self.settings.toggle_sprint,
            self.settings.toggle_attack,
            self.settings.toggle_use,
        ];
        if playing {
            self.toggles.screen_closed(options);
        } else {
            self.toggles.screen_opened(options);
        }
        self.sync_toggle(0);
        self.sync_toggle(1);
        self.left = self.toggles.is_down(2);
        self.right = self.toggles.is_down(3);
    }
    /// The options `MouseHandler.turnPlayer` reads.
    fn look_options(&self) -> LookOptions {
        LookOptions {
            sensitivity: f64::from(self.settings.sensitivity),
            smooth_camera: self.smooth_camera,
            scoping: self.scoping(),
            invert_x: self.settings.invert_x,
            invert_y: self.settings.invert_y,
        }
    }
    /// `MouseHandler.handleAccumulatedMovement` and `Entity.turn`, once per frame.
    /// An agent moving the mouse (`mouse-move`) counts as a grabbed pointer in
    /// an active window, as it cannot grab the real one.
    fn apply_mouse_look(&mut self) {
        let mut look = self.look_options();
        let time = self.started.elapsed().as_secs_f64();
        let pointer = self.captured || self.agent_pointer;
        let grabbed = pointer && self.screen == Screen::Playing && self.player.is_some();
        let active = self.focused || self.agent_pointer;
        if active
            && grabbed
            && mod_api::__private::listened(mod_api::HookId::CalculatePlayerTurn)
            && let Some(remote) = &self.remote
        {
            self.native_calculate_player_turn(
                self.own_entity_id(),
                remote.profile,
                &mut look.sensitivity,
                &mut look.smooth_camera,
            );
        }
        if let Some((xo, yo)) = self
            .mouse
            .handle_accumulated_movement(time, active, grabbed, look)
        {
            self.look_turn(xo, yo);
        }
    }
    /// A look from any device, in `Entity.turn` units: the tutorial's "look
    /// around" step hears it and Horizon's zoom slows it, for the mouse and a
    /// controller alike.
    fn look_turn(&mut self, x: f64, y: f64) {
        self.tutorial.mouse(x, y);
        let zoom = self.zoom_turn_scale();
        self.turn_player(x * zoom, y * zoom);
    }
    /// `Minecraft.pause`: a single-player game with a pausing screen open.
    fn pauses_game(&self) -> bool {
        self.has_singleplayer_server()
            && !self.is_published()
            && !self.has_split_players()
            && self.title.is_none()
            && (matches!(
                self.screen,
                Screen::Paused
                    | Screen::Options
                    | Screen::Stats
                    | Screen::Advancements
                    | Screen::WorldOptions
                    | Screen::Friends
                    | Screen::Social
                    | Screen::ServerLinks
                    | Screen::ConfirmLink
                    | Screen::DialogWarning
                    | Screen::CommandBlock
                    | Screen::DebugOptions
                    | Screen::HudEditor
            ) || self.book.is_some()
                || self.win_screen_pauses())
    }
    /// A screen open over gameplay while `screen` stays `Playing`: a book or
    /// lectern, a sign, a server dialog, the code of conduct, a link or
    /// server pack prompt (vanilla's `Minecraft.screen` is set for each).
    fn screen_over_playing(&self) -> bool {
        self.book.is_some() || self.sign_edit.is_some() || self.prompt_open()
    }
    /// A prompt is up: over the world or over another open screen, it takes the input and is
    /// answered first, in the order of `handle_window_event`'s arms and of what draws on top.
    fn prompt_open(&self) -> bool {
        self.dialog_open()
            || self.code_of_conduct.is_some()
            || self.link_prompt.is_some()
            || self.command_prompt.is_some()
            || self.server_packs.prompt.is_some()
    }
    /// `Minecraft.pauseIfInactive`: a window without focus for more than 500 ms
    /// pauses the game. An open screen (books, signs, the pack prompt, a server
    /// dialog) stays instead of pausing (`Gui.setPauseScreen`).
    fn pause_if_inactive(&mut self) {
        if self
            .focus_lost
            .is_some_and(|t| t.elapsed() > Duration::from_millis(500))
            && self.control.is_none()
            && self.screen == Screen::Playing
            && self.player.is_some()
            && !self.options.no_pause
            && self.settings.file.bool("pauseOnLostFocus")
            && self.book.is_none()
            && self.sign_edit.is_none()
            && self.server_packs.prompt.is_none()
            && !self.dialog_open()
        {
            self.pause_focus = None;
            self.set_screen(Screen::Paused);
        }
    }
    /// `KeyMapping.setAll`: every key physically held is held again, as the
    /// mapping it is bound to, so a key pressed or kept down behind a screen
    /// counts once the game has the keyboard back. A sneak or sprint that
    /// toggles keeps its toggle state (`shouldSetOnIngameFocus`).
    fn restore_held_keys(&mut self) {
        let held: Vec<KeyCode> = self.physical_keys.iter().copied().collect();
        let toggles = [self.settings.toggle_crouch, self.settings.toggle_sprint];
        for key in held {
            for mapped in self.settings.keys.translate_all(key) {
                match mapped {
                    Ok(slot_key @ (KeyCode::ShiftLeft | KeyCode::ControlLeft)) => {
                        let slot = usize::from(slot_key == KeyCode::ControlLeft);
                        if !toggles[slot] {
                            self.toggles.0[slot].set_down(true, false);
                            self.sync_toggle(slot);
                        }
                    }
                    Ok(other) => {
                        self.keys.insert(other);
                    }
                    Err(_) => {}
                }
            }
        }
    }
    /// `MouseHandler.grabMouse` / `releaseMouse`. Both put the pointer at
    /// the window's centre, so a screen opened from gameplay (inventory,
    /// chest, pause, chat, sign, book, death) starts with the cursor in the
    /// middle instead of wherever the system last left it. Releasing when
    /// already released does nothing, so moving from screen to screen keeps
    /// the cursor where it is. Like GLFW's `glfwSetCursorPos`, nothing warps
    /// while the window is unfocused (losing focus releases the pointer).
    fn capture(&mut self, capture: bool) {
        // A guest's action (`as_guest`) opens and closes screens on the
        // guest's own client: the window's pointer stays the host's (#687).
        if self.lan_acting.is_some() {
            return;
        }
        // MouseHandler.grabMouse never captures an inactive window, and vanilla
        // grabs only on `setScreen(null)`: the pack prompt and server dialogs
        // are screens over the world that keep the pointer.
        if capture
            && (!self.focused
                || self.server_packs.prompt.is_some()
                || self.dialog_open()
                || self.command_prompt.is_some())
        {
            return;
        }
        // The container screen's press state belongs to the screen: a screen
        // that takes the pointer over starts with its own state, as
        // `AbstractContainerScreen` does, so a press whose release never
        // arrived (the pointer left the window) cannot wedge the next
        // screen's quick move and clicks.
        self.reset_inventory_clicks();
        if !capture {
            self.stop_mining();
        }
        // The real pointer takes over from an agent's.
        self.agent_pointer = false;
        // Splitscreen: another player has the pointer; this one's menus leave it alone.
        if !self.keyboard_mouse {
            self.captured = false;
            self.capture_mode = None;
            return;
        }
        if capture && !self.captured && !cfg!(target_os = "macos") {
            // MouseHandler.grabMouse: KeyMapping.setAll unless
            // InputQuirks.RESTORE_KEY_STATE_AFTER_MOUSE_GRAB is off (macOS).
            self.restore_held_keys();
        }
        let Some(window) = &self.window else { return };
        let size = self.renderer.as_ref().map_or_else(
            || window.inner_size().into(),
            |r| [r.config.width, r.config.height],
        );
        let [vx, vy] = self.view_at.map(f64::from);
        let centre = winit::dpi::PhysicalPosition::new(
            vx + f64::from(size[0]) / 2.0,
            vy + f64::from(size[1]) / 2.0,
        );
        let warp = |window: &Window| {
            let _ = window.set_cursor_position(centre);
        };
        let scale = HudBatch::new(size, self.settings.gui_scale_inputs()).scale;
        let was_grabbed = self.captured;
        if self.options.offscreen && !self.split_window {
            // A genuinely hidden `--offscreen` run leaves the system pointer where
            // the person at the computer has it. Its grab is internal (an
            // agent turns the view with `mouse-move`).
            if capture {
                self.mouse.set_ignore_first_move();
            }
            self.captured = capture;
            self.capture_mode = None;
        } else if capture {
            self.mouse.set_ignore_first_move();
            // macOS warps by re-associating the mouse with the pointer,
            // which undoes a Locked grab and leaves the pointer free: warp
            // first there. Wayland moves a pointer only while it is locked,
            // so everywhere else warp after grabbing (Windows does both).
            #[cfg(target_os = "macos")]
            warp(window);
            // Windows clips a Locked pointer to a 1x1 rectangle where it is, and `SetCursorPos`
            // stays inside that clip: warp first, or the pointer stays frozen wherever it was
            // (over another window, on another monitor) instead of the window's centre (#6076).
            #[cfg(windows)]
            if self.focused {
                warp(window);
            }
            self.capture_mode = if window.set_cursor_grab(CursorGrabMode::Locked).is_ok() {
                Some(CursorGrabMode::Locked)
            } else if window.set_cursor_grab(CursorGrabMode::Confined).is_ok() {
                Some(CursorGrabMode::Confined)
            } else {
                None
            };
            self.captured = self.capture_mode.is_some();
            // Windows already warped above, inside the clip's own position.
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            if self.captured && self.focused {
                warp(window);
            }
        } else {
            if self.captured {
                // Wayland moves a pointer only as the locked pointer's
                // position hint, which the compositor applies as the lock
                // goes (sway and Hyprland do): set it before ungrabbing.
                // X11, Windows and macOS warp, so warp again once free
                // (Wayland refuses that one).
                if self.focused {
                    warp(window);
                }
                let _ = window.set_cursor_grab(CursorGrabMode::None);
                if self.focused {
                    warp(window);
                }
            } else {
                let _ = window.set_cursor_grab(CursorGrabMode::None);
            }
            self.captured = false;
            self.capture_mode = None;
        }
        // MouseHandler.grabMouse and releaseMouse: the mouse position is
        // the window's centre whenever the pointer's grab state changes, so
        // a screen opened from gameplay starts with the cursor in the middle
        // and the game starts with it away from the screen's last spot.
        if let Some(pos) = horizon_client::mouse::mouse_position_after_grab_change(
            capture,
            was_grabbed,
            size,
            scale,
        ) {
            self.cursor = pos;
        }
        if !self.options.offscreen || self.split_window {
            window.set_cursor_visible(!self.captured);
        }
    }
    /// A Linux window manager can release the native pointer grab while the
    /// window remains focused. Reapply the recorded mode on the next gameplay
    /// click or wheel event, without warping the pointer or consuming input.
    fn reassert_pointer_capture(&mut self) {
        #[cfg(target_os = "linux")]
        if self.screen == Screen::Playing
            && self.focused
            && self.captured
            && !self.options.offscreen
            && let (Some(window), Some(mode)) = (&self.window, self.capture_mode)
        {
            let other = match mode {
                CursorGrabMode::Locked => CursorGrabMode::Confined,
                CursorGrabMode::Confined => CursorGrabMode::Locked,
                CursorGrabMode::None => return,
            };
            self.capture_mode = if window.set_cursor_grab(mode).is_ok() {
                Some(mode)
            } else if window.set_cursor_grab(other).is_ok() {
                Some(other)
            } else {
                None
            };
            self.captured = self.capture_mode.is_some();
            window.set_cursor_visible(!self.captured);
        }
    }
    /// `GuiGraphicsExtractor.applyCursor` -> `Window.selectCursor` at the end
    /// of a frame: the cursor the frame's widgets asked for, set on the window
    /// only when it changes.
    fn apply_cursor_icon(&mut self) {
        use horizon_client::widgets::Cursor;
        let wanted = horizon_client::widgets::take_cursor();
        // The widgets under the keyboard's hidden pointer aren't under the
        // mouse (`handleCursor` asks only for a hovered widget).
        let effective = if self.settings.allow_cursor_changes && !self.key_focus_driving() {
            wanted
        } else {
            Cursor::Arrow
        };
        if effective == self.cursor_icon || !self.keyboard_mouse {
            return;
        }
        if let Some(window) = &self.window {
            window.set_cursor(match effective {
                Cursor::Arrow => winit::window::CursorIcon::Default,
                Cursor::PointingHand => winit::window::CursorIcon::Pointer,
                Cursor::NotAllowed => winit::window::CursorIcon::NotAllowed,
            });
        }
        self.cursor_icon = effective;
    }
    /// LevelLoadingScreen.onClose -> Gui.setScreen(null) -> grabMouse.
    fn capture_loaded_world(&mut self) {
        // Loading may finish after focus was lost or another screen opened.
        if self.title.is_none() && self.screen == Screen::Playing {
            self.capture(true);
        }
    }
    /// The window came back into focus after holding the pointer: the system
    /// dropped the grab with the focus, and vanilla's grab outlives it
    /// (`MouseHandler.mouseGrabbed` stays true while GLFW keeps the cursor
    /// mode), so the game takes the pointer again without waiting for a
    /// click. A screen that is open keeps it free, as releasing the pointer
    /// for that screen did: it is the screen, not the focus, that shows it.
    fn regrab_on_refocus(&mut self) {
        if std::mem::take(&mut self.regrab_on_refocus) && self.screen == Screen::Playing {
            self.capture(true);
        }
    }
    fn draw_item(&self, batch: &mut HudBatch, stack: &ItemStack, x: f32, y: f32) {
        // `GuiGraphics.renderItem`: the resolved item model (tints, foil and
        // special renderers included), then the decorations.
        self.draw_item_model(batch, stack, x, y);
        self.draw_item_decorations(batch, stack, x, y, None);
    }
    /// `extractSlot` of slot `slot` holding `stack` at (x, y): during a quick
    /// craft drag a dragged slot shows what it would hold over a white
    /// highlight (its count yellow when the slot caps it), and hides while
    /// it is the drag's only slot.
    fn draw_slot_item(
        &self,
        batch: &mut HudBatch,
        drag: Option<&DragView>,
        slot: usize,
        stack: &Option<ItemStack>,
        x: f32,
        y: f32,
    ) {
        let carried;
        let (stack, capped) = match drag.and_then(|d| d.slots.get(&slot)) {
            Some(None) => return,
            Some(Some((count, capped))) => {
                batch.fill(&self.gui.atlas, x, y, 16.0, 16.0, rgba(255, 255, 255, 128));
                carried = self
                    .carried
                    .clone()
                    .map(|c| ItemStack { count: *count, ..c });
                (carried.as_ref(), *capped)
            }
            None => (stack.as_ref(), false),
        };
        if let Some(stack) = stack {
            self.draw_item_model(batch, stack, x, y);
            let text = capped.then(|| format!("\u{a7}e{}", stack.count));
            self.draw_item_decorations(batch, stack, x, y, text);
        }
    }
    /// `extractCarriedItem`: the carried stack at the cursor, showing what a
    /// quick craft drag over several slots leaves in hand.
    fn draw_carried(&mut self, batch: &mut HudBatch) {
        let Some(mut stack) = self.carried.clone() else {
            return;
        };
        if let Some(left) = self.drag_view().and_then(|d| d.left) {
            if left == 0 {
                return;
            }
            stack.count = left;
        }
        self.draw_item(batch, &stack, self.cursor.0 - 8.0, self.cursor.1 - 8.0);
    }
    /// The quick craft drag in progress, as the menu engine previews it.
    fn drag_view(&mut self) -> Option<DragView> {
        let (ty, slots) = self.inv_drag.clone()?;
        let mut view = None;
        self.menu_session(|menu, env, to_menu| {
            let menu_slots: Option<Vec<usize>> = slots.iter().map(|&g| to_menu(g)).collect();
            let shown = match menu.quick_craft_preview(env, ty, &menu_slots?) {
                Some((shown, left)) => {
                    let slots = slots.iter().copied().zip(shown).collect();
                    DragView {
                        slots,
                        left: Some(left),
                    }
                }
                // A single dragged slot is hidden.
                None => DragView {
                    slots: slots.iter().map(|&g| (g, None)).collect(),
                    left: None,
                },
            };
            view = Some(shown);
            Some(false)
        });
        view
    }
    /// Whether a quick craft drag takes `slot`
    /// (`shouldAddSlotToQuickCraft`); screens the menu engine doesn't
    /// model take every slot.
    fn quick_craft_accepts(&mut self, slot: usize, ty: i32, dragged: usize) -> bool {
        let mut accepts = true;
        self.menu_session(|menu, env, to_menu| {
            accepts = to_menu(slot).is_some_and(|s| menu.quick_craft_accepts(env, s, ty, dragged));
            Some(false)
        });
        accepts
    }
    /// `GuiGraphicsExtractor.itemDecorations`: the count (or `count_text`)
    /// and damage bar.
    fn draw_item_decorations(
        &self,
        batch: &mut HudBatch,
        stack: &ItemStack,
        x: f32,
        y: f32,
        count_text: Option<String>,
    ) {
        // GuiGraphicsExtractor.itemDecorations, laid out at the origin and
        // moved to (x, y): the damage bar, the cooldown shade and the count.
        let info = self.items().get(&stack.item);
        // ItemStack.isBarVisible: isDamaged, which an unbreakable stack never is.
        let max_damage = if horizon_client::stations::is_damageable(self.items(), stack) {
            stack.max_damage(info)
        } else {
            0
        };
        let cooldown =
            horizon_client::cooldowns::group(stack, info.and_then(|i| i.use_cooldown.as_ref()));
        let partial = self.clock.fraction() as f32;
        // BundleItem overrides the bar with its fullness.
        let mut bar = if info.is_some_and(|item| item.bundle) {
            horizon_client::bundle::Contents::of(self.items(), stack)
                .and_then(|contents| contents.bar())
        } else {
            hud_overlays::damage_bar(stack.effective_damage(info), max_damage)
        };
        // Client-only: a mod may replace or hide the bar vanilla just chose.
        self.native_item_bar(stack, &mut bar);
        let decorations = hud_overlays::ItemDecorations {
            count: stack.count,
            count_text,
            bar,
            cooldown: self.cooldowns.percent(&cooldown, partial),
        };
        let mut draws = vec![hud_layout::GuiDraw::Push(vec![
            hud_layout::PoseOp::Translate(x, y),
        ])];
        draws.extend(hud_overlays::item_decorations(
            0,
            0,
            &decorations,
            &self.gui.atlas.font,
        ));
        draws.push(hud_layout::GuiDraw::Pop);
        hud_layout::paint(batch, &self.gui.atlas, &draws);
        self.native_item_decorations(stack, x, y, batch);
    }
    fn hud(&mut self, size: [u32; 2]) -> HudBatch {
        // The pack prompt replaces the connecting screen (setScreen), but
        // in game it opens over the world.
        let mut batch = if (self.server_packs.prompt.is_some()
            || self.code_of_conduct.is_some()
            || self.dialog_open())
            && self.player.is_none()
        {
            HudBatch::new(size, self.settings.gui_scale_inputs())
        } else {
            self.hud_layers(size)
        };
        // Vanilla's confirmation replaces the dialog as the screen.
        if self.command_prompt.is_none() {
            self.draw_dialog(&mut batch);
        }
        self.draw_pack_prompt(&mut batch);
        self.draw_code_of_conduct(&mut batch);
        self.draw_link_prompt(&mut batch);
        self.draw_command_prompt(&mut batch);
        // The ToastManager draws over every screen, the title and the
        // options included: the narrator's toast follows the player there.
        self.draw_narrator_toast(&mut batch);
        // Gui.extractRenderState: the debug overlay comes after the screen
        // and the toasts, so it is over everything, menus included.
        self.draw_debug_overlay(&mut batch, size);
        self.frame_graph
            .draw(self.settings.horizon.overlay, &mut batch, &self.gui.atlas);
        self.draw_pad_cursor(&mut batch);
        batch
    }
    fn hud_layers(&mut self, size: [u32; 2]) -> HudBatch {
        let mut batch = HudBatch::new(size, self.settings.gui_scale_inputs());
        batch.glint_colour = self.settings.extras.pvp.tweaks.glint_texel();
        // GlobalSettingsUniform's GameTime, for a resource pack's text shader.
        batch.game_time = ((self.world.game_time.rem_euclid(24000)) as f32
            + self.clock.fraction() as f32)
            / 24000.0;
        let (w, h) = (batch.width, batch.height);
        // HDR calibration ends when its page does, whatever moved the screen
        // off Options (a death screen opened by a server, say).
        if self.screen != Screen::Options && self.ui.options.calibration_fullscreen.is_some() {
            self.show_calibration(h);
        }
        if self.screen == Screen::Win {
            let drawn = self.draw_screen_with_pre_hook(&mut batch);
            if drawn {
                self.draw_win_screen(&mut batch);
            }
            // The early return skips the match below: the hooks still see the
            // screen, after its own drawing.
            self.draw_screen_with_post_hook(&mut batch, drawn);
            return batch;
        }
        if self.screen == Screen::Options {
            if self.title.is_none() {
                self.flush_skin_slots();
                self.update_remote_skins();
            }
            let drawn = self.draw_screen_with_pre_hook(&mut batch);
            if drawn {
                self.draw_options(&mut batch);
            }
            self.draw_screen_with_post_hook(&mut batch, drawn);
            self.draw_split_hud(&mut batch, true);
            return batch;
        }
        self.settle_title_toasts();
        if let Some(title) = &mut self.title {
            if std::mem::take(&mut title.onboarding_finished) {
                // Options.onboardingAccessibilityFinished.
                self.settings.file.set(
                    "onboardAccessibility",
                    horizon_client::options_file::V::Bool(false),
                );
                self.settings.save(&Self::options_path());
            }
            if let Some(sound) = title.update_toast(self.settings.notification_display_time) {
                self.audio.play(sound, None, 1.0, 1.0);
            }
        }
        if let Some(title) = &self.title {
            for found in title.lan_found_narrations((w, h)) {
                self.say_system_queued(&found);
            }
        }
        if self.title.is_some() {
            // TitleScreen.extractRenderState, then ToastManager over it: vanilla
            // draws the toast manager on every screen, so a pack download or a
            // resource-reload failure still shows on the title (#4428).
            let drawn = self.draw_title_screen_with_pre_hook(&mut batch);
            if drawn && let Some(title) = &self.title {
                title.draw_with_items(
                    &self.gui.atlas,
                    Some(&self.pack.item_renderer),
                    &mut batch,
                    self.cursor,
                    self.settings.file.bool("hideSplashTexts"),
                    self.settings.file.bool("hideServerAddress"),
                );
            }
            // ScreenEvent.Render.Post over the title's own screens: a mod draws
            // over them here (a pack browser over Create New World, say), and the
            // toasts still come after, as over any screen.
            self.draw_title_screen_with_post_hook(&mut batch, drawn);
            self.draw_toasts(&mut batch, false);
            self.draw_tutorial_toasts(&mut batch, false);
            self.draw_pack_toasts(&mut batch, false);
            self.draw_music_toast(&mut batch, false);
            return batch;
        }
        let atlas_ptr: *const HudAtlas = &self.gui.atlas;
        let _ = atlas_ptr;
        // The ToastManager draws over every screen: a server's packs
        // download while the connection screen shows.
        if self.remote_overlay(&mut batch) {
            self.draw_pack_toasts(&mut batch, false);
            self.draw_music_toast(&mut batch, false);
            return batch;
        }
        // Hud.extractRenderState: F1 (keyToggleGui) hides camera overlays, the
        // crosshair, hotbar, effects, bars, sidebar, action bar, titles and chat;
        // the sleep overlay, debug screen and open screens still draw.
        let spectator = self.spectator();
        let (wi, hi) = (w as i32, h as i32);
        let mut gui_canceled = false;
        if !self.ui.hide_gui {
            self.native_render_gui_pre(&mut gui_canceled);
        }
        // RenderGuiEvent.Pre: a mod's cancel hides the HUD as F1 does.
        let show_hud = !self.ui.hide_gui && !gui_canceled;
        if show_hud {
            self.draw_spyglass(&mut batch);
            // Hud.extractCrosshair / extractItemHotbar / extractPlayerHealth /
            // extractVehicleHealth and the experience bar, laid out by hud_layout.
            // Combat feel Legacy hides the attack indicator unless the player keeps it.
            let combat = self.settings.extras.pvp.combat;
            let indicator = if combat.legacy && !combat.indicator {
                0
            } else {
                self.settings.attack_indicator
            };
            let delay = horizon_client::attack_strength::delay(self.main_hand_attack_speed());
            let strength = self.hand_strength.scale(0.0, self.main_hand_attack_speed());
            // The 3D crosshair (F3) takes the 2D one's place.
            // Hud.extractCrosshair: a spectator only sees it over something to open.
            if self.camera_type.is_first_person()
                && (!spectator || self.spectator_crosshair())
                && !self.debug_crosshair()
            {
                let attack = (indicator == 1).then_some(strength);
                let full = attack.is_some_and(|a| {
                    hud_layout::full_attack_indicator(a, delay, self.crosshair_living())
                });
                let draws = hud_layout::crosshair(wi, hi, attack, full);
                self.gui_layer("minecraft:crosshair", &mut batch, |g, batch| {
                    // The custom crosshair takes the sprite's place (the first draw); the
                    // attack indicator stays.
                    if g.custom_crosshair() {
                        g.draw_custom_crosshair(batch, wi, hi);
                        hud_layout::paint(batch, &g.gui.atlas, &draws[1..]);
                    } else {
                        hud_layout::paint(batch, &g.gui.atlas, &draws);
                    }
                });
            }
            self.gui_layer("minecraft:hotbar", &mut batch, |g, batch| {
                if spectator {
                    g.draw_spectator_hotbar(batch);
                } else {
                    let partial = g.clock.fraction() as f32;
                    let inputs = hud_layout::HotbarInputs {
                        selected: g.selected as i32,
                        right_handed: !g.settings.main_hand_left,
                        offhand: g.offhand.is_some(),
                        slots: std::array::from_fn(|i| {
                            g.inventory[i]
                                .as_ref()
                                .map(|_| (f32::from(g.hotbar_pop[i]) - partial).max(0.0))
                        }),
                        offhand_pop: (f32::from(g.hotbar_pop[9]) - partial).max(0.0),
                        attack: (indicator == 2).then_some(strength),
                    };
                    let draws = hud_layout::hotbar(wi, hi, &inputs);
                    hud_layout::paint(batch, &g.gui.atlas, &draws);
                    for d in &draws {
                        if let hud_layout::GuiDraw::Item { slot, x, y, .. } = *d {
                            let stack = if slot == 40 {
                                g.offhand.as_ref()
                            } else {
                                g.inventory[slot as usize].as_ref()
                            };
                            if let Some(stack) = stack {
                                g.draw_item(batch, stack, x as f32, y as f32);
                            }
                        }
                    }
                }
            });
            self.gui_layer("minecraft:effects", &mut batch, |g, batch| {
                // The PvP & HUD effects widget takes the icons' place.
                if !g.pvp_effects_shown() {
                    g.draw_effect_icons(batch);
                }
            });
            // Hud: the action bar and titles draw over the hotbar (nextStratum).
            self.gui_layer("minecraft:boss_overlay", &mut batch, |g, batch| {
                g.draw_boss_bars(batch);
            });
            self.draw_extras(&mut batch);
        }
        if show_hud {
            // Hud.nextContextualInfoState: the jump bar, the experience bar or
            // the locator bar above the hotbar.
            let has_experience = !self.creative && !spectator;
            let contextual = horizon_client::locator_bar::ContextualBar::next(
                self.waypoints.has_waypoints(),
                !spectator && self.jumpable_mount(),
                self.mount_jumping(),
                has_experience,
                self.experience_recent(),
            );
            use horizon_client::locator_bar::ContextualBar;
            // The mount's hearts.
            let vehicle_hearts = if spectator {
                0
            } else {
                self.draw_mount_hud(&mut batch, wi, hi)
            };
            // The one bar above the hotbar: the mount's jump bar, the locator
            // bar (with its background) or the experience bar, with the level
            // number that a survival player always has.
            self.gui_layer("minecraft:contextual_info_bar", &mut batch, |g, batch| {
                if contextual == ContextualBar::JumpableVehicle {
                    g.draw_jump_bar(batch, wi, hi);
                }
                if contextual == ContextualBar::Locator {
                    let (sprite, [x, y, w, h]) = horizon_client::locator_bar::background(wi, hi);
                    horizon_client::ui::nine_slice(
                        batch,
                        &g.gui.atlas,
                        sprite,
                        x as f32,
                        y as f32,
                        w as f32,
                        h as f32,
                        u32::MAX,
                    );
                }
                if has_experience && g.player.is_some() {
                    let atlas = &g.gui.atlas;
                    let width = |s: &str| HudBatch::text_width(atlas, s) as i32;
                    let draws = hud_layout::experience(
                        wi,
                        hi,
                        g.experience.progress,
                        g.experience.level,
                        contextual == ContextualBar::Experience,
                        &width,
                    );
                    hud_layout::paint(batch, atlas, &draws);
                }
                if contextual == ContextualBar::Locator {
                    let draws = g.locator_bar_draws(wi, hi);
                    hud_layout::paint(batch, &g.gui.atlas, &draws);
                }
            });
            let (visual_ticks, visual_millis) = (self.visual_ticks(), self.visual_millis());
            let kind = (!self.creative && !spectator)
                .then(|| self.player_heart_kind())
                .flatten();
            if let Some(kind) = kind
                && let Some(p) = &mut self.player
            {
                let effects = &self.potions.effects;
                // Hud.extractAirBubbles: `isEyeInFluidMatching`, so the eyes are below
                // the surface, not merely in the water block.
                let under_water = p.is_eye_in_fluid(horizon_client::motion::WATER);
                let inputs = hud_layout::HealthInputs {
                    tick: visual_ticks as i32,
                    millis: visual_millis,
                    health: p.health.health(),
                    max_health: p.attribute("minecraft:max_health").unwrap_or(20.0) as f32,
                    absorption: p.health.absorption(),
                    armor: p
                        .attribute("minecraft:armor")
                        .or_else(|_| p.attribute("armor"))
                        .unwrap_or(0.0) as i32,
                    food: p.health.food.food,
                    saturation: p.health.food.saturation,
                    kind,
                    regeneration: effects.get("regeneration").is_some(),
                    hunger: effects.get("hunger").is_some(),
                    hardcore: self.level.hardcore,
                    damage_cooldown: p.health.hurt.cooldown > 0,
                    air: p.air,
                    max_air: 300,
                    under_water,
                    vehicle_hearts,
                };
                let at = [p.position.x, p.position.y, p.position.z];
                // The native mods' health HUD (`native_health_hud`): a mod
                // that answers draws every row instead.
                let mut native_health_hud = None;
                self.native_health_hud(&inputs, wi, hi, &mut native_health_hud);
                let draws = match native_health_hud {
                    Some(draws) => draws,
                    None => self.health_hud.layout(&inputs, wi, hi),
                };
                // The rows draw in one pass here, so a mod's `player_health`
                // layer stands for the armor, food and air rows too.
                self.gui_layer("minecraft:player_health", &mut batch, |g, batch| {
                    hud_layout::paint(batch, &g.gui.atlas, &draws);
                });
                // Hud.playAirBubblePoppedSound: Player.playSound (PLAYERS), heard
                // by this client only.
                if let Some((volume, pitch)) = self.health_hud.bubble_pop_sound.take() {
                    let players = horizon_client::settings::SOUND_SOURCES
                        .iter()
                        .position(|s| *s == "player")
                        .unwrap_or(0);
                    let capture = self.audio.capture.take();
                    self.audio
                        .play_on(players, "ui.hud.bubble_pop", Some(at), volume, pitch);
                    self.audio.capture = capture;
                }
            }
            if !spectator {
                self.gui_layer("minecraft:selected_item_name", &mut batch, |g, batch| {
                    g.draw_selected_name(batch, !g.creative);
                });
            }
        }
        // Hud.extractSleepOverlay: over all health/food bars, under titles and chat.
        let draws = hud_overlays::sleep_overlay(wi, hi, self.sleep_timer as i32);
        self.gui_layer("minecraft:sleep_overlay", &mut batch, |g, batch| {
            hud_layout::paint(batch, &g.gui.atlas, &draws);
        });
        if show_hud {
            self.gui_layer("minecraft:overlay_message", &mut batch, |g, batch| {
                g.draw_overlay_message(batch);
            });
            self.gui_layer("minecraft:title", &mut batch, |g, batch| {
                g.draw_title(batch);
            });
            self.gui_layer("minecraft:scoreboard_sidebar", &mut batch, |g, batch| {
                g.draw_scoreboard(batch);
            });
            self.gui_layer("minecraft:subtitle_overlay", &mut batch, |g, batch| {
                g.draw_subtitles(batch);
            });
        }
        let atlas = &self.gui.atlas;
        if show_hud
            && let Some((text, at)) = &self.message
            && at.elapsed() < Duration::from_secs(3)
        {
            let tw = HudBatch::text_width(atlas, text);
            batch.text(atlas, text, (w - tw) / 2.0, h - 59.0, u32::MAX, true);
        }
        // The chat screen draws the focused chat itself; F1 hides the rest.
        if show_hud && self.screen != Screen::Chat {
            self.gui_layer("minecraft:chat", &mut batch, |g, batch| g.draw_chat(batch));
            self.gui_layer("minecraft:tab_list", &mut batch, |g, batch| {
                g.draw_tab_list(batch);
            });
        } else {
            self.draw_chat(&mut batch);
            self.draw_tab_list(&mut batch);
        }
        // Screen.extractBlurredBackground blurs the world and HUD, while the
        // screen's background, widgets, toasts and debug overlay stay sharp.
        batch.blur_at = batch.vertices.len() as u32;
        // PackConfirmScreen takes the open screen's place (it comes back once
        // the prompt is answered): the screen under the prompt isn't drawn.
        let screen = if self.server_packs.prompt.is_some() || self.book.is_some() {
            Screen::Playing
        } else {
            self.screen
        };
        let drawn = self.draw_screen_with_pre_hook(&mut batch);
        match screen {
            _ if !drawn => {}
            Screen::Paused if self.pause_menu => self.draw_pause_menu(&mut batch),
            // PauseScreen(false): only its title, at the top.
            Screen::Paused => {
                let title = self.translate_with_fallback("menu.paused", "Game Paused");
                horizon_client::widgets::begin_screen(|| {
                    horizon_client::widgets::ScreenInfo::new("paused", title.clone())
                });
                let tw = HudBatch::text_width(&self.gui.atlas, &title);
                batch.text(
                    &self.gui.atlas,
                    &title,
                    ((batch.width / 2.0).floor() - (tw / 2.0).floor()).floor(),
                    10.0,
                    u32::MAX,
                    true,
                );
            }
            Screen::GameModeSwitcher => self.draw_gamemode_switcher(&mut batch),
            Screen::Dead => self.draw_death_screen(&mut batch),
            Screen::Stats => self.draw_stats(&mut batch),
            Screen::Advancements => self.draw_advancements(&mut batch),
            Screen::Mod => {}
            Screen::WorldOptions => self.draw_world_options(&mut batch),
            Screen::Friends => self.draw_friends_screen(&mut batch),
            Screen::Social => self.draw_social_screen(&mut batch),
            Screen::ServerLinks => self.draw_server_links(&mut batch),
            Screen::ConfirmLink => self.draw_confirm_link(&mut batch),
            Screen::DialogWarning => self.draw_dialog_warning(&mut batch),
            Screen::CommandBlock => self.draw_command_block(&mut batch),
            Screen::BlockEditor => self.draw_block_editor(&mut batch),
            Screen::DebugOptions => self.draw_debug_options(&mut batch),
            Screen::HudEditor => self.draw_hud_editor(&mut batch),
            Screen::Inventory => {
                self.draw_inventory(&mut batch);
                self.draw_mod_browser(&mut batch);
            }
            Screen::Creative => {
                self.draw_creative(&mut batch);
            }
            Screen::Chat => self.draw_chat_screen(&mut batch),
            Screen::Playing | Screen::Options | Screen::Win => {}
        }
        // ScreenEvent.Render.Post: the open screen has drawn itself and its
        // tooltips, so a native mod draws over it here. Never `Playing`: the
        // prompts, books and sign editors drawn over gameplay keep it, draw
        // after this point and so over what a mod drew.
        self.draw_screen_with_post_hook(&mut batch, drawn);
        self.draw_saving_indicator(&mut batch);
        // ToastManager.extractRenderState: F1 hides the toasts too.
        self.draw_toasts(&mut batch, !show_hud);
        self.draw_tutorial_toasts(&mut batch, !show_hud);
        self.draw_pack_toasts(&mut batch, !show_hud);
        self.draw_music_toast(&mut batch, !show_hud);
        if self.server_packs.prompt.is_none() {
            self.remote_hud(&mut batch);
            self.draw_sign_editor(&mut batch);
            self.draw_book(&mut batch);
        }
        batch
    }
    /// The open screen as the render hooks see it, once a frame: not while
    /// playing with no screen, nor while the server pack prompt or a book
    /// hides the open screen (`hud_layers` draws `Playing` then), and None
    /// with no mod listening to a render hook, so nothing is built then.
    fn screen_render_view(&self, batch: &HudBatch) -> Option<mod_api::NativeScreenView<'_>> {
        let screen = self.screen;
        if screen == Screen::Playing
            || self.server_packs.prompt.is_some()
            || self.book.is_some()
            || !self.screen_render_listened()
        {
            return None;
        }
        // slot_count mirrors slots() without allocating; keep them in step.
        debug_assert_eq!(self.slot_count(), self.slots().len());
        Some(self.screen_view(batch, screen))
    }
    /// The title's own screen as the render hooks see it, once a frame: not
    /// when a screen of the game's own is over the title (Options draws and
    /// fires its own), nor while a dialog, a prompt or a connect or load
    /// hides it, and None with no mod listening to a render hook.
    fn title_screen_render_view(&self, batch: &HudBatch) -> Option<mod_api::NativeScreenView<'_>> {
        if self.screen != Screen::Playing
            || self.screen_over_playing()
            || self.remote_loading()
            || !self.screen_render_listened()
        {
            return None;
        }
        self.title_screen_view(batch)
    }
    /// `ScreenEvent.Render.Pre` then `Render.Background` for the open screen,
    /// once a frame, before it draws itself: whether it is drawn (a cancelled
    /// `Render.Pre` skips it and its Background). The Win and Options screens
    /// call this on their own path. Client only: what a mod draws is its own.
    fn draw_screen_with_pre_hook(&self, batch: &mut HudBatch) -> bool {
        match self.screen_render_view(batch) {
            Some(view) => self.screen_render_begin(batch, &view),
            None => true,
        }
    }
    /// `draw_screen_with_pre_hook` for the title's own screens.
    fn draw_title_screen_with_pre_hook(&self, batch: &mut HudBatch) -> bool {
        match self.title_screen_render_view(batch) {
            Some(view) => self.screen_render_begin(batch, &view),
            None => true,
        }
    }
    /// `ScreenEvent.Render.Foreground` (only when the screen `drawn`) and
    /// `Render.Post` for the open screen, once a frame, after it has drawn
    /// itself and its tooltips, so a native mod draws over it here. Never
    /// `Playing`: the prompts, books and sign editors drawn over gameplay keep
    /// it, draw after this point and so over what a mod drew.
    fn draw_screen_with_post_hook(&self, batch: &mut HudBatch, drawn: bool) {
        if let Some(view) = self.screen_render_view(batch) {
            self.screen_render_end(batch, &view, drawn);
        }
    }
    /// `draw_screen_with_post_hook` for the title's own screen, once a frame,
    /// after it has drawn and before the toasts.
    fn draw_title_screen_with_post_hook(&self, batch: &mut HudBatch, drawn: bool) {
        if let Some(view) = self.title_screen_render_view(batch) {
            self.screen_render_end(batch, &view, drawn);
        }
    }
    /// `CreativeModeInventoryScreen.hasPermissions` (`canUseGameMasterBlocks`
    /// and the Operator Items Tab option) given to `tryRefreshInvalidatedTabs`,
    /// as the screen's constructor and `containerTick` do.
    fn refresh_creative_tabs(&mut self) {
        let permissions = self.can_use_game_master_blocks() && self.settings.operator_items_tab;
        self.creative_screen
            .set_permissions(permissions, &self.pack.names);
    }
    fn creative_inventory_open(&self) -> bool {
        self.screen == Screen::Creative && self.creative_screen.tab().inventory
    }
    /// An item tab of the creative screen (its ItemPickerMenu).
    fn creative_tab_open(&self) -> bool {
        self.screen == Screen::Creative && !self.creative_screen.tab().inventory
    }
    fn inventory_origin(&self, batch: &HudBatch) -> (f32, f32) {
        if self.screen == Screen::Creative {
            return Self::creative_origin(batch);
        }
        let width = self.native_screen_width().unwrap_or(176.0);
        let (x, y) = batch.centered_origin(width, self.screen_height());
        (x + self.recipe_book_shift(batch.width), y)
    }
    /// Slot ids: 0..36 player inventory, 100..109 crafting grid, 200 result.
    fn container_kind(&self) -> Option<&BlockEntity> {
        self.container
            .and_then(|k| self.world.block_entities.get(&k))
    }
    /// 300.. are the open container's slots.
    fn slots(&self) -> Vec<usize> {
        if self.creative_inventory_open() {
            return Creative::inventory_slots().collect();
        }
        if self.creative_tab_open() {
            return Creative::tab_slots().collect();
        }
        if let Some(v) = self.station_slots().or_else(|| self.mount_slots()) {
            return v;
        }
        let mut v: Vec<usize> = (0..36).collect();
        if let Some(shown) = self.native_screen_slots() {
            v.extend(shown);
            return v;
        }
        match self.container_kind() {
            Some(BlockEntity::Chest(_)) => {
                v.extend((0..self.open_container_size()).map(|i| 300 + i))
            }
            Some(BlockEntity::Furnace(_)) => v.extend(300..303),
            Some(BlockEntity::Brewing(_)) => v.extend(300..305),
            Some(BlockEntity::Crafter(_)) => v.extend(300..310),
            Some(
                BlockEntity::Sign(_)
                | BlockEntity::Jukebox(_)
                | BlockEntity::Beacon(_)
                | BlockEntity::Lectern(_)
                | BlockEntity::Sculk(_)
                | BlockEntity::Banner(_)
                | BlockEntity::CreakingHeart(_)
                | BlockEntity::Spawner(_)
                | BlockEntity::TrialSpawner(_)
                | BlockEntity::Vault(_)
                | BlockEntity::Beehive(_)
                | BlockEntity::CommandBlock(_)
                | BlockEntity::EndGateway(_)
                | BlockEntity::Skull(_)
                | BlockEntity::DaylightDetector
                | BlockEntity::Campfire(_)
                | BlockEntity::ChiseledBookshelf(_)
                | BlockEntity::Brushable(_)
                | BlockEntity::Shelf(_)
                | BlockEntity::Mod(_)
                | BlockEntity::PotentSulfur(_)
                | BlockEntity::StructureBlock(_)
                | BlockEntity::JigsawBlock(_)
                | BlockEntity::TestBlock(_)
                | BlockEntity::TestInstanceBlock(_),
            ) => {}
            None => {
                v.extend((0..self.craft_size * self.craft_size).map(|i| 100 + i));
                v.push(200);
                if self.craft_size == 2 {
                    v.extend(500..505);
                }
            }
        }
        v
    }
    /// How many slots `slots` lists, counted without building the list: the
    /// screen render hook asks once a frame and must not allocate. Same
    /// branches in the same order, so the two stay in step.
    fn slot_count(&self) -> usize {
        if self.creative_inventory_open() {
            return Creative::inventory_slots().count();
        }
        if self.creative_tab_open() {
            return Creative::tab_slots().count();
        }
        if let Some(n) = self
            .station_slot_count()
            .or_else(|| self.mount_slot_count())
        {
            return n;
        }
        let inventory = 36;
        if let Some(shown) = self.native_screen_slot_count() {
            return inventory + shown;
        }
        let container = match self.container_kind() {
            Some(BlockEntity::Chest(_)) => self.open_container_size(),
            Some(BlockEntity::Furnace(_)) => 3,
            Some(BlockEntity::Brewing(_)) => 5,
            Some(BlockEntity::Crafter(_)) => 10,
            Some(
                BlockEntity::Sign(_)
                | BlockEntity::Jukebox(_)
                | BlockEntity::Beacon(_)
                | BlockEntity::Lectern(_)
                | BlockEntity::Sculk(_)
                | BlockEntity::Banner(_)
                | BlockEntity::CreakingHeart(_)
                | BlockEntity::Spawner(_)
                | BlockEntity::TrialSpawner(_)
                | BlockEntity::Vault(_)
                | BlockEntity::Beehive(_)
                | BlockEntity::CommandBlock(_)
                | BlockEntity::EndGateway(_)
                | BlockEntity::Skull(_)
                | BlockEntity::DaylightDetector
                | BlockEntity::Campfire(_)
                | BlockEntity::ChiseledBookshelf(_)
                | BlockEntity::Brushable(_)
                | BlockEntity::Shelf(_)
                | BlockEntity::Mod(_)
                | BlockEntity::PotentSulfur(_)
                | BlockEntity::StructureBlock(_)
                | BlockEntity::JigsawBlock(_)
                | BlockEntity::TestBlock(_)
                | BlockEntity::TestInstanceBlock(_),
            ) => 0,
            None => {
                self.craft_size * self.craft_size + 1 + if self.craft_size == 2 { 5 } else { 0 }
            }
        };
        inventory + container
    }
    /// InventoryMenu / CraftingMenu slot layouts.
    fn slot_position(&self, i: usize) -> (f32, f32) {
        if self.creative_inventory_open()
            && let Some(position) = Creative::inventory_slot_position(i)
        {
            return position;
        }
        if self.creative_tab_open()
            && let Some(position) = Creative::tab_slot_position(i)
        {
            return position;
        }
        if let Some(p) = self
            .station_slot_position(i)
            .or_else(|| {
                Self::stock_machine_slot(self.machine_slot_position(i), self.native_screen_shown())
            })
            .or_else(|| self.crafter_slot_position(i))
            .or_else(|| self.mount_slot_position(i))
        {
            return p;
        }
        let table = self.craft_size == 3;
        // ContainerScreen (3 rows) shifts the player inventory down by one pixel.
        // ChestMenu: the inventory sits `18 * (rows - 3) + 1` lower than the
        // 3-row layout of the other menus.
        let shift = match (self.chest_rows(), self.native_screen_inventory_y()) {
            (_, Some(y)) => y - 84.0,
            (Some(rows), None) => 1.0 + (rows as f32 - 3.0) * 18.0,
            (None, None) => 0.0,
        };
        let inventory_x = self.native_screen_inventory_x().unwrap_or(8.0);
        match i {
            0..9 => (inventory_x + i as f32 * 18.0, 142.0 + shift),
            9..36 => (
                inventory_x + ((i - 9) % 9) as f32 * 18.0,
                84.0 + shift + ((i - 9) / 9) as f32 * 18.0,
            ),
            500..504 => (8.0, 8.0 + (i - 500) as f32 * 18.0),
            // InventoryMenu: the shield slot.
            504 => (77.0, 62.0),
            300..500 if let Some(p) = self.native_screen_slot(i - 300) => p,
            300..500 => match self.container_kind() {
                Some(BlockEntity::Furnace(_)) => {
                    [(56.0, 17.0), (56.0, 53.0), (116.0, 35.0)][(i - 300).min(2)]
                }
                _ => (
                    8.0 + ((i - 300) % 9) as f32 * 18.0,
                    18.0 + ((i - 300) / 9) as f32 * 18.0,
                ),
            },
            200 => {
                if table {
                    (124.0, 35.0)
                } else {
                    (154.0, 28.0)
                }
            }
            _ => {
                let g = i - 100;
                let (r, c) = (g / self.craft_size, g % self.craft_size);
                if table {
                    (30.0 + c as f32 * 18.0, 17.0 + r as f32 * 18.0)
                } else {
                    (98.0 + c as f32 * 18.0, 18.0 + r as f32 * 18.0)
                }
            }
        }
    }
    /// The stock hopper/dispenser slot layout, unless a mod screen is shown for
    /// that menu: a mod screen places its own container slots
    /// (`native_screen_slot`) and the player's inventory (`native_screen_inventory_y`),
    /// and this layout answers first in `slot_position`, which kept a mod's screen
    /// on a hopper or a dispenser on the stock grid (#5141). The rest of those
    /// menus is already the mod's: `draw_native_screen` owns the background and
    /// `screen_height` its height.
    fn stock_machine_slot(stock: Option<(f32, f32)>, mod_screen_shown: bool) -> Option<(f32, f32)> {
        if mod_screen_shown { None } else { stock }
    }

    fn slot_at(&self, batch: &HudBatch, x: f32, y: f32) -> Option<usize> {
        if self.book_covers_menu(batch.width) {
            return None;
        }
        let (ox, oy) = self.inventory_origin(batch);
        self.slots().into_iter().find(|&i| {
            let (sx, sy) = self.slot_position(i);
            // AbstractContainerScreen.isHovering: the 16x16 slot grows by one
            // pixel on every side, so the gaps between slots still hover one.
            x >= ox + sx - 1.0 && x < ox + sx + 17.0 && y >= oy + sy - 1.0 && y < oy + sy + 17.0
        })
    }
    /// `AbstractContainerScreen.extractSlotHighlightBack` and
    /// `AbstractContainerScreen.extractSlotHighlightFront`: the 24 x 24
    /// `container/slot_highlight_back` sprite behind the slot at `x`, `y` (its
    /// 16 x 16 item box), the `_front` one over its item.
    fn draw_slot_highlight(&self, batch: &mut HudBatch, front: bool, x: f32, y: f32) {
        let sprite = if front {
            "gui/sprites/container/slot_highlight_front"
        } else {
            "gui/sprites/container/slot_highlight_back"
        };
        horizon_client::ui::blit_sprite(
            batch,
            &self.gui.atlas,
            sprite,
            x - 4.0,
            y - 4.0,
            24.0,
            24.0,
            u32::MAX,
        );
    }
    /// The highlight of the slot under the cursor, for the screens whose
    /// slots all sit in `slot_at`'s grid. `Slot.isHighlightable` is false
    /// for the crafter's result (`NonInteractiveResultSlot`).
    fn draw_hovered_slot_highlight(&self, batch: &mut HudBatch, front: bool, ox: f32, oy: f32) {
        let Some(slot) = self.slot_at(batch, self.cursor.0, self.cursor.1) else {
            return;
        };
        if slot == 309 && matches!(self.container_kind(), Some(BlockEntity::Crafter(_))) {
            return;
        }
        let (sx, sy) = self.slot_position(slot);
        self.draw_slot_highlight(batch, front, ox + sx, oy + sy);
    }
    fn slot_mut(&mut self, i: usize) -> &mut Option<ItemStack> {
        if i >= mount_menu::SADDLE {
            return self.mount_slot_mut(i);
        }
        if i >= stations::INPUT {
            return self.station_slot_mut(i);
        }
        if (500..504).contains(&i) {
            return &mut self.armor[i - 500];
        }
        if i == 504 {
            return &mut self.offhand;
        }
        if i >= 300 {
            return self.container_slot_mut(i - 300);
        }
        if (100..109).contains(&i) {
            &mut self.craft_grid[i - 100]
        } else {
            &mut self.inventory[i]
        }
    }
    /// What a slot shows: the crafting result slots are computed.
    fn slot_view(&self, i: usize) -> Option<ItemStack> {
        match i {
            200 => self.craft_result(),
            309 if matches!(self.container_kind(), Some(BlockEntity::Crafter(_))) => {
                self.crafter_result()
            }
            i if i >= mount_menu::SADDLE => self.mount_slot(i).clone(),
            i if i >= stations::INPUT => self.station_slot(i).clone(),
            500..=503 => self.armor[i - 500].clone(),
            504 => self.offhand.clone(),
            i if i >= 300 => self.container_slot(i - 300).clone(),
            100..=108 => self.craft_grid[i - 100].clone(),
            _ => self.inventory[i].clone(),
        }
    }
    fn craft_result(&self) -> Option<ItemStack> {
        // A server's result slot is the server's (`CraftingMenu.slotChangedCraftingGrid`).
        if let Some(r) = &self.remote {
            return r.craft_result.clone();
        }
        self.craft_recipe().map(|(_, result)| result)
    }
    /// The recipe id and result of the crafting grid.
    /// `RecipeCraftingHolder.awardUsedRecipes`: a crafted recipe becomes known
    /// (special ones never do).
    fn award_crafted(&mut self, crafts: &[(String, Vec<Option<ItemStack>>)]) {
        for (recipe, ingredients) in crafts {
            // The whole id: the advancements' `recipe_crafted` criteria name
            // `minecraft:x`, and the crafting copy names the recipe bare.
            let recipe = horizon_client::crafting::qualified(recipe);
            self.trigger_recipe_crafted(&recipe, ingredients);
        }
        let recipes: Vec<String> = crafts
            .iter()
            .map(|(r, _)| horizon_client::crafting::qualified(r))
            .collect();
        self.award_recipes(&recipes);
    }
    fn craft_recipe(&self) -> Option<(String, ItemStack)> {
        let n = self.craft_size;
        let grid: Vec<Option<ItemStack>> = (0..n * n).map(|i| self.craft_grid[i].clone()).collect();
        let scale = |id: i32| self.maps.data.get(&id).map(|m| m.scale);
        self.recipes()
            .find_with(&self.loop_context, &grid, n, &scale)
            .map(|(name, result)| (name.to_owned(), result))
    }
    fn draw_inventory_slots(&mut self, batch: &mut HudBatch, ox: f32, oy: f32) {
        let drag = self.drag_view();
        for i in self.slots() {
            let (sx, sy) = self.slot_position(i);
            if i == 504 && self.offhand.is_none() {
                batch.sprite(
                    &self.gui.atlas,
                    "gui/sprites/container/slot/shield",
                    ox + sx,
                    oy + sy,
                );
            }
            if (500..504).contains(&i) && self.armor[i - 500].is_none() {
                let icon = ["helmet", "chestplate", "leggings", "boots"][i - 500];
                batch.sprite(
                    &self.gui.atlas,
                    &format!("gui/sprites/container/slot/{icon}"),
                    ox + sx,
                    oy + sy,
                );
            }
            let stack = self.slot_view(i);
            self.draw_slot_item(batch, drag.as_ref(), i, &stack, ox + sx, oy + sy);
        }
    }
    fn draw_inventory(&mut self, batch: &mut HudBatch) {
        screen_info::begin_container("container");
        if self.draw_station(batch) {
            return;
        }
        let (ox, oy) = self.inventory_origin(batch);
        {
            let atlas = &self.gui.atlas;
            self.render_transparent_background("inventory", batch);
            let part = |batch: &mut HudBatch,
                        name: &str,
                        sx: f32,
                        sy: f32,
                        w: f32,
                        h: f32,
                        dx: f32,
                        dy: f32| {
                if let Some(mut r) = atlas.sprites.get(name).copied() {
                    let (du, dv) = ((r.u1 - r.u0) / r.w as f32, (r.v1 - r.v0) / r.h as f32);
                    r.u0 += sx * du;
                    r.v0 += sy * dv;
                    r.u1 = r.u0 + w * du;
                    r.v1 = r.v0 + h * dv;
                    batch.rect_uv(dx, dy, w, h, r, u32::MAX);
                }
            };
            let grey = rgba(64, 64, 64, 255);
            match self
                .container
                .and_then(|k| self.world.block_entities.get(&k))
            {
                Some(_) if self.draw_native_screen(batch, ox, oy) => {}
                Some(BlockEntity::Chest(_)) if self.draw_machine_screen(batch, ox, oy) => {}
                Some(BlockEntity::Chest(_)) => self.draw_chest_screen(batch, ox, oy),
                Some(BlockEntity::Furnace(f)) => {
                    let (tex, key, fallback) = match f.kind {
                        horizon_client::containers::FurnaceKind::Furnace => {
                            ("gui/container/furnace", "container.furnace", "Furnace")
                        }
                        horizon_client::containers::FurnaceKind::BlastFurnace => (
                            "gui/container/blast_furnace",
                            "container.blast_furnace",
                            "Blast Furnace",
                        ),
                        horizon_client::containers::FurnaceKind::Smoker => {
                            ("gui/container/smoker", "container.smoker", "Smoker")
                        }
                    };
                    part(batch, tex, 0.0, 0.0, 176.0, 166.0, ox, oy);
                    // A server's furnace menu has the title it was given.
                    let title = self.remote_menu_title().unwrap_or_else(|| {
                        horizon_client::text::Text::literal(
                            self.container_title(self.translate_with_fallback(key, fallback)),
                        )
                        .resolve_legacy()
                    });
                    screen_info::name_container(|| title.plain());
                    let tw = atlas.font.width_text(&title);
                    batch.text_styled(
                        atlas,
                        &title,
                        ox + (176.0 - tw) / 2.0,
                        oy + 6.0,
                        grey,
                        false,
                    );
                    let inventory =
                        self.translate_with_fallback("container.inventory", "Inventory");
                    batch.text(atlas, &inventory, ox + 8.0, oy + 72.0, grey, false);
                    if f.lit_remaining > 0 {
                        // AbstractFurnaceScreen: flame height ceil(lit * 13) + 1.
                        let total = if f.lit_total == 0 { 200 } else { f.lit_total };
                        let h = ((f.lit_remaining as f32 / total as f32) * 13.0).ceil() + 1.0;
                        part(
                            batch,
                            "gui/sprites/container/furnace/lit_progress",
                            0.0,
                            14.0 - h,
                            14.0,
                            h,
                            ox + 56.0,
                            oy + 36.0 + 14.0 - h,
                        );
                    }
                    if f.cooking_timer > 0 && f.cooking_total > 0 {
                        let w = ((f.cooking_timer as f32 / f.cooking_total as f32) * 24.0).ceil();
                        part(
                            batch,
                            "gui/sprites/container/furnace/burn_progress",
                            0.0,
                            0.0,
                            w,
                            16.0,
                            ox + 79.0,
                            oy + 34.0,
                        );
                    }
                }
                Some(BlockEntity::Brewing(_)) => {}
                Some(BlockEntity::Crafter(_)) => self.draw_crafter_screen(batch, ox, oy),
                Some(
                    BlockEntity::Sign(_)
                    | BlockEntity::Jukebox(_)
                    | BlockEntity::Beacon(_)
                    | BlockEntity::Lectern(_)
                    | BlockEntity::Sculk(_)
                    | BlockEntity::Banner(_)
                    | BlockEntity::CreakingHeart(_)
                    | BlockEntity::Spawner(_)
                    | BlockEntity::TrialSpawner(_)
                    | BlockEntity::Vault(_)
                    | BlockEntity::Beehive(_)
                    | BlockEntity::CommandBlock(_)
                    | BlockEntity::EndGateway(_)
                    | BlockEntity::Skull(_)
                    | BlockEntity::DaylightDetector
                    | BlockEntity::Campfire(_)
                    | BlockEntity::ChiseledBookshelf(_)
                    | BlockEntity::Brushable(_)
                    | BlockEntity::Shelf(_)
                    | BlockEntity::Mod(_)
                    | BlockEntity::PotentSulfur(_)
                    | BlockEntity::StructureBlock(_)
                    | BlockEntity::JigsawBlock(_)
                    | BlockEntity::TestBlock(_)
                    | BlockEntity::TestInstanceBlock(_),
                ) => {}
                None if self.mount_active() => self.draw_mount_background(batch, ox, oy),
                None => {
                    let texture = if self.craft_size == 3 {
                        "gui/container/crafting_table"
                    } else {
                        "gui/container/inventory"
                    };
                    part(batch, texture, 0.0, 0.0, 176.0, 166.0, ox, oy);
                    let title = self.translate_with_fallback("container.crafting", "Crafting");
                    screen_info::name_container(|| title.clone());
                    let tx = if self.craft_size == 3 { 29.0 } else { 97.0 };
                    batch.text(atlas, &title, ox + tx, oy + 6.0, grey, false);
                    if self.craft_size == 3 {
                        let inventory =
                            self.translate_with_fallback("container.inventory", "Inventory");
                        batch.text(atlas, &inventory, ox + 8.0, oy + 72.0, grey, false);
                    }
                }
            }
        }
        if self.container.is_none() && self.craft_size != 3 && !self.mount_active() {
            self.draw_inventory_player(batch, ox, oy);
        }
        self.draw_hovered_slot_highlight(batch, false, ox, oy);
        self.draw_inventory_slots(batch, ox, oy);
        self.draw_hovered_slot_highlight(batch, true, ox, oy);
        self.draw_sort_buttons(batch, ox, oy);
        self.draw_recipe_book(batch);
        let effect_tip = if self.container.is_none() && self.craft_size == 2 && !self.mount_active()
        {
            self.draw_inventory_effects(batch, (ox, oy), 176)
        } else {
            None
        };
        self.draw_carried(batch);
        self.draw_slot_tooltip(batch);
        self.draw_native_screen_tooltip(batch);
        if let Some(lines) = effect_tip {
            self.draw_tooltip_lines(batch, &lines, self.cursor.0, self.cursor.1);
        }
    }
    /// `AbstractContainerScreen` keeps `isQuickCrafting` and the drag it
    /// belongs to on the screen. Horizon keeps both on `Game` (`inv_clicks`,
    /// `inv_drag`, `last_inv_click`), so they are dropped when the pointer
    /// changes hands: a screen that opens or closes starts with its own
    /// state, and a press whose release never reached the screen -- the
    /// pointer left the window, or the screen closed first -- would
    /// otherwise leave every later press on every later screen swallowed.
    pub(super) fn reset_inventory_clicks(&mut self) {
        self.inv_clicks.cancel_drag();
        self.inv_drag = None;
        self.last_inv_click = None;
    }
    pub(super) fn close_container(&mut self) {
        // `Gui.setScreen` clears the old screen's focus before it goes: the
        // key that closed the container (Tab bound to the inventory) has
        // already moved the focus, and it must not outlive the screen.
        self.pad_clear_focus();
        self.mod_browser_closed();
        self.reset_inventory_clicks();
        self.close_station();
        self.mount_closed();
        self.recipe_ui.reset_screen();
        if self.remote.is_some() {
            return self.remote_close_container();
        }
        let mut returned: Vec<ItemStack> = self.carried.take().into_iter().collect();
        returned.extend(self.craft_grid.iter_mut().filter_map(Option::take));
        for stack in returned {
            if let Some(rest) = self.add_stack_to_inventory(stack)
                && let Some(eye) = self.eye_position()
            {
                // AbstractContainerMenu.dropOrPlaceInInventory: `drop(stack, false)`.
                self.throw_from_eyes(rest, eye, false, false);
            }
        }
        self.craft_size = 2;
        self.craft_table = None;
        if let Some(key) = self.container.take() {
            // A cart's or chest boat's items go back to it.
            self.close_cart_container(key);
            let last = self.container_opener_event(key, false);
            if last {
                self.post_block_game_event(
                    "minecraft:container_close",
                    [key.0, key.1, key.2],
                    Some(horizon_client::mobs::Who::Player),
                    None,
                );
                self.container_edge_sound(key, false);
            }
            self.world.modified.insert((key.0 >> 4, key.2 >> 4));
        }
        self.close_ender_chest();
    }
    /// `quickMoveStack` of the open plain menu for a shift-click.
    fn inventory_quick_move(&mut self, slot: usize) -> bool {
        self.click_open_inventory(Some(slot), 0, horizon_client::menu::Input::QuickMove)
    }
    /// `AbstractContainerMenu.clicked` on the open inventory, crafting table,
    /// chest, hopper, dispenser or furnace menu, run by the verified menu
    /// engine (`horizon_client::menu`). `slot` None is outside the window.
    /// False when the open screen is not one the engine models.
    fn click_open_inventory(
        &mut self,
        slot: Option<usize>,
        button: i32,
        input: horizon_client::menu::Input,
    ) -> bool {
        if let Some(slot) = slot
            && slot >= Creative::DISPLAY
            && self.creative_tab_open()
        {
            self.creative_display_click(slot, button, input);
            return true;
        }
        if let Some(slot) = slot
            && input == horizon_client::menu::Input::Throw
            && self.carried.is_some()
            && self.remote.is_some()
            && ((slot < 9 && self.creative_tab_open()) || self.creative_inventory_open())
        {
            self.creative_throw_carrying(slot, button);
            return true;
        }
        self.menu_ops(&[(slot, button, input)])
    }
    /// `CreativeModeInventoryScreen.slotClicked`: THROW while a stack is carried, on an item
    /// tab's hotbar slot or any slot of the Survival Inventory tab. The menu's own THROW needs an
    /// empty hand, so the screen drops from the slot itself: one item, or with the control key
    /// the whole stack (on the inventory tab `Slot.remove(getMaxStackSize())`), through
    /// `handleCreativeModeItemDrop`, then sends the slot (`broadcastChanges`,
    /// `handleCreativeModeItemAdd`); the carried stack stays. Only the server drops it: this is
    /// the client of a server (`remote`).
    fn creative_throw_carrying(&mut self, slot: usize, button: i32) {
        let Some(old) = self.slot_mut(slot).clone() else {
            return;
        };
        let count = if button == 0 {
            1
        } else if self.creative_inventory_open() {
            let max = world_items!(self)
                .get(&old.item)
                .map_or(64, |i| i.max_stack);
            old.count.min(max)
        } else {
            old.count
        };
        let before = self.player_equipment();
        let left = old.count - count;
        *self.slot_mut(slot) = (left > 0).then(|| ItemStack {
            count: left,
            ..old.clone()
        });
        self.player_equipment_changed(&before);
        self.apply_equipment();
        self.remote_creative_throw(ItemStack { count, ..old });
        self.send_creative_slots();
    }
    /// A click on an item tab's display slot: the screen's own handling
    /// (`creative::click_display`), never the menu's.
    fn creative_display_click(
        &mut self,
        slot: usize,
        button: i32,
        input: horizon_client::menu::Input,
    ) {
        use horizon_client::creative::{DisplayClick, click_display};
        if self.creative_screen.tab().search {
            // CreativeModeInventoryScreen.slotClicked: moveCursorToEnd, then
            // setHighlightPos(0), so typing replaces the whole query.
            let search = &mut self.creative_screen.search;
            search.move_cursor_to_end(false);
            search.highlight = 0;
        }
        let first = self.creative_screen.first_row(&self.pack.names);
        let shown = self
            .creative_screen
            .shown(&self.pack.names, first * 9 + slot - Creative::DISPLAY);
        let catalog = world_items!(self);
        let max = |s: &ItemStack| catalog.get(&s.item).map_or(64, |i| i.max_stack);
        match click_display(&mut self.carried, shown.as_ref(), max, button, input) {
            DisplayClick::Hotbar(i, stack) if i < self.inventory.len() => {
                self.inventory[i] = Some(stack);
            }
            DisplayClick::Offhand(stack) => self.offhand = Some(stack),
            // handleCreativeModeItemDrop: on a server the stack goes to it (SetCreativeModeSlot
            // to slot -1), which drops it; a local drop here would never reach the server.
            DisplayClick::Drop(stack) if self.remote.is_some() => self.remote_creative_throw(stack),
            DisplayClick::Drop(stack) => {
                self.player_drop(stack);
            }
            _ => {}
        }
        // handleCreativeModeItemAdd / broadcastChanges: a swapped-in hotbar slot goes out now.
        if self.remote.is_some() {
            self.send_creative_slots();
        }
    }
    /// Several clicks on one menu instance (a quick craft drag is three).
    fn menu_ops(&mut self, ops: &[(Option<usize>, i32, horizon_client::menu::Input)]) -> bool {
        use horizon_client::menu::Input;
        // BundleMouseActions.onSlotClicked: a Shift click or a swap clears the bundle's selection.
        for &(slot, _, input) in ops {
            if let (Some(slot), Input::QuickMove | Input::Swap) = (slot, input) {
                self.unselect_bundle(slot);
            }
        }
        if self.remote.is_some() && self.screen == Screen::Inventory {
            // On a server the clicks are the server's: predicted and sent.
            return self.remote_menu_ops(ops);
        }
        if let [(Some(slot), button, input)] = ops {
            self.crafter_slot_clicked(*slot, *button, *input);
        }
        let ops = ops.to_vec();
        let done = self.menu_session(move |menu, env, to_menu| {
            let mut clicks = Vec::new();
            for &(slot, button, input) in &ops {
                let menu_slot = match slot {
                    Some(g) => {
                        if g == 200 && input != Input::QuickMove {
                            return None;
                        }
                        match to_menu(g) {
                            Some(i) => i as i32,
                            None => return None,
                        }
                    }
                    None => -999,
                };
                clicks.push((menu_slot, button, input));
            }
            for (menu_slot, button, input) in clicks {
                if menu.click(env, menu_slot, button, input).is_err() {
                    // Vanilla's server throws: nothing of this click is kept.
                    return Some(false);
                }
            }
            Some(true)
        });
        // CreativeModeInventoryScreen.slotClicked: the changed slots go out with the click.
        if done && self.remote.is_some() && self.screen == Screen::Creative {
            self.send_creative_slots();
        }
        done
    }
    /// A sound a menu click made the player play (a bundle's insert or
    /// remove): `Entity.playSound` at the player.
    pub(super) fn player_menu_sound(&mut self, sound: &str) {
        let Some(p) = self.player.as_ref() else {
            return;
        };
        let at = [p.position.x, p.position.y, p.position.z];
        let (volume, pitch) = if sound == horizon_client::bundle::INSERT_FAIL {
            (1.0, 1.0)
        } else {
            (0.8, 0.8 + self.world.random.next_float() * 0.4)
        };
        if self.is_logical_server() {
            self.audio.play(sound, Some(at), volume, pitch);
        } else {
            self.play_client_player_sound(sound, volume, pitch);
        }
    }
    /// The open menu as the verified engine (`horizon_client::menu`): the
    /// game's slots are copied in, `run` acts on it, and the slots, carried
    /// stack, menu data and drops are copied back when it returns Some(true)
    /// (Some(false): handled, nothing kept; None: not handled). False when the
    /// open screen is not one the engine models or `run` did not handle it.
    pub(super) fn menu_session(
        &mut self,
        run: impl FnOnce(
            &mut horizon_client::menu::Menu,
            &horizon_client::menu::Env,
            &dyn Fn(usize) -> Option<usize>,
        ) -> Option<bool>,
    ) -> bool {
        use horizon_client::menu::{Env, Kind, Menu};
        #[derive(Clone, Copy)]
        struct Map {
            container: usize,
            table: bool,
            cells: usize,
            plain: bool,
        }
        impl Map {
            fn main(&self) -> usize {
                match (self.plain, self.table) {
                    (false, _) => self.container,
                    (true, true) => 10,
                    (true, false) => 9,
                }
            }
            fn to_menu(self, g: usize) -> Option<usize> {
                let main = self.main();
                if self.plain {
                    match g {
                        200 => Some(0),
                        100..109 if g - 100 < self.cells => Some(1 + g - 100),
                        500..504 if !self.table => Some(5 + g - 500),
                        504 if !self.table => Some(45),
                        9..36 => Some(main + g - 9),
                        0..9 => Some(main + 27 + g),
                        _ => None,
                    }
                } else {
                    match g {
                        300.. if g < 300 + self.container => Some(g - 300),
                        9..36 => Some(main + g - 9),
                        0..9 => Some(main + 27 + g),
                        _ => None,
                    }
                }
            }
            fn to_game(self, i: usize) -> Option<usize> {
                let main = self.main();
                if self.plain {
                    match i {
                        0 => None,
                        1..=9 if i - 1 < self.cells => Some(100 + i - 1),
                        5..9 if !self.table => Some(500 + i - 5),
                        45 if !self.table => Some(504),
                        _ if i >= main && i < main + 27 => Some(9 + i - main),
                        _ if i >= main + 27 && i < main + 36 => Some(i - main - 27),
                        _ => None,
                    }
                } else if i < self.container {
                    Some(300 + i)
                } else if i < self.container + 27 {
                    Some(9 + i - self.container)
                } else {
                    Some(i - self.container - 27)
                }
            }
        }
        // Menu slot -> game slot, for every menu slot.
        let mut plain_map = None;
        let crafter_open = matches!(self.container_kind(), Some(BlockEntity::Crafter(_)));
        let loom_open = self
            .station
            .as_ref()
            .is_some_and(|s| s.kind == stations::Kind::Loom);
        let merchant_open = self
            .station
            .as_ref()
            .is_some_and(|s| s.kind == stations::Kind::Merchant);
        let stonecutter_open = self
            .station
            .as_ref()
            .is_some_and(|s| s.kind == stations::Kind::Stonecutter);
        let station_kind = self.station.as_ref().map(|s| s.kind);
        let grindstone_open = station_kind == Some(stations::Kind::Grindstone);
        let cartography_open = station_kind == Some(stations::Kind::Cartography);
        let anvil_open = station_kind == Some(stations::Kind::Anvil);
        let smithing_open = station_kind == Some(stations::Kind::Smithing);
        let enchanting_open = station_kind == Some(stations::Kind::Enchanting);
        let beacon_open = station_kind == Some(stations::Kind::Beacon);
        let (kind, game_of): (Kind, Vec<Option<usize>>) = if self.creative_tab_open() {
            // ItemPickerMenu: the display slots (clicked by the screen), then the hotbar.
            let mut v = vec![None; 45];
            v.extend((0..9).map(Some));
            (Kind::ItemPicker, v)
        } else if let Some(columns) = self
            .mount_menu_columns()
            .filter(|_| self.remote.is_some() && self.mount_active())
        {
            // A server's HorseInventoryMenu: the saddle, the body armour,
            // the chest, then the inventory.
            let mut v = vec![Some(mount_menu::SADDLE), Some(mount_menu::ARMOR)];
            v.extend((0..columns * 3).map(|i| Some(mount_menu::CHEST + i)));
            v.extend((9..36).chain(0..9).map(Some));
            (Kind::Mount(columns), v)
        } else if merchant_open {
            // MerchantMenu: two payment slots, the result, then the inventory.
            let mut v: Vec<Option<usize>> = vec![
                Some(stations::INPUT),
                Some(stations::INPUT + 1),
                Some(stations::RESULT),
            ];
            v.extend((9..36).chain(0..9).map(Some));
            (Kind::Merchant, v)
        } else if crafter_open {
            // CrafterMenu: the grid, the inventory, then the result.
            let mut v: Vec<Option<usize>> = (300..309).map(Some).collect();
            v.extend((9..36).chain(0..9).map(Some));
            v.push(None);
            (Kind::CrafterBlock, v)
        } else if loom_open {
            // LoomMenu: banner, dye, pattern, result, then the inventory.
            let mut v: Vec<Option<usize>> = (0..3).map(|i| Some(stations::INPUT + i)).collect();
            v.push(Some(stations::RESULT));
            v.extend((9..36).chain(0..9).map(Some));
            (Kind::Loom, v)
        } else if stonecutter_open {
            // StonecutterMenu: the input, the result, then the inventory.
            let mut v = vec![Some(stations::INPUT), Some(stations::RESULT)];
            v.extend((9..36).chain(0..9).map(Some));
            (Kind::Stonecutter, v)
        } else if grindstone_open || cartography_open {
            // GrindstoneMenu and CartographyTableMenu: the two inputs, the
            // result, then the inventory.
            let mut v = vec![
                Some(stations::INPUT),
                Some(stations::INPUT + 1),
                Some(stations::RESULT),
            ];
            v.extend((9..36).chain(0..9).map(Some));
            let kind = if grindstone_open {
                Kind::Grindstone
            } else {
                Kind::Cartography
            };
            (kind, v)
        } else if enchanting_open {
            // EnchantmentMenu: the item, the lapis, then the inventory.
            let mut v = vec![Some(stations::INPUT), Some(stations::INPUT + 1)];
            v.extend((9..36).chain(0..9).map(Some));
            (Kind::Enchanting, v)
        } else if beacon_open {
            // BeaconMenu: the payment, then the inventory.
            let mut v = vec![Some(stations::INPUT)];
            v.extend((9..36).chain(0..9).map(Some));
            (Kind::Beacon, v)
        } else if anvil_open || smithing_open {
            // ItemCombinerMenu: the inputs, the result, then the inventory.
            let inputs = if anvil_open { 2 } else { 3 };
            let mut v: Vec<Option<usize>> =
                (0..inputs).map(|i| Some(stations::INPUT + i)).collect();
            v.push(Some(stations::RESULT));
            v.extend((9..36).chain(0..9).map(Some));
            (
                if anvil_open {
                    Kind::Anvil
                } else {
                    Kind::Smithing
                },
                v,
            )
        } else {
            let (kind, map) = if self.container.is_some() {
                let size = self.open_container_size();
                let (kind, count) = match self.container_kind() {
                    Some(BlockEntity::Chest(_)) => {
                        let shulker = self.container.is_some_and(|k| {
                            let id = self.world.block(k.0, k.1, k.2);
                            storage::is_shulker_box(&self.res.facts().get(id).name)
                        });
                        let kind = match size {
                            5 => Kind::Hopper,
                            9 => Kind::Dispenser,
                            27 if shulker => Kind::Shulker,
                            n => Kind::Chest((n / 9).clamp(1, 6)),
                        };
                        (kind, size)
                    }
                    Some(BlockEntity::Brewing(_)) => (Kind::Brewing, 5),
                    Some(BlockEntity::Furnace(f)) => (
                        Kind::Furnace(
                            match f.kind.recipe_type().trim_start_matches("minecraft:") {
                                "blasting" => "blasting",
                                "smoking" => "smoking",
                                _ => "smelting",
                            },
                        ),
                        3,
                    ),
                    _ => return false,
                };
                let map = Map {
                    container: count,
                    table: false,
                    cells: 0,
                    plain: false,
                };
                (kind, map)
            } else if self.station.is_none() && !self.mount_active() {
                let table = self.craft_size == 3;
                let map = Map {
                    container: 0,
                    table,
                    cells: self.craft_size * self.craft_size,
                    plain: true,
                };
                (
                    if table {
                        Kind::Crafting
                    } else {
                        Kind::Inventory
                    },
                    map,
                )
            } else {
                return false;
            };
            let total = Menu::new(kind).slot_count();
            plain_map = Some(map);
            (kind, (0..total).map(|i| map.to_game(i)).collect())
        };
        // The plain map also takes the crafting result slot (a shift-click on it).
        let to_menu = |g: usize| match plain_map {
            Some(map) => map.to_menu(g),
            None => game_of.iter().position(|x| *x == Some(g)),
        };
        let mut menu = Menu::new(kind);
        let menu_id = if matches!(kind, Kind::Mount(_)) {
            Some("horizon:mount_inventory")
        } else if matches!(kind, Kind::Inventory | Kind::ItemPicker) {
            None
        } else {
            self.remote_menu_type()
        };
        menu.set_registry_id(menu_id);
        menu.brewed_potion_hook = !mod_api::__private::mods().is_empty();
        if matches!(kind, Kind::Mount(_)) {
            menu.mount = self.mount_menu_entity();
        }
        let total = menu.slot_count();
        let before: Vec<Option<ItemStack>> = game_of
            .iter()
            .map(|g| g.and_then(|g| self.slot_mut(g).clone()))
            .collect();
        let carried = self.carried.clone();
        let creative = self.creative;
        // CreativeModeInventoryScreen clicks the menu itself and sends every
        // changed slot, which the server sets by player.
        let equipment_before = (self.screen == Screen::Creative).then(|| self.player_equipment());
        let station_menu = self.station.as_ref().map(|s| {
            (
                s.loom.selected.map_or(-1, |i| i as i32),
                s.loom.patterns.clone(),
            )
        });
        let stone = self
            .station
            .as_ref()
            .map(|s| (s.selected.map_or(-1, |i| i as i32), s.stone.clone()));
        let crafter_data = match self.container_kind() {
            Some(BlockEntity::Crafter(c)) => Some((c.disabled, c.triggered)),
            _ => None,
        };
        // The merchant's session, lent to the menu for this click.
        let mut session = self
            .station
            .as_ref()
            .and_then(|s| s.merchant.as_ref())
            .map(|m| (m.session.clone(), m.mob));
        if let Some((s, mob)) = session.as_mut() {
            self.merchant_sync_in(s, *mob);
        }
        // The cartography table reads and makes the level's maps; a
        // server's (the client menu's ContainerLevelAccess.NULL) doesn't.
        let level_maps = (kind == Kind::Cartography && self.remote.is_none()).then(|| {
            let held: Vec<ItemStack> = before
                .iter()
                .flatten()
                .chain(self.inventory.iter().flatten())
                .chain(self.carried.iter())
                .cloned()
                .collect();
            self.level_maps(&held)
        });
        let next_map = if merchant_open { self.peek_map_id() } else { 0 };
        let tag = |t: &str| self.structure_tag(t);
        let maps = trading::GameMaps {
            terrain: self.end.locator(),
            structure_tag: &tag,
            made: Default::default(),
            next_id: std::cell::Cell::new(next_map),
        };
        let trade_env = horizon_client::merchant::TradeEnv {
            trades: &self.stations().trades,
            enchantments: &self.stations().enchantments,
            maps: Some(&maps),
        };
        let env = Env {
            player: self
                .lan_acting_uuid
                .or_else(|| self.remote.as_ref().map(|r| r.profile())),
            player_entity_id: Some(self.own_entity_id()),
            loop_context: &self.loop_context,
            items: self.items(),
            recipes: self.recipes(),
            brewing: &self.stations().brewing,
            banners: &self.stations().banners,
            station_recipes: &self.stations().recipes,
            enchantments: &self.stations().enchantments,
            names: &self.pack.names,
            trade: Some(&trade_env),
            maps: level_maps
                .as_ref()
                .map(|m| m as &dyn horizon_client::menu::Maps),
            limited_crafting: self.limited_crafting(),
            known_recipes: Some(&self.level.recipe_book),
        };
        menu.creative = creative;
        menu.selected = self.selected;
        menu.levels = self.experience.level;
        if anvil_open {
            // The rename box as AnvilScreen.onNameChanged gives it to the menu.
            menu.anvil.name = self.station.as_ref().map(|s| self.anvil_item_name(s));
        }
        for (i, stack) in before.iter().enumerate() {
            menu.set_slot(&env, i, stack.clone());
        }
        // The off hand is part of the inventory even when the menu has no slot for it.
        let offhand_mapped = game_of.contains(&Some(504));
        if !offhand_mapped {
            menu.set_player_item(40, self.offhand.clone());
        }
        menu.settle(&env);
        if kind == Kind::Loom
            && let Some((selected, patterns)) = station_menu
        {
            // The menu's state between clicks: what set_slot recomputed is replaced.
            menu.data[0] = selected;
            menu.loom_patterns = patterns;
            menu.set_slot(&env, 3, before[3].clone());
        }
        if kind == Kind::Stonecutter
            && let Some((selected, state)) = stone
        {
            // The same: the list and selection between clicks, then the result.
            menu.data[0] = selected;
            menu.stone = state;
            menu.set_slot(&env, 1, before[1].clone());
        }
        if anvil_open && let Some(s) = self.station.as_ref() {
            // The same: the cost, material cost and rename flag between clicks, as
            // the station's createResult and the mods' anvil_update left them (the
            // result came with the slots), so a take is predicted as the server makes it.
            menu.data[0] = s.anvil.cost;
            menu.anvil.repair_count = s.anvil.repair_item_count_cost;
            menu.anvil.only_renaming = s.anvil.only_renaming;
        }
        if kind == Kind::Enchanting
            && let Some(s) = self.station.as_ref()
        {
            // The offers between clicks (what copying the slots in flagged is not a change).
            menu.enchant_changed = false;
            for i in 0..3 {
                menu.data[i] = s.offers.costs[i];
                menu.data[7 + i] = s.offers.clues[i].as_ref().map_or(-1, |(_, l)| *l as i32);
            }
            menu.data[3] = self.level.xp_seed;
        }
        if let Some((disabled, triggered)) = crafter_data {
            for (i, d) in disabled.iter().enumerate() {
                menu.data[i] = i32::from(*d);
            }
            menu.data[9] = i32::from(triggered);
        }
        menu.carried = carried;
        menu.drops.clear();
        if kind == Kind::Merchant {
            // Attached after the slots were set: setting them is not a click.
            menu.merchant = session.map(|(s, _)| s);
        }
        // `FurnaceResultSlot.checkTakeAchievements` pays out on what left the output slot.
        let furnace_before = if matches!(kind, Kind::Furnace(_)) {
            menu.slot(2).cloned().map_or(0, |s| s.count)
        } else {
            0
        };
        let outcome = run(&mut menu, &env, &to_menu);
        let made = maps.made.take();
        match outcome {
            None => return false,
            Some(false) => return true,
            Some(true) => {}
        }
        // On a server the clicks only predict: the server drops, crafts and
        // makes maps itself.
        let predicting = self.remote.is_some();
        if !predicting {
            self.register_exploration_maps(made);
            for (item, count) in std::mem::take(&mut menu.crafted_counts) {
                self.award_stat("crafted", &item, count);
            }
        }
        if let Some(s) = menu.merchant.take() {
            self.merchant_sync_out(s);
        }
        let after: Vec<Option<ItemStack>> = (0..total).map(|i| menu.slot(i).cloned()).collect();
        // Only a furnace has a result slot to compare: an enchanting table has two slots.
        let furnace_after = if matches!(kind, Kind::Furnace(_)) {
            after[2].as_ref().map_or(0, |s| s.count)
        } else {
            0
        };
        let carried = menu.carried.take();
        let dropped = std::mem::take(&mut menu.dropped);
        let sounds = std::mem::take(&mut menu.sounds);
        // A bundle's sounds are the player's own (`Entity.playSound`): the
        // client plays them as it clicks, predicting or not.
        for sound in std::mem::take(&mut menu.player_sounds) {
            self.player_menu_sound(sound);
        }
        let crafted = std::mem::take(&mut menu.crafted);
        let brewed = std::mem::take(&mut menu.brewed);
        let brewed_potion_takes = std::mem::take(&mut menu.brewed_potion_takes);
        let equipped = std::mem::take(&mut menu.equipped);
        let item_crafted = std::mem::take(&mut menu.item_crafted);
        for (_, stack) in &item_crafted {
            self.tutorial_get_item(Some(stack));
        }
        for (i, stack) in after.into_iter().enumerate() {
            if let Some(g) = game_of[i] {
                *self.slot_mut(g) = stack;
            }
        }
        self.carried = carried;
        if !offhand_mapped {
            self.offhand = menu.player_item(40).cloned();
        }
        if !predicting {
            if !brewed_potion_takes.is_empty() {
                let (player, entity_id) = (self.native_acting_id(), self.own_entity_id());
                for stack in &brewed_potion_takes {
                    self.native_player_brewed_potion(player, entity_id, stack);
                }
            }
            for stack in dropped {
                self.player_drop(stack);
            }
            self.award_crafted(&crafted);
            for stack in &brewed {
                self.trigger_brewed_potion(stack);
            }
            if furnace_before > furnace_after
                && self.remote.is_none()
                && let (Some(pos), Some(p)) = (self.container, self.player.as_ref())
            {
                let at = [p.position.x, p.position.y, p.position.z];
                self.award_furnace_recipes(pos, at);
            }
        } else if self.screen == Screen::Creative {
            // A THROW in the creative screen with an empty hand: on an item tab's hotbar
            // `AbstractContainerMenu.doClick` calls `Player.handleCreativeModeItemDrop`, on the
            // Survival Inventory tab the screen does. The stack goes to the server, the slot
            // follows with the others (SetCreativeModeSlot). Only the server can drop it; the
            // slot has already given it up.
            for stack in dropped {
                self.remote_creative_throw(stack);
            }
        }
        if kind == Kind::Loom {
            if let Some(s) = self.station.as_mut() {
                s.loom.selected = usize::try_from(menu.data[0]).ok();
                s.loom.patterns = std::mem::take(&mut menu.loom_patterns);
            }
            // A server's loom plays its own sounds (the client's menu has no level).
            for sound in sounds.into_iter().filter(|_| !predicting) {
                self.station_sound(sound);
            }
            self.loom_screen_changed();
        } else if kind == Kind::Cartography {
            for sound in sounds.into_iter().filter(|_| !predicting) {
                self.station_sound(sound);
            }
        } else if kind == Kind::Stonecutter {
            if let Some(s) = self.station.as_mut() {
                s.selected = usize::try_from(menu.data[0]).ok();
                s.stone = std::mem::take(&mut menu.stone);
                s.stonecutter_changed();
            }
            for sound in sounds.into_iter().filter(|_| !predicting) {
                self.station_sound(sound);
            }
        }
        if let Some(amount) = menu.ground.take().filter(|_| !predicting)
            && let Some(pos) = self.station.as_ref().map(|s| s.pos)
        {
            self.grindstone_taken(pos, amount);
        }
        if kind == Kind::Anvil || kind == Kind::Smithing {
            self.combiner_session_end(&menu, predicting);
        }
        if kind == Kind::Enchanting && (menu.enchant_changed || before[0] != menu.slot(0).cloned())
        {
            // EnchantmentMenu.slotsChanged: new offers for the item.
            self.station_changed();
        }
        if kind == Kind::CrafterBlock
            && let Some(BlockEntity::Crafter(c)) = self
                .container
                .and_then(|k| self.world.block_entities.get_mut(&k))
        {
            for (i, d) in c.disabled.iter_mut().enumerate() {
                *d = menu.data[i] == 1;
            }
        }
        if matches!(kind, Kind::Furnace(_))
            && let Some(BlockEntity::Furnace(f)) = self
                .container
                .and_then(|k| self.world.block_entities.get_mut(&k))
        {
            // AbstractFurnaceBlockEntity.setItem: only a new input restarts cooking.
            f.input_clicked(&self.loop_context, world_recipes!(self), before[0].clone());
        }
        match equipment_before {
            Some(before) => self.player_equipment_changed(&before),
            None => {
                for (slot, stack) in equipped {
                    self.player_equipped(slot, stack.as_ref());
                }
            }
        }
        self.apply_equipment();
        if let Some(maps) = level_maps {
            self.register_made_maps(maps);
        }
        true
    }
    /// `CreativeModeInventoryScreen.keyPressed` on the search tab: a hotbar
    /// key over a slot swaps before the search box sees it.
    fn search_tab_hotbar_key(&mut self, code: KeyCode) -> bool {
        use horizon_client::creative::{Hover, search_key_swaps};
        let numeric = HOTBAR_DIGITS.contains(&code) || NUMERIC_KEYS.contains(&code);
        if !numeric {
            return false;
        }
        let Some(key) = self.settings.keys.container_hotbar_key(code) else {
            return false;
        };
        let Some(size) = self
            .renderer
            .as_ref()
            .map(|r| [r.config.width, r.config.height])
        else {
            return false;
        };
        let batch = HudBatch::new(size, self.settings.gui_scale_inputs());
        let hover = match self.slot_at(&batch, self.cursor.0, self.cursor.1) {
            None => Hover::Nothing,
            Some(slot) if slot >= Creative::DISPLAY => {
                let first = self.creative_screen.first_row(&self.pack.names);
                Hover::Display {
                    filled: self
                        .creative_screen
                        .shown(&self.pack.names, first * 9 + slot - Creative::DISPLAY)
                        .is_some(),
                }
            }
            Some(_) => Hover::Slot,
        };
        search_key_swaps(numeric, true, hover, self.carried.is_some()) && self.inventory_hotkey(key)
    }
    /// Number keys, F and Q over a slot of the open menu: SWAP with a hotbar
    /// slot or the off hand, THROW one or the whole stack.
    fn inventory_hotkey(&mut self, key: KeyCode) -> bool {
        use horizon_client::menu::Input;
        let Some(size) = self
            .renderer
            .as_ref()
            .map(|r| [r.config.width, r.config.height])
        else {
            return false;
        };
        let batch = HudBatch::new(size, self.settings.gui_scale_inputs());
        let Some(slot) = self.slot_at(&batch, self.cursor.0, self.cursor.1) else {
            return false;
        };
        let digit = HOTBAR_DIGITS.iter().position(|k| *k == key);
        let done = match (digit, key) {
            (Some(n), _) => self.click_open_inventory(Some(slot), n as i32, Input::Swap),
            (None, KeyCode::KeyF) => self.click_open_inventory(Some(slot), 40, Input::Swap),
            (None, KeyCode::KeyQ) => {
                let all = drop_whole_stack(self.modifiers);
                self.click_open_inventory(Some(slot), i32::from(all), Input::Throw)
            }
            _ => false,
        };
        if done && let Some(key) = self.container {
            self.container_changed(key);
        }
        done
    }
    /// The pick block key bound to a keyboard key, over a slot of the open
    /// menu (`AbstractContainerScreen.keyPressed`): `CLONE` with button 0. The
    /// search tab's box takes the key instead.
    fn inventory_pick_key(&mut self) {
        let menu = (self.screen == Screen::Inventory
            || (self.screen == Screen::Creative && !self.creative_screen.tab().search))
            && !self.dialog_open();
        let Some(size) = self
            .renderer
            .as_ref()
            .map(|r| [r.config.width, r.config.height])
        else {
            return;
        };
        let batch = HudBatch::new(size, self.settings.gui_scale_inputs());
        let Some(slot) = self
            .slot_at(&batch, self.cursor.0, self.cursor.1)
            .filter(|_| menu)
        else {
            return;
        };
        if self.click_open_inventory(Some(slot), 0, horizon_client::menu::Input::Clone)
            && let Some(key) = self.container
        {
            self.container_changed(key);
        }
    }
    /// The plain inventory mouse press: starts a quick craft drag when a
    /// stack is held, gathers on a double click, else the normal click.
    fn inventory_press(&mut self, right: bool) {
        use horizon_client::container_screen::Target;
        use horizon_client::menu::Input;
        if self.mod_browser_press(right) {
            return;
        }
        if !right && self.sort_press() {
            return;
        }
        if self.native_screen_press(right) {
            self.inv_clicks.skip_release();
            return;
        }
        // AbstractRecipeBookScreen lets the book handle its buttons before
        // AbstractContainerScreen starts a drag or chooses an outside slot.
        // Toggling it can move the inventory under the pointer before release.
        if self.recipe_book_click(right) {
            self.inv_clicks.skip_release();
            return;
        }
        let (slot, outside) = self.inventory_target();
        let shift = self.has_shift_down();
        let target = Target::of(slot, outside);
        let input = self.inv_clicks.press(
            &target,
            Self::screen_button(right),
            self.carried.is_none(),
            shift,
            false,
        );
        if outside {
            // slotId -999: an empty hand throws nothing; a carried stack
            // starts a drag whose release drops it.
            match input {
                Some(input) => self.throw_click(right, input),
                None if self.inv_clicks.quick_crafting().is_some() => {
                    self.inv_drag = Some((i32::from(right), Vec::new()));
                }
                None => {}
            }
            return;
        }
        if self.tweak_press(slot, right, shift) {
            return;
        }
        if let Some(slot) = slot
            && shift
            && slot != 200
        {
            let now = std::time::Instant::now();
            let double = self.last_inv_click.is_some_and(|(at, last, was_right)| {
                last == slot && was_right == right && now.duration_since(at).as_millis() < 250
            });
            self.last_inv_click = Some((now, slot, right));
            // mouseClicked's QUICK_MOVE (and Horizon's at-press QUICK_MOVE
            // of a carried stack, mouseReleased's in vanilla) remember the
            // stack it moves.
            if input == Some(Input::QuickMove) || (self.carried.is_some() && !double) {
                self.last_quick_moved = self.slot_view(slot);
            }
            if double && !right {
                // mouseReleased: every stack of lastQuickMoved's item in the
                // clicked slot's container is quick-moved.
                if let Some(input) = input
                    && !self.click_open_inventory(Some(slot), 0, input)
                {
                    // A screen the menu engine doesn't model: its own click.
                    self.click_inventory(right);
                    return;
                }
                if let Some(moved) = self.last_quick_moved.clone() {
                    // The creative screen clicks its own menu, on a server too.
                    if self.remote.is_some() && self.screen != Screen::Creative {
                        self.remote_quick_move_all(slot, &moved);
                    } else {
                        self.menu_session(|menu, env, to_menu| {
                            Some(menu.quick_move_all(env, to_menu(slot)?, &moved).is_ok())
                        });
                        if self.remote.is_some() {
                            self.send_creative_slots();
                        }
                    }
                }
                self.last_inv_click = None;
                self.inv_clicks.skip_release();
                return;
            }
        }
        if let Some(slot) = slot
            && slot != 200
        {
            // An item tab's display slots take no drags or PICKUP_ALL.
            let display = slot >= Creative::DISPLAY;
            let now = std::time::Instant::now();
            let double = !shift
                && !right
                && !display
                && self.carried.is_some()
                && self.last_inv_click.is_some_and(|(at, last, was_right)| {
                    last == slot && !was_right && now.duration_since(at).as_millis() < 250
                });
            self.last_inv_click = Some((now, slot, right));
            if double && self.click_open_inventory(Some(slot), 0, Input::PickupAll) {
                self.last_inv_click = None;
                // The double click's own release (`mouseReleased`'s PICKUP_ALL).
                self.inv_clicks.skip_release();
                return;
            }
            if self.carried.is_some() {
                let ty = i32::from(right);
                // `mouseClicked` starts a quick craft drag whenever the hand holds a stack,
                // Shift or not: `mouseDragged` fills the slots the cursor crosses, so a
                // Shift held over the drag spreads the same way (#4166). The drag starts
                // empty on a Shift press: a release with nothing spread is `mouseReleased`'s
                // click on the hovered slot (Pickup, or QuickMove under Shift).
                if !shift && !display && self.quick_craft_accepts(slot, ty, 0) {
                    self.inv_drag = Some((ty, vec![slot]));
                } else {
                    self.inv_drag = Some((ty, Vec::new()));
                }
                return;
            }
        }
        if self.creative_tab_open() {
            // CreativeModeInventoryScreen goes through the same mouseClicked:
            // a click now, or a drag that its release ends.
            match input {
                Some(input) => {
                    self.click_open_inventory(slot, i32::from(right), input);
                }
                None if self.inv_clicks.quick_crafting().is_some() => {
                    self.inv_drag = Some((i32::from(right), Vec::new()));
                }
                None => {}
            }
            return;
        }
        // mouseClicked: an empty hand's PICKUP is the menu's click at once
        // (a right click on a bundle takes its first stack out).
        if let (Some(input), Some(_)) = (input, slot)
            && self.remote.is_none()
            && self.click_open_inventory(slot, i32::from(right), input)
        {
            if let Some(key) = self.container {
                self.container_changed(key);
            }
            return;
        }
        self.click_inventory(right);
    }
    /// Mouse movement while a quick craft drag is running.
    fn inventory_drag_move(&mut self) {
        if self.tweaks_on() && self.tweak_drag_move(self.inventory_target().0) {
            return;
        }
        let Some((ty, slots)) = self.inv_drag.clone() else {
            return;
        };
        let Some(r) = self.renderer.as_ref() else {
            return;
        };
        let batch = HudBatch::new(
            [r.config.width, r.config.height],
            self.settings.gui_scale_inputs(),
        );
        if let Some(slot) = self.slot_at(&batch, self.cursor.0, self.cursor.1)
            && slot != 200
            && slot < Creative::DISPLAY
            && !slots.contains(&slot)
            && self.quick_craft_accepts(slot, ty, slots.len())
            && let Some((_, slots)) = self.inv_drag.as_mut()
        {
            slots.push(slot);
        }
    }
    /// The hovered slot and whether the cursor is outside the window.
    fn inventory_target(&self) -> (Option<usize>, bool) {
        let Some(r) = self.renderer.as_ref() else {
            return (None, false);
        };
        let batch = HudBatch::new(
            [r.config.width, r.config.height],
            self.settings.gui_scale_inputs(),
        );
        (
            self.slot_at(&batch, self.cursor.0, self.cursor.1),
            self.clicked_outside(&batch),
        )
    }
    fn screen_button(right: bool) -> horizon_client::container_screen::Button {
        use horizon_client::container_screen::Button;
        if right { Button::Right } else { Button::Left }
    }
    /// `Screen.hasShiftDown` / `InputWithModifiers.hasShiftDown`: 26.3 keeps
    /// no key cache — every event answers with its own modifiers bit
    /// (`InputWithModifiers.java:54-56`, `keys = 3`), and the pointer and
    /// keyboard events carry the OS's live word, refreshed at the focus. The
    /// window's `ModifiersChanged` is that word here; key releases can die
    /// on the way from the OS (a stuck Shift then kept every screen's Shift
    /// and a sneak the key no longer held: #4166's in-game report), so a
    /// cached key set is not vanilla's answer.
    pub(super) fn has_shift_down(&self) -> bool {
        self.modifiers.shift_key()
    }
    /// The same live word for Control (`InputWithModifiers.hasControlDown`).
    pub(super) fn has_control_down(&self) -> bool {
        self.modifiers.control_key()
    }
    /// Real events word the modifiers by their own `ModifiersChanged`; a
    /// control socket's `key` is synthesized, so it words the bit directly
    /// and every reader can ask the one live state.
    fn press_modifier_key(&mut self, key: KeyCode, down: bool) {
        horizon_client_input::modifiers::set_key(
            &mut self.modifiers,
            &mut self.physical_keys,
            key,
            down,
        );
    }
    /// `hasClickedOutside` of the open screen at the cursor.
    fn clicked_outside(&self, batch: &HudBatch) -> bool {
        use horizon_client::container_screen::{Book, clicked_outside};
        let image = self.container_image(batch);
        let book = self
            .recipe_book_area(batch)
            .map(|(visible, tab_hovered)| Book {
                visible,
                selected_tab_hovered: tab_hovered,
            });
        clicked_outside(
            f64::from(self.cursor.0),
            f64::from(self.cursor.1),
            image,
            book,
        )
    }
    /// The open container screen's window: its place and size.
    fn container_image(&self, batch: &HudBatch) -> horizon_client::container_screen::Image {
        let (ox, oy) = self.inventory_origin(batch);
        let (shift, width, height) = if self.screen == Screen::Creative {
            (0.0, Creative::WIDTH as i32, Creative::HEIGHT as i32)
        } else {
            let (shift, width) = match self.station.as_ref().map(|s| s.kind) {
                Some(stations::Kind::Merchant) => (-50.0, 276),
                Some(stations::Kind::Beacon) => (beacon::SHIFT, 230),
                _ => (0.0, self.native_screen_width().map_or(176, |w| w as i32)),
            };
            (shift, width, self.screen_height() as i32)
        };
        horizon_client::container_screen::Image {
            left: (ox + shift) as i32,
            top: oy as i32,
            width,
            height,
        }
    }
    /// A click outside the window (slot id -999): `THROW` does nothing; a
    /// `PICKUP` drops the carried stack (left) or one of it (right), thrown
    /// from the hand (`AbstractContainerMenu.doClick`, `LivingEntity.drop`).
    fn throw_click(&mut self, right: bool, input: horizon_client::menu::Input) {
        use horizon_client::menu::Input;
        let button = i32::from(right);
        if self.remote.is_some() {
            if self.screen == Screen::Creative {
                self.remote_creative_drop(right);
            } else {
                self.remote_throw(right, input);
            }
            return;
        }
        let before = self.carried.clone();
        if !self.click_open_inventory(None, button, input) && input == Input::Pickup {
            // A screen the menu engine doesn't model: the same doClick.
            if let Some(c) = self.carried.as_mut() {
                let thrown = if right {
                    let one = ItemStack {
                        count: 1,
                        ..c.clone()
                    };
                    c.count -= 1;
                    if c.count == 0 {
                        self.carried = None;
                    }
                    one
                } else {
                    self.carried.take().expect("some")
                };
                self.player_drop(thrown);
            }
        }
        if self.carried != before {
            self.swing_arm();
        }
        if let Some(key) = self.container {
            self.container_changed(key);
        }
    }
    /// Mouse release over an inventory screen: `mouseReleased` ends or
    /// cancels the drag, drops a stack released outside, or clicks.
    fn inventory_mouse_release(&mut self, right: bool) {
        use horizon_client::container_screen::{Release, Target};
        self.tweak_release();
        let (slot, outside) = self.inventory_target();
        let dragged = self.inv_drag.as_ref().is_some_and(|(_, s)| !s.is_empty());
        let release = self.inv_clicks.release(
            &Target::of(slot, outside),
            Self::screen_button(right),
            self.carried.is_none(),
            dragged,
            self.has_shift_down(),
            false,
        );
        match (release, Target::of(slot, outside)) {
            (Release::QuickCraft, _) => self.inventory_release(),
            (Release::Click(input), Target::Outside) => {
                self.inv_drag = None;
                self.throw_click(right, input);
            }
            (Release::Click(input), Target::Slot(slot)) if self.inv_drag.is_some() => {
                // The drag never reached a slot: a plain click where it ends.
                self.inv_drag = None;
                if !self.click_open_inventory(Some(slot), i32::from(right), input) {
                    self.click_inventory(right);
                }
                if let Some(key) = self.container {
                    self.container_changed(key);
                }
            }
            _ => self.inv_drag = None,
        }
    }
    /// A press of the pick block button (`keyPickItem`, the middle button by
    /// default) over an inventory screen: `mouseClicked` with `cloning`
    /// (infinite materials). An empty hand clones the hovered stack at its
    /// full size (`CLONE`), a carried stack starts a quick craft drag of type
    /// 2 that fills every slot it crosses; without infinite materials the
    /// press does nothing.
    fn inventory_pick_press(&mut self) {
        use horizon_client::container_screen::{Button, Target};
        let (slot, outside) = self.inventory_target();
        let was_crafting = self.inv_clicks.quick_crafting().is_some();
        let input = self.inv_clicks.press(
            &Target::of(slot, outside),
            Button::Middle,
            self.carried.is_none(),
            self.has_shift_down(),
            self.creative,
        );
        match (input, slot) {
            // Slot -999 (outside the window) clones nothing.
            (Some(input), Some(slot)) if !outside => {
                self.click_open_inventory(Some(slot), 2, input);
            }
            (None, _) if !was_crafting && self.inv_clicks.quick_crafting().is_some() => {
                // Item tab display slots take no drags.
                let first = slot.filter(|&s| {
                    s != 200 && s < Creative::DISPLAY && self.quick_craft_accepts(s, 2, 0)
                });
                self.inv_drag = Some((2, first.into_iter().collect()));
            }
            _ => {}
        }
    }
    /// The pick block button's release (`mouseReleased`): ends its quick
    /// craft drag, cancels another button's, or clones onto the slot under a
    /// carried stack (`CLONE`, which only an item tab's display slot takes).
    fn inventory_pick_release(&mut self) {
        use horizon_client::container_screen::{Button, Release, Target};
        let (slot, outside) = self.inventory_target();
        let dragged = self.inv_drag.as_ref().is_some_and(|(_, s)| !s.is_empty());
        let release = self.inv_clicks.release(
            &Target::of(slot, outside),
            Button::Middle,
            self.carried.is_none(),
            dragged,
            self.has_shift_down(),
            true,
        );
        match (release, slot) {
            (Release::QuickCraft, _) => self.inventory_release(),
            (Release::Click(input), Some(slot)) if !outside => {
                self.inv_drag = None;
                self.click_open_inventory(Some(slot), 2, input);
            }
            _ => self.inv_drag = None,
        }
    }
    /// Mouse release: finishes a quick craft drag (one slot is a plain click).
    fn inventory_release(&mut self) {
        use horizon_client::menu::Input;
        let Some((ty, slots)) = self.inv_drag.take() else {
            return;
        };
        if slots.len() == 1 {
            if !self.click_open_inventory(Some(slots[0]), ty, Input::Pickup) {
                self.click_inventory(ty == 1);
            }
        } else {
            let mask = |header: i32| header | (ty << 2);
            let mut ops = vec![(None, mask(0), Input::QuickCraft)];
            ops.extend(
                slots
                    .into_iter()
                    .map(|slot| (Some(slot), mask(1), Input::QuickCraft)),
            );
            ops.push((None, mask(2), Input::QuickCraft));
            if !self.menu_ops(&ops) {
                // Not a menu the engine models: the drag is a plain click.
                self.click_inventory(ty == 1);
            }
        }
        if let Some(key) = self.container {
            self.container_changed(key);
        }
    }
    fn click_inventory(&mut self, right: bool) {
        if self.mod_browser_press(right) || self.mount_click(right) {
            return;
        }
        if self.recipe_book_click(right) {
            return;
        }
        if self.station_click(right) {
            return;
        }
        if self.remote.is_some() {
            return self.remote_click(right);
        }
        let Some(size) = self
            .renderer
            .as_ref()
            .map(|r| [r.config.width, r.config.height])
        else {
            return;
        };
        let batch = HudBatch::new(size, self.settings.gui_scale_inputs());
        let Some(slot) = self.slot_at(&batch, self.cursor.0, self.cursor.1) else {
            return;
        };
        let held_shift = self.has_shift_down();
        if self.crafter_click(slot, held_shift, right) {
            return;
        }
        if held_shift && self.container.is_some() && self.container_quick_move(slot) {
            if let Some(key) = self.container {
                self.world.modified.insert((key.0 >> 4, key.2 >> 4));
            }
            return;
        }
        if let Some(c) = &self.carried
            && self.refuses(slot, c)
        {
            return;
        }
        let max = |items: &Items, s: &ItemStack| items.get(&s.item).map_or(64, |i| i.max_stack);
        let shift = held_shift;
        if slot >= 300
            && let Some(key) = self.container
        {
            self.world.modified.insert((key.0 >> 4, key.2 >> 4));
        }
        let is_furnace = matches!(self.container_kind(), Some(BlockEntity::Furnace(_)));
        if is_furnace && slot == 302 {
            // FurnaceResultSlot: take only.
            let Some(result) = self.slot_mut(302).take() else {
                return;
            };
            match &mut self.carried {
                None => self.carried = Some(result),
                Some(c)
                    if c.item == result.item
                        && c.count + result.count <= max(world_items!(self), c) =>
                {
                    c.count += result.count
                }
                _ => *self.slot_mut(302) = Some(result),
            }
            return;
        }
        // Changing the input restarts cooking (AbstractFurnaceBlockEntity.setItem).
        let furnace_input = (is_furnace && slot == 300).then(|| self.slot_mut(300).clone());
        if slot == 200 {
            // ResultSlot: take the crafted stack and consume one of each ingredient.
            let mut crafted = 0;
            while let Some((recipe, result)) = self.craft_recipe() {
                let made = (result.item.clone(), result.count);
                let taken = result.clone();
                let cells = self.craft_size * self.craft_size;
                let ingredients = self.craft_grid[..cells].to_vec();
                let remainders = world_recipes!(self).remainders(
                    &recipe,
                    &self.craft_grid[..cells],
                    self.craft_size,
                );
                if shift {
                    if self.add_stack_to_inventory(result).is_some() {
                        break;
                    }
                } else {
                    match &mut self.carried {
                        None => self.carried = Some(result),
                        Some(c)
                            if c.item == result.item
                                && c.count + result.count <= max(world_items!(self), c) =>
                        {
                            c.count += result.count
                        }
                        _ => return,
                    }
                }
                for cell in self.craft_grid.iter_mut() {
                    if let Some(s) = cell {
                        s.count -= 1;
                        if s.count == 0 {
                            *cell = None;
                        }
                    }
                }
                for (i, rest) in remainders.into_iter().enumerate() {
                    let Some(rest) = rest else { continue };
                    match &mut self.craft_grid[i] {
                        cell @ None => *cell = Some(rest),
                        Some(c) if c.same_item_same_components(&rest) => c.count += rest.count,
                        _ => self.give_or_drop(rest),
                    }
                }
                crafted += 1;
                // `ResultSlot.onTake`: the recipe and the stack taken
                // (native_item_crafted).
                let player = self.native_acting_id();
                self.native_item_crafted(player, &format!("minecraft:{recipe}"), &taken);
                self.tutorial_get_item(Some(&taken));
                self.award_stat("crafted", &made.0, made.1 as i32);
                self.award_crafted(&[(format!("minecraft:{recipe}"), ingredients)]);
                if !shift || crafted >= 64 {
                    break;
                }
            }
            return;
        }
        if shift
            && (slot < 36 || (500..504).contains(&slot) || (100..109).contains(&slot))
            && self.inventory_quick_move(slot)
        {
            return;
        }
        if (500..504).contains(&slot) {
            let wanted = ["head", "chest", "legs", "feet"][slot - 500];
            let fits = self.carried.as_ref().is_none_or(|c| {
                self.res
                    .items()
                    .get(&c.item)
                    .and_then(|i| i.equip_slot.as_deref())
                    == Some(wanted)
            });
            if !fits {
                return;
            }
            std::mem::swap(&mut self.carried, &mut self.armor[slot - 500]);
            self.apply_equipment();
            return;
        }
        let carried = self.carried.take();
        let current = self.slot_mut(slot).take();
        let items = self.items();
        let (new_slot, new_carried) = match (carried, current) {
            (None, Some(mut s)) if right => {
                let half = s.count.div_ceil(2);
                s.count -= half;
                let taken = ItemStack {
                    count: half,
                    ..s.clone()
                };
                ((s.count > 0).then_some(s), Some(taken))
            }
            (None, s) => (None, s),
            (Some(mut c), None) if right => {
                let one = ItemStack {
                    count: 1,
                    ..c.clone()
                };
                c.count -= 1;
                (Some(one), (c.count > 0).then_some(c))
            }
            (Some(c), None) => (Some(c), None),
            (Some(mut c), Some(mut s)) if c.same_item_same_components(&s) => {
                let room = max(items, &s).saturating_sub(s.count);
                let moved = if right {
                    1.min(room).min(c.count)
                } else {
                    room.min(c.count)
                };
                s.count += moved;
                c.count -= moved;
                (Some(s), (c.count > 0).then_some(c))
            }
            (Some(c), Some(s)) => (Some(c), Some(s)),
        };
        self.carried = new_carried;
        *self.slot_mut(slot) = new_slot;
        if let Some(before) = furnace_input
            && let Some(BlockEntity::Furnace(f)) = self
                .container
                .and_then(|k| self.world.block_entities.get_mut(&k))
        {
            f.input_clicked(&self.loop_context, world_recipes!(self), before);
        }
    }
    /// `Camera.setRotation`: the camera's orientation, turned round in
    /// third-person front view.
    pub(super) fn camera_orientation(&self) -> glam::Quat {
        match self.posed_rotation {
            // The frame's camera, already turned round in front view.
            Some((yaw, pitch)) => {
                camera_rotation(horizon_client::camera::CameraType::FirstPerson, yaw, pitch)
            }
            None => camera_rotation(self.camera_type, self.yaw, self.pitch),
        }
    }
    /// The entity whose geometry starts at vertex `start` (0: none), for frame
    /// generation to follow its own motion. Each run lasts until the next mark.
    pub(super) fn mark_motion(&self, start: usize, id: u64) {
        self.entity_motion.borrow_mut().push((start as u32, id));
        // Where its GPU-posed vertices start (`entity_skin`): a shader pack's `at_velocity`
        // follows those too.
        let posed = self.skin_frame.borrow().vertices;
        self.entity_motion_posed.borrow_mut().push(posed);
    }
    /// ItemEntityRenderer: items bob and spin; blocks render as small cubes.
    fn entity_geometry(
        &mut self,
        camera: glam::DVec3,
        alpha: f64,
        mut out: Vec<Vertex>,
    ) -> (
        Vec<Vertex>,
        Arc<[StaticSectionSnapshot]>,
        Vec<[i32; 3]>,
        StaticCacheFrameStats,
    ) {
        // Its steps as spans (HORIZON_SPIKES): each lap names the step that just ended.
        let mut laps = horizon_client::spans::laps();
        let mut static_block_entities = std::sync::Arc::from([]);
        let mut visible_static_block_entity_sections = Vec::new();
        let mut static_block_entity_stats = StaticCacheFrameStats::default();
        self.entity_spans.borrow_mut().clear();
        self.entity_motion.borrow_mut().clear();
        self.entity_motion_posed.borrow_mut().clear();
        // LevelExtractor.extract: the transient blocks queued since the last frame.
        self.drain_transient_block_queue();
        self.prepare_entity_culling();
        self.prepare_occlusion_culling(camera);
        self.prepare_trims();
        self.begin_skinning();
        self.mark_entity(out.len(), "minecraft:entity_flame");
        // One scan for both passes that draw them (block entities below and the
        // overlay near the end of this function).
        let cracks = self.cracks(camera);
        let fire_yaw = if self.camera_type == horizon_client::camera::CameraType::ThirdPersonFront {
            self.yaw + 180.0
        } else {
            self.yaw
        };
        // Packs may move vertices or draw from another camera; TAA jitters the
        // projection beyond this unjittered frustum. Keep their original geometry path.
        let fire_view = self.view_frustum.as_ref().filter(|_| {
            self.pack_ids.is_none()
                && self.native_shaders.picked.is_empty()
                && !self.settings.horizon.taa
        });
        let fire_visible = |position, width, height| {
            fire_view.is_none_or(|view| {
                horizon_client::fire_render::in_view(view, camera, position, width, height)
            })
        };
        laps.lap("eg_prep");
        for m in &self.mobs {
            // Blaze.isOnFire: a charged blaze burns.
            let blazing = m.species() == horizon_client::mobs::Species::Blaze && m.extra.flag;
            if (m.is_on_fire() || blazing) && !m.removed {
                let at = m.previous.lerp(m.position, alpha);
                if !fire_visible([at.x, at.y, at.z], m.width(), m.height()) {
                    continue;
                }
                horizon_client::fire_render::flames(
                    [at.x, at.y, at.z],
                    m.width(),
                    m.height(),
                    camera,
                    fire_yaw,
                    &self.gui.atlas.sprites,
                    self.world.light(
                        at.x.floor() as i32,
                        at.y.floor() as i32,
                        at.z.floor() as i32,
                    ),
                    &mut out,
                );
            }
        }
        if self.own_body_in_view()
            && let Some(p) = &self.player
            && p.fire_ticks > 0
            && !p.spectator
        {
            let feet = self
                .previous_position
                .lerp(p.position, self.clock.camera_fraction());
            if fire_visible(
                [feet.x, feet.y, feet.z],
                p.dimensions.width,
                p.dimensions.height,
            ) {
                horizon_client::fire_render::flames(
                    [feet.x, feet.y, feet.z],
                    p.dimensions.width,
                    p.dimensions.height,
                    camera,
                    fire_yaw,
                    &self.gui.atlas.sprites,
                    self.world.light(
                        feet.x.floor() as i32,
                        feet.y.floor() as i32,
                        feet.z.floor() as i32,
                    ),
                    &mut out,
                );
            }
        }
        laps.lap("eg_fire_mobs");
        for item in &self.dropped {
            if item.fire_ticks > 0
                && !self
                    .res
                    .items()
                    .resists_fire(&item.stack, "minecraft:in_fire")
            {
                let at = item.previous.lerp(item.position, alpha);
                if !fire_visible([at.x, at.y, at.z], 0.25, 0.25) {
                    continue;
                }
                horizon_client::fire_render::flames(
                    [at.x, at.y, at.z],
                    0.25,
                    0.25,
                    camera,
                    fire_yaw,
                    &self.gui.atlas.sprites,
                    self.world.light(
                        at.x.floor() as i32,
                        at.y.floor() as i32,
                        at.z.floor() as i32,
                    ),
                    &mut out,
                );
            }
        }
        laps.lap("eg_fire_items");
        for stand in &self.armor_stands {
            if stand.fire_ticks > 0 && !stand.marker {
                let scale = if stand.small { 0.5 } else { 1.0 };
                let at = stand.render_position(alpha);
                if !fire_visible([at.x, at.y, at.z], 0.5 * scale, 1.975 * scale) {
                    continue;
                }
                horizon_client::fire_render::flames(
                    [at.x, at.y, at.z],
                    0.5 * scale,
                    1.975 * scale,
                    camera,
                    fire_yaw,
                    &self.gui.atlas.sprites,
                    self.world.light(
                        at.x.floor() as i32,
                        at.y.floor() as i32,
                        at.z.floor() as i32,
                    ),
                    &mut out,
                );
            }
        }
        laps.lap("eg_fire_stands");
        // `ItemEntityRenderer`.
        self.mark_entity(out.len(), "minecraft:item");
        self.dropped_item_geometry(camera, alpha, &mut out);
        self.mark_motion(out.len(), 0);
        // `VaultRenderer`.
        self.mark_entity(out.len(), "");
        self.vault_geometry(camera, alpha, &mut out);
        // `CampfireRenderer`.
        self.mark_entity(out.len(), "");
        self.campfire_geometry(camera, &mut out);
        self.brushable_geometry(camera, &mut out);
        self.shelf_geometry(camera, &mut out);
        self.mark_entity(out.len(), "minecraft:ominous_item_spawner");
        self.ominous_item_spawner_geometry(camera, alpha, &mut out);
        laps.lap("eg_items_misc");
        // Mobs and their projectiles (a span each).
        self.mob_geometry(camera, alpha, &mut out);
        self.mark_motion(out.len(), 0);
        laps.lap("eg_mobs");
        // In first person only a shader pack's shadow pass draws the player.
        self.mark_player(out.len(), !self.own_body_in_view());
        let player_start = out.len();
        self.player_model_geometry(camera, self.clock.camera_fraction(), &mut out);
        // A native pack has no per-entity draws to hide it with: its shadow pass alone gets
        // the first-person model.
        if self.pack_ids.is_none() && !self.own_body_in_view() {
            self.shadow_only_vertices
                .borrow_mut()
                .extend(out.drain(player_start..));
        }
        self.mark_entity(out.len(), "minecraft:evoker_fangs");
        self.fangs_geometry(camera, alpha, &mut out);
        self.block_item_display_geometry(camera, alpha, &mut out);
        laps.lap("eg_players");
        let mut see_through = std::mem::take(&mut self.see_through_vertices);
        see_through.clear();
        self.name_tag_geometry(camera, alpha, &mut out, &mut see_through);
        self.test_instance_marker_geometry(camera, &mut out, &mut see_through);
        self.debug_renderer_geometry(camera, &mut out);
        self.text_display_geometry(camera, alpha, &mut out, &mut see_through);
        self.see_through_vertices = see_through;
        laps.lap("eg_name_tags");
        // Entity shadows (EntityRenderer.extractShadow).
        self.mark_translucent_entity(out.len(), "minecraft:entity_shadow");
        // A native shader pack's sun shadows already fall under the entities in the Overworld:
        // the round blob would be a second one. The Nether and the End have no sun to cast them.
        let pack_shadows = self.renderer.as_ref().is_some_and(|r| {
            r.pack_entity_shadows() || (self.dim == Dim::Overworld && r.native_shadow())
        });
        let shadow_sprite = (self.settings.entity_shadows && !pack_shadows)
            .then(|| self.gui.atlas.sprites.get("misc/shadow").copied())
            .flatten();
        if let Some(sprite) = shadow_sprite {
            // DimensionType.ambientLight: the_nether 0.1, the_end 0.25.
            let ambient = match self.dim {
                Dim::Nether => 0.1,
                Dim::End => 0.25,
                Dim::Overworld => 0.0,
            };
            let darken = self.sky_darken();
            let a = alpha;
            let lerp = |p: Vec3, q: Vec3| {
                glam::DVec3::new(
                    p.x + (q.x - p.x) * a,
                    p.y + (q.y - p.y) * a,
                    p.z + (q.z - p.z) * a,
                )
            };
            let mut casters: Vec<(glam::DVec3, f32, f32)> =
                horizon_client::shadows::mob_casters(&self.mobs, a).collect();
            let item = horizon_client::shadows::shadow_params("minecraft:item");
            casters.extend(
                self.dropped
                    .iter()
                    .map(|i| (lerp(i.previous, i.position), item.0, item.1)),
            );
            let orb = horizon_client::shadows::shadow_params("minecraft:experience_orb");
            casters.extend(
                self.orbs
                    .iter()
                    .map(|o| (lerp(o.previous, o.position), orb.0, orb.1)),
            );
            // Text, block and item displays (DisplayRenderer's shadow radius).
            casters.extend(self.text_displays.values().filter_map(|d| d.shadow(a)));
            // Packs may move vertices or draw from another view.
            let shadow_view = self
                .view_frustum
                .as_ref()
                .filter(|_| self.pack_ids.is_none() && self.native_shaders.picked.is_empty());
            for (pos, radius, strength) in casters {
                if let Some(view) = shadow_view
                    && !horizon_client::shadows::in_view(view, camera, pos, radius, strength)
                {
                    continue;
                }
                horizon_client::shadows::shadow(
                    &self.world,
                    self.res.facts(),
                    &|state| {
                        self.res
                            .collision()
                            .cached_shapes()
                            .cached_shape(state)
                            .ok()
                            .flatten()
                            .and_then(|shape| shape.bounds())
                    },
                    sprite,
                    camera,
                    pos,
                    radius,
                    strength,
                    ambient,
                    darken,
                    &mut out,
                );
            }
        }
        laps.lap("eg_shadows");
        // Block entities (BlockEntityRenderers): chests, shulker boxes, banners, skulls...
        if let Some(r) = &self.renderer {
            // Block entities outside the view frustum are skipped (with a
            // block of margin for models larger than their block).
            let view = self.view_frustum;
            // Beams and structure outlines reach far outside their block
            // (BlockEntityRenderer.shouldRenderOffScreen): never culled.
            let off_screen = |state: u32| {
                let name = &*self.res.facts().get(BlockStateId(state)).name;
                matches!(
                    name,
                    "minecraft:beacon"
                        | "minecraft:end_gateway"
                        | "minecraft:structure_block"
                        | "minecraft:test_instance_block"
                )
            };
            let long_range = |state: u32| {
                &*self.res.facts().get(BlockStateId(state)).name == "minecraft:end_gateway"
            };
            let in_view = |p: &[i32; 3]| {
                view.is_none_or(|v| {
                    let o = (glam::DVec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]))
                        - camera)
                        .as_vec3();
                    v.intersects(o - glam::Vec3::ONE, o + glam::Vec3::splat(2.0))
                })
            };
            let camera_list: Vec<&([i32; 3], u32)> = r
                .visible_block_entities_in_frustum(
                    self.settings.chunk_fade,
                    camera,
                    view,
                    &off_screen,
                    long_range,
                )
                .filter(|(block, section_inside)| {
                    let (pos, state) = **block;
                    self.res.block_entity_render().renders(state)
                        && (off_screen(state) || *section_inside || in_view(&pos))
                })
                .map(|(block, _)| block)
                .collect();
            // A shader pack's shadow pass draws the block entities of every section it draws,
            // in the view or not (`Renderer::shadow_pass_block_entities`, #8464): those the
            // view leaves out go in hidden from the camera.
            let shadow_only: Vec<&([i32; 3], u32)> =
                if self.pack_ids.is_some() && self.pack_shadow_casters.0 {
                    let listed: HashSet<[i32; 3]> = camera_list.iter().map(|b| b.0).collect();
                    r.shadow_pass_block_entities(camera)
                        .filter(|(pos, state)| {
                            !listed.contains(pos) && self.res.block_entity_render().renders(*state)
                        })
                        .collect()
                } else {
                    Vec::new()
                };
            let shadow_only_at: HashSet<[i32; 3]> = shadow_only.iter().map(|b| b.0).collect();
            let dynamic_visible = |pos: [i32; 3], state: u32, lo: glam::DVec3, hi: glam::DVec3| {
                if off_screen(state) || shadow_only_at.contains(&pos) {
                    return true;
                }
                if let Some(view) = view
                    && !view.intersects(
                        (lo - camera - glam::DVec3::splat(0.5)).as_vec3(),
                        (hi - camera + glam::DVec3::splat(0.5)).as_vec3(),
                    )
                {
                    return false;
                }
                let name = &*self.res.facts().get(BlockStateId(state)).name;
                if self.settings.horizon.entity_culling == EntityCullingMode::Off
                    || self.pack_shadow_casters.0
                    || name == "minecraft:end_portal"
                    || name == "minecraft:end_gateway"
                    || hi.y - lo.y > 2.0
                {
                    return true;
                }
                // Culler::aabb_bounds adds 0.05 on each side. Keep its final
                // query just inside the conservative rendered-model envelope.
                let bounds = Aabb::new(
                    Vec3::new(lo.x + 0.06, lo.y + 0.06, lo.z + 0.06),
                    Vec3::new(hi.x - 0.06, hi.y - 0.06, hi.z - 0.06),
                );
                !self.block_entity_occluded(bounds)
            };
            let lid = |p: [i32; 3]| self.chest_lid(p, alpha as f32);
            let anim = |p: [i32; 3]| self.block_anim(p, alpha as f32);
            let scene = BlockEntityScene {
                camera,
                orientation: self.camera_orientation(),
                anim: &anim,
                texts: None,
                partial: alpha as f32,
                game_time: self.world.game_time,
                max_y: self.world.shape.max_y() - 1,
                world: &self.world,
                sprites: &self.gui.atlas.sprites,
                emissive: &self.gui.atlas.emissive,
                translucent_sprites: &self.gui.atlas.translucent,
                atlas_revision: self.gui.atlas.revision,
                nether: self.dim == Dim::Nether,
                lid: &lid,
                sign: &|p| match self.world.block_entities.get(&(p[0], p[1], p[2])) {
                    Some(BlockEntity::Sign(sign)) => Some(sign),
                    _ => None,
                },
                banner: &|p| self.banner_layers_at(p),
                skull: &|p| self.skull_skin_at(p),
                glyph: &|c| self.gui.atlas.glyph_cell(c),
                glyphs: self.gui.atlas.font.epoch(),
                dynamic_visible: Some(&dynamic_visible),
            };
            let visible = camera_list.into_iter().chain(shadow_only.iter().copied());
            let block_entity_cache = CacheOptions {
                shulker_boxes: self.settings.horizon.cache_shulker_boxes,
                signs: self.settings.horizon.cache_signs,
                decorated_pots: self.settings.horizon.cache_decorated_pots,
                bells: self.settings.horizon.cache_bells,
                chests: self.settings.horizon.cache_chests,
                banners: self.settings.horizon.cache_banners,
                copper_golem_statues: self.settings.horizon.cache_copper_golem_statues,
            };
            // Entity occlusion is for ordinary entity meshes. Block entities
            // already have section and per-block frustum culling; routing them
            // through the per-vertex occlusion callback both repeats that work
            // and disables their per-section GPU cache. Shader packs still need
            // per-entity span marks, so they keep the callback path.
            if self.pack_ids.is_some() {
                let section_revision = |pos| r.block_entity_section_revision(pos);
                static_block_entity_stats = self
                    .res
                    .block_entity_render()
                    .render_with_cache_options_and_callbacks(
                        &scene,
                        visible,
                        block_entity_cache,
                        r.block_entity_revision(),
                        Some(&section_revision),
                        &mut out,
                        |_, state, start| self.mark_block_entity(start, state),
                        |pos, _, start, out| {
                            if shadow_only_at.contains(&pos) {
                                self.hide_spans_from(start, out.len());
                            }
                        },
                    );
            } else {
                let cache_candidates = r
                    .visible_block_entities_in_frustum(
                        self.settings.chunk_fade,
                        camera,
                        None,
                        |_| false,
                        |_| false,
                    )
                    .filter(|(block, _)| self.res.block_entity_render().renders(block.1))
                    .map(|(block, _)| block);
                (
                    static_block_entities,
                    visible_static_block_entity_sections,
                    static_block_entity_stats,
                ) = self
                    .res
                    .block_entity_render()
                    .render_with_gpu_cache_options(
                        &scene,
                        visible,
                        cache_candidates,
                        block_entity_cache,
                        r.block_entity_revision(),
                        |pos| r.block_entity_section_revision(pos),
                        &mut out,
                    );
            }
            // Mod block entities: their mods draw them (`native_block_entity_submit`),
            // culled by the bounds they give. A world without any (`has_mods`) has none
            // to find among its other unrendered block entities (furnaces, hoppers...).
            let mods = self
                .world
                .block_entities
                .has_mods()
                .then(|| {
                    r.visible_block_entities(self.settings.chunk_fade)
                        .filter(|(_, state)| !self.res.block_entity_render().renders(*state))
                })
                .into_iter()
                .flatten();
            let untemplated = self.untemplated_mod_block_entities(camera);
            self.mod_block_entity_geometry(
                mods.chain(untemplated.iter()),
                camera,
                alpha as f32,
                &mut out,
            );
            self.mark_stage(out.len(), "DESTROY");
            for &(pos, stage) in &cracks {
                let state = self.world.block(pos[0], pos[1], pos[2]);
                self.res
                    .block_entity_render()
                    .render_breaking(&scene, pos, state.0, stage, &mut out);
            }
        }
        laps.lap("eg_block_entities");
        self.mark_entity(out.len(), "minecraft:experience_orb");
        let live = self.orbs.iter().map(|o| {
            let at = [
                o.previous.x + (o.position.x - o.previous.x) * alpha,
                o.previous.y + (o.position.y - o.previous.y) * alpha,
                o.previous.z + (o.position.z - o.previous.z) * alpha,
            ];
            (o, at, alpha as f32)
        });
        self.orb_geometry(camera, live, &mut out);
        laps.lap("eg_orbs");
        // Falling blocks use their baked model, including non-cubic shapes.
        self.mark_entity(out.len(), "minecraft:falling_block");
        // Packs can move vertices or draw another view; otherwise only depth
        // planes are used, which TAA's X/Y projection jitter cannot change.
        let falling_view = self
            .view_frustum
            .as_ref()
            .filter(|_| self.pack_ids.is_none() && self.native_shaders.picked.is_empty());
        let tables = &self.pack.context.tables;
        let falling_forward = self.camera_orientation() * -glam::Vec3::Z;
        for f in &self.falling {
            let p = f.model_origin(alpha, |state, x, z| tables.offset(state, x, z)) - camera;
            let o = p.as_vec3();
            // Only rear-origin models pay for the envelope/depth check. The
            // full posed-camera test still decides rejection, including models
            // whose origin is behind but whose geometry reaches into view.
            if falling_forward.dot(o) < 0.0
                && falling_view.is_some_and(|view| {
                    self.pack
                        .render
                        .model_catalog
                        .render_bounds(f.state)
                        .is_some_and(|[lo, hi]| {
                            let lo = glam::Vec3::from_array(lo) + o;
                            let hi = glam::Vec3::from_array(hi) + o;
                            let magnitude = o
                                .abs()
                                .max_element()
                                .max(lo.abs().max_element())
                                .max(hi.abs().max_element());
                            let padding =
                                glam::Vec3::splat(1.0e-4 + magnitude * f32::EPSILON * 4.0);
                            !view.intersects_depth(lo - padding, hi + padding)
                        })
                })
            {
                continue;
            }
            // `FallingBlockEntity.blockPosition()`: the cell its feet are in.
            let (x, y, z) = (
                f.position.x.floor() as i32,
                f.position.y.floor() as i32,
                f.position.z.floor() as i32,
            );
            if !falling_block_visible(f.state, self.world.block(x, y, z)) {
                continue;
            }
            let Some((_, tint)) = self.falling_block_face_sprite(f.state) else {
                continue;
            };
            let light = self.world.light(x, y + 1, z);
            let packed = (u32::from(light >> 4) * 16) | ((u32::from(light & 15) * 16) << 8);
            let start = out.len();
            self.block_model_vertices(
                f.state,
                horizon_core::random::position_seed(f.start_pos),
                glam::Mat4::from_translation(o),
                packed,
                false,
                &mut out,
            );
            let tint = horizon_client::mobs::render::abgr(tint);
            for vertex in &mut out[start..] {
                vertex.color = horizon_client::hud_layout::multiply(vertex.color, tint);
            }
        }
        self.mark_entity(out.len(), "");
        self.moving_piston_geometry(camera, alpha, &mut out);
        // LevelRenderer.submitTransientBlocks.
        self.transient_block_geometry(camera, &mut out);
        self.mark_entity(out.len(), "minecraft:tnt");
        self.tnt_geometry(camera, alpha, &mut out);
        self.mark_entity(out.len(), "minecraft:painting");
        self.painting_geometry(camera, &mut out);
        self.mark_entity(out.len(), "minecraft:item_frame");
        self.frame_geometry(camera, &mut out);
        self.mark_entity(out.len(), "minecraft:armor_stand");
        self.armor_stand_geometry(camera, alpha, &mut out);
        self.cushion_geometry(camera, &mut out);
        self.mark_entity(out.len(), "minecraft:oak_boat");
        self.boat_geometry(camera, alpha, &mut out);
        self.mark_entity(out.len(), "minecraft:minecart");
        self.cart_geometry(camera, alpha, &mut out);
        // Beams: `gbuffers_beaconbeam`, not a block entity's program (5.4 has no stage for them).
        self.mark_stage(out.len(), "NONE");
        self.beacon_geometry(camera, alpha, &mut out);
        self.mark_entity(out.len(), "minecraft:fishing_bobber");
        self.fishing_geometry(camera, alpha, &mut out);
        self.mark_entity(out.len(), "minecraft:leash_knot");
        self.leash_geometry(camera, alpha, &mut out);
        self.mark_entity(out.len(), "minecraft:trident");
        self.trident_geometry(camera, alpha, &mut out);
        self.mark_entity(out.len(), "minecraft:firework_rocket");
        self.rocket_geometry(camera, alpha, &mut out);
        self.mark_entity(out.len(), "minecraft:splash_potion");
        self.thrown_potion_geometry(camera, alpha, &mut out);
        laps.lap("eg_objects");
        // Breaking progress overlays (destroy_stage_0..9): ours and the other players'.
        self.mark_stage(out.len(), "DESTROY");
        for &(pos, stage) in &cracks {
            let o = (glam::DVec3::new(f64::from(pos[0]), f64::from(pos[1]), f64::from(pos[2]))
                - camera)
                .as_vec3();
            let state = self.world.block(pos[0], pos[1], pos[2]);
            if self.res.facts().get(state).render_shape
                == horizon_client::blocks::RenderShape::Model
                && let Ok(models) = self
                    .pack
                    .render
                    .model_catalog
                    .collect(state, BlockPos::new(pos[0], pos[1], pos[2]))
            {
                let offset = self
                    .pack
                    .context
                    .tables
                    .offset(state, pos[0], pos[2])
                    .unwrap_or([0.0; 3]);
                let origin = o + glam::DVec3::from(offset).as_vec3();
                let translucent = models
                    .iter()
                    .any(|(_, model)| model.faces.iter().any(|f| f.translucent));
                for (_, model) in models {
                    horizon_client::breaking::geometry(model, origin, translucent, stage, &mut out);
                }
            }
        }
        laps.lap("eg_cracks");
        self.finish_async_occlusion_culling();
        laps.lap("eg_culling_finish");
        (
            out,
            static_block_entities,
            visible_static_block_entity_sections,
            static_block_entity_stats,
        )
    }

    fn orb_geometry<'a>(
        &self,
        camera: glam::DVec3,
        orbs: impl Iterator<Item = (&'a horizon_client::entities::ExperienceOrb, [f64; 3], f32)>,
        out: &mut Vec<Vertex>,
    ) {
        if let Some(r) = self
            .gui
            .atlas
            .sprites
            .get("entity/experience/experience_orb")
            .copied()
        {
            let (yaw, pitch) = (self.yaw.to_radians(), self.pitch.to_radians());
            let forward = glam::Vec3::new(
                -yaw.sin() * pitch.cos(),
                -pitch.sin(),
                yaw.cos() * pitch.cos(),
            );
            let right = forward.cross(glam::Vec3::Y).normalize_or_zero();
            let up = right.cross(forward).normalize_or_zero();
            for (o, at, partial) in orbs {
                let p = glam::DVec3::new(at[0], at[1] + 0.1, at[2]) - camera;
                let icon = o.icon();
                let (du, dv) = ((r.u1 - r.u0) / 4.0, (r.v1 - r.v0) / 4.0);
                let (u0, v0) = (r.u0 + (icon % 4) as f32 * du, r.v0 + (icon / 4) as f32 * dv);
                let t = (o.age as f32 + partial) / 2.0;
                let red = ((t.sin() + 1.0) * 0.5 * 255.0) as u32;
                let blue = (((t + 4.188_79).sin() + 1.0) * 0.1 * 255.0) as u32;
                let color = 0x8000_0000 | blue << 16 | 255 << 8 | red;
                // `ExperienceOrbRenderer.getBlockLightLevel` (the block light
                // plus 7, clamped) at the block the orb's eye is in.
                let light = self.world.light(
                    o.position.x.floor() as i32,
                    o.light_probe_y().floor() as i32,
                    o.position.z.floor() as i32,
                );
                let packed = o.render_light(i32::from(light));
                let c = p.as_vec3();
                // `ExperienceOrbRenderer.submit`: the quad runs from
                // (-0.5, -0.25) to (0.5, 0.75) and is scaled by 0.3, so the
                // sprite hangs a quarter of its height below the orb's centre.
                let s = horizon_client::entities::ORB_QUAD_SCALE;
                for (k, uv) in horizon_client::entities::ORB_QUAD.iter().zip([
                    [u0, v0 + dv],
                    [u0 + du, v0 + dv],
                    [u0 + du, v0],
                    [u0, v0],
                ]) {
                    let v = c + right * (k[0] * s) + up * (k[1] * s);
                    out.push(Vertex {
                        normal: [0; 4],
                        pos: v.to_array(),
                        uv,
                        // ExperienceOrbRenderer: `RenderTypes.entityTranslucent`; in HDR
                        // output the orbs shine (entity.wgsl fs_hdr_mark).
                        layer: horizon_client::gfx::OIT_ENTITY
                            | u32::from(horizon_client::mesher::VERY_BRIGHT) << 16,
                        color,
                        light: packed,

                        glint_uv: [0.0, 0.0],
                    });
                }
            }
        }
    }

    /// Extract every particle group after the frame's entity, screen and hand callbacks.
    /// Item model callbacks can clear during extraction too: roll back only this tail of
    /// geometry (including GPU jobs and outlines), never the frame's ordinary entities.
    fn client_particle_geometry(
        &mut self,
        camera: glam::DVec3,
        alpha: f64,
        fov_degrees: f32,
        out: &mut Vec<Vertex>,
    ) {
        self.apply_mod_particle_clear();
        if self.client_particle_count() == 0 {
            return;
        }
        let first = out.len();
        let skin = self.skin_frame.borrow().mark();
        let outline_jobs = self.skin_frame.borrow().outline_jobs.len();
        let outline_vertices = self.skin_frame.borrow().outline_vertices;
        let spans = self.entity_spans.borrow().len();
        let motion = self.entity_motion.borrow().len();
        let glowing = self.glowing_vertices.borrow().len();
        self.mark_motion(out.len(), 0);
        // Particles (ParticleEngine.extract): camera-facing quads.
        self.mark_stage(out.len(), "PARTICLES");
        let world = &self.world;
        let particle_mode = self.settings.horizon.particle_culling;
        if particle_mode == horizon_client::graphics::ParticleCullingMode::Off {
            self.particles.render(
                camera,
                self.yaw,
                self.pitch,
                alpha as f32,
                &|b| world.light(b[0], b[1], b[2]),
                out,
            );
        } else {
            let frustum = self.view_frustum;
            let pixel_focal_length = self.renderer.as_ref().map_or(0.0, |renderer| {
                let tangent = (f64::from(fov_degrees).to_radians() * 0.5).tan();
                if tangent.is_finite() && tangent > 0.0 {
                    (f64::from(renderer.config.height.max(1)) / (2.0 * tangent)) as f32
                } else {
                    0.0
                }
            });
            {
                let mut particle_culling = self.particle_occlusion_context(camera);
                self.particles.render_culled(
                    camera,
                    self.yaw,
                    self.pitch,
                    alpha as f32,
                    pixel_focal_length,
                    true,
                    frustum.as_ref(),
                    &mut |min, max| particle_culling.hidden(min, max),
                    &|b| world.light(b[0], b[1], b[2]),
                    out,
                );
            }
            self.finish_async_particle_culling();
        }
        self.mark_entity(out.len(), "minecraft:item");
        self.pickup_item_geometry(camera, alpha, out);
        self.mark_motion(out.len(), 0);
        self.mark_entity(out.len(), "minecraft:arrow");
        self.pickup_arrow_geometry(camera, alpha, out);
        self.mark_entity(out.len(), "minecraft:trident");
        self.pickup_trident_geometry(camera, alpha, out);
        self.mark_entity(out.len(), "minecraft:experience_orb");
        // ItemPickupParticle: the render state extracted at partial tick 1.
        let taken = self.pickups.live.iter().filter_map(|p| match &p.taken {
            pickups::Taken::Orb(o) => Some((&**o, pickups::position(p, alpha), 1.0)),
            _ => None,
        });
        self.orb_geometry(camera, taken, out);
        self.mark_entity(out.len(), "minecraft:elder_guardian");
        self.elder_apparition_geometry(alpha, out);
        if self.mod_clear_particles.get() {
            self.apply_mod_particle_clear();
            out.truncate(first);
            {
                let mut frame = self.skin_frame.borrow_mut();
                frame.truncate(skin);
                frame.outline_jobs.truncate(outline_jobs);
                frame.outline_vertices = outline_vertices;
            }
            self.entity_spans.borrow_mut().truncate(spans);
            self.entity_motion.borrow_mut().truncate(motion);
            self.glowing_vertices.borrow_mut().truncate(glowing);
            return;
        }
        self.mark_motion(out.len(), 0);
        self.mark_stage(out.len(), "NONE");
    }

    /// `LevelRenderer.destroyBlockProgress`: a player's crack on a block (0..=9), or none
    /// (any other progress). Cracks not updated for 400 ticks are dropped.
    pub(crate) fn destroy_block_progress(&mut self, breaker: i32, pos: [i32; 3], progress: i32) {
        let now = self.world.game_time;
        self.destroying
            .retain(|_, c| now.saturating_sub(c.2) <= 400);
        if (0..10).contains(&progress) {
            self.destroying.insert(breaker, (pos, progress as u8, now));
        } else {
            self.destroying.remove(&breaker);
        }
    }

    /// `ServerLevel.destroyBlockProgress`: breaker `breaker`'s crack on `pos`
    /// (0..=9, anything else takes it away), for every player within 64 blocks.
    pub(super) fn server_destroy_block_progress(
        &mut self,
        breaker: i32,
        pos: [i32; 3],
        progress: i32,
    ) {
        self.destroy_block_progress(breaker, pos, progress);
        if let Some(host) = self.lan.as_ref() {
            host.hub.broadcast_near(
                [f64::from(pos[0]), f64::from(pos[1]), f64::from(pos[2])],
                64.0,
                &horizon_protocol::packets::play::Clientbound::BlockDestruction(
                    horizon_protocol::packets::play::BlockDestruction {
                        id: breaker,
                        pos,
                        progress: if (0..10).contains(&progress) {
                            progress as u8
                        } else {
                            255
                        },
                    },
                ),
                None,
            );
        }
    }

    /// The cracks to draw and their stage sprite's atlas origin: our own mining and the
    /// other players' (`renderBlockDestroyAnimation`: within 32 blocks of the camera, the
    /// furthest progress on each block).
    fn cracks(&self, camera: glam::DVec3) -> Vec<([i32; 3], [f32; 2])> {
        // `destroy_stage_0` .. `destroy_stage_9`: the sprite name by stage, so the
        // lookup does not format a string per crack.
        const DESTROY_STAGES: [&str; 10] = [
            "block/destroy_stage_0",
            "block/destroy_stage_1",
            "block/destroy_stage_2",
            "block/destroy_stage_3",
            "block/destroy_stage_4",
            "block/destroy_stage_5",
            "block/destroy_stage_6",
            "block/destroy_stage_7",
            "block/destroy_stage_8",
            "block/destroy_stage_9",
        ];
        // `ModelBakery.DESTROY_STAGES_OIT`: with improved transparency the cracks are
        // ordinary translucent quads (`OIT_CRUMBLING`) and use their own sprites.
        const DESTROY_STAGES_OIT: [&str; 10] = [
            "block/destroy_oit_stage_0",
            "block/destroy_oit_stage_1",
            "block/destroy_oit_stage_2",
            "block/destroy_oit_stage_3",
            "block/destroy_oit_stage_4",
            "block/destroy_oit_stage_5",
            "block/destroy_oit_stage_6",
            "block/destroy_oit_stage_7",
            "block/destroy_oit_stage_8",
            "block/destroy_oit_stage_9",
        ];
        let names = if self.settings.improved_transparency {
            &DESTROY_STAGES_OIT
        } else {
            &DESTROY_STAGES
        };
        let mut stages: Vec<([i32; 3], u32)> = Vec::new();
        let mut add = |pos: [i32; 3], stage: u32| match stages.iter_mut().find(|c| c.0 == pos) {
            Some(c) => c.1 = c.1.max(stage),
            None => stages.push((pos, stage)),
        };
        if let Some(m) = &self.mining
            && let Some(stage) = m.stage()
        {
            add(m.pos, u32::from(stage));
        }
        let now = self.world.game_time;
        for (pos, stage, at) in self.destroying.values() {
            if now.saturating_sub(*at) <= 400 {
                add(*pos, u32::from(*stage));
            }
        }
        stages
            .into_iter()
            .filter(|(pos, _)| {
                let d = glam::DVec3::new(f64::from(pos[0]), f64::from(pos[1]), f64::from(pos[2]))
                    + 0.5
                    - camera;
                d.length_squared() <= 1024.0
            })
            .filter(|(pos, _)| {
                let state = self.world.block(pos[0], pos[1], pos[2]);
                self.res.facts().get(state).destroy_overlay
            })
            .filter_map(|(pos, stage)| {
                let name = names.get(stage as usize)?;
                let r = self.gui.atlas.sprites.get(*name)?;
                Some((pos, [r.u0, r.v0]))
            })
            .collect()
    }

    fn sky_darken(&self) -> i32 {
        if self.dim == Dim::Nether {
            return 11;
        }
        if self.dim == Dim::End {
            let sky = self
                .res
                .end_environment()
                .sample(self.world.day_time, None, None);
            return (15.0 - 15.0 * sky.sky_light_level).round() as i32;
        }
        let (rain, thunder) = self.weather_levels(1.0);
        // Sampled once per (time, weather): every mob asks each tick.
        let key = (self.world.day_time, rain.to_bits(), thunder.to_bits());
        if let Some((k, darken)) = self.sky_darken_memo.get()
            && k == key
        {
            return darken;
        }
        let darken = horizon_client::environment::sky_darken(
            self.res.environment(),
            self.world.day_time,
            rain,
            thunder,
        );
        self.sky_darken_memo.set(Some((key, darken)));
        darken
    }
    /// The mob hit along the view, the distance to it and the point the ray
    /// met its box at, measured from the mob's feet (`EntityHitResult
    /// .getLocation() - entity.position()`, the interact packet's location).
    fn mob_hit(&self) -> Option<(usize, f64, [f64; 3])> {
        if let Some(hit) = self.tick_mob {
            return hit;
        }
        self.mob_hit_within(self.entity_reach)
    }
    /// [`Game::mob_hit`] out to `reach` instead of the entity interaction range
    /// (`ProjectileUtil.getHitResultOnViewVector` with another range).
    pub(super) fn mob_hit_within(&self, reach: f64) -> Option<(usize, f64, [f64; 3])> {
        let (eye, dir) = self.aim()?;
        let mut best: Option<(usize, f64, [f64; 3])> = None;
        for (i, m) in self.mobs.iter().enumerate() {
            // ProjectileUtil.getEntityHitResult: the mob the player rides
            // shares its root vehicle and is never the hit, nor is the
            // camera entity the pick starts from. EntitySelector.CAN_BE_PICKED
            // is isPickable — `!isRemoved`, not alive: a health-0 fake player
            // still answers a click.
            if m.removed || Some(m.id) == self.riding || Some(m.id) == self.remote_camera_id() {
                continue;
            }
            if let Some(t) = ray_box(eye, dir, reach, m.bounds())
                && best.is_none_or(|b| t < b.1)
            {
                let point = Vec3::new(
                    eye[0] + dir[0] * t,
                    eye[1] + dir[1] * t,
                    eye[2] + dir[2] * t,
                );
                best = Some((i, t, interact_location(point, m.position)));
            }
        }
        best
    }
    /// `Player.attack`'s damage against `target_kind` (with the attack
    /// strength and whether it is a critical hit): the weapon and effects
    /// scaled by the attack strength, a mace's smash, the critical hit, then
    /// enchantments.
    pub(super) fn melee_damage(
        &mut self,
        target_kind: &str,
        living: bool,
    ) -> (f32, f32, bool, bool) {
        if self.legacy_rules() {
            return self.legacy_melee_damage(target_kind, living);
        }
        let hand_damage = self.attack_damage();
        // Riptide's spin replaces the attack damage attribute.
        let spin = self.player.as_ref().map_or(0.0, |p| p.auto_spin_attack_dmg);
        let damage = if spin > 0.0 { spin } else { hand_damage };
        let scale = self.attack_strength_scale();
        // getEnchantedDamage(target, damage, source) - damage, taken before the
        // attack strength scales the base damage.
        let enchant_bonus = self.enchanted_damage(target_kind, damage) - damage;
        let mut amount = damage * (0.2 + scale * scale * 0.8);
        // Player.attack decides the critical hit only for a hit with base or
        // enchantment damage: checked before the mace's smash bonus is added.
        let damaging = amount > 0.0 || enchant_bonus * scale > 0.0;
        // Item.getAttackDamageBonus (a mace's smash), before the critical hit.
        amount += self.smash_damage_bonus(target_kind);
        let vanilla_crit = living && self.critical_attack(scale);
        let mut crit = vanilla_crit;
        let mut multiplier = if vanilla_crit { 1.5 } else { 1.0 };
        if living && damaging {
            self.native_critical_hit(target_kind, vanilla_crit, &mut crit, &mut multiplier);
        }
        if crit {
            amount *= multiplier;
        }
        // Enchantment damage (sharpness, smite, bane) scaled by the attack strength.
        amount += enchant_bonus * scale;
        (amount, scale, crit, enchant_bonus * scale > 0.0)
    }
    /// `Player.getAttackStrengthScale(0.5)`.
    fn attack_strength_scale(&self) -> f32 {
        // 1.8.9 had no attack cooldown.
        if self.legacy_rules() {
            return 1.0;
        }
        let delay = horizon_client::attack_strength::delay(self.main_hand_attack_speed());
        ((self.attack_ticks as f32 + 0.5) / delay).clamp(0.0, 1.0)
    }
    /// `Player.isSweepAttack`: a charged, grounded sword hit below the movement
    /// limit, which `native_sweep_attack` may force or stop for the hit on `target`.
    pub(super) fn sweep_attack_position(
        &mut self,
        scale: f32,
        crit: bool,
        target: &str,
    ) -> Option<[f64; 3]> {
        // The sweep came with 1.9.
        if self.legacy_rules() {
            return None;
        }
        // Player.getSpeed: the movement speed attribute as it is now (the
        // field is only refreshed by the local movement step, never for a
        // server's player).
        let p = self.player.as_mut()?;
        if let Ok(speed) = p.attribute("minecraft:movement_speed") {
            p.speed = speed as f32;
        }
        if !self.sweep_decision(scale, crit, target) {
            return None;
        }
        let p = self.player.as_ref()?;
        // Mth.sin / Mth.cos of yRot * (PI / 180), from its sine table.
        use horizon_core::geometry::trig;
        let yaw = f64::from(self.yaw * trig::DEG_TO_RAD);
        Some([
            p.position.x - f64::from(trig::sin(yaw)),
            p.position.y + f64::from(p.dimensions.height) * 0.5,
            p.position.z + f64::from(trig::cos(yaw)),
        ])
    }
    /// `Player.isSweepAttack` with the player's speed as it is, then
    /// `native_sweep_attack`, on either side: whether the hit on `target` sweeps.
    pub(super) fn sweep_decision(&mut self, scale: f32, crit: bool, target: &str) -> bool {
        let Some(p) = self.player.as_ref() else {
            return false;
        };
        let sword = self.held().is_some_and(|s| {
            self.res
                .items()
                .item_tags
                .get("minecraft:swords")
                .is_some_and(|tag| tag.contains(&*s.item))
        });
        let vanilla = p.is_sweep_attack(scale, crit, sword);
        let (mut sweeping, mut canceled) = (vanilla, false);
        // `setCanceled` skips later mods and keeps the current `sweeping`.
        self.native_sweep_attack(target, vanilla, &mut sweeping, &mut canceled);
        sweeping
    }
    /// `Player.doSweepAttack` after a successful hit, in vanilla's order: the
    /// sweep sound, the victims around `target` (its box, and the mob that
    /// was hit, when there is one) and then the arc.
    pub(super) fn sweep_attack(
        &mut self,
        at: [f64; 3],
        from: Option<[f64; 3]>,
        target: Option<(Aabb, mace::Spared)>,
        scale: f32,
    ) {
        self.audio
            .play("minecraft:entity.player.attack.sweep", from, 1.0, 1.0);
        if let Some((area, mob)) = target {
            self.sweep_damage(area, mob, scale);
        }
        let arc = horizon_client::particles::ParticleOptions::simple(
            horizon_client::particles::ParticleType::SweepAttack,
        );
        self.particles.add(&arc, at, [0.0; 3]);
        // Player.sweepAttack's `sendParticles(SWEEP_ATTACK, ..., 0, dx, 0, dz, 0)`:
        // the offset is the one from the player to the particle.
        let origin = from.unwrap_or(at);
        let spread = [at[0] - origin[0], 0.0, at[2] - origin[2]];
        self.lan_send_particles(lan_particles::QueuedParticle {
            dimension: self.dim.id(),
            options: arc,
            state: None,
            at,
            spread,
            speed: [0.0; 3],
            count: 0,
            randomization: 0,
        });
    }
    /// `Player.attack`'s `isSweepAttack`, decided before the hit as vanilla does
    /// (`crit`: the hit on a living target is critical): where the sweep plays,
    /// none without one. Reads the charge, so before the hit resets it.
    pub(super) fn sweep_before_hit(&mut self, target: &str, crit: bool) -> Option<[f64; 3]> {
        let scale = self.attack_strength_scale();
        self.sweep_attack_position(scale, crit, target)
    }
    /// `Player.attack`'s sounds when a vehicle takes the hit (it always does, and
    /// can't be hit critically): knockback when sprinting at full strength, then
    /// strong or weak, or the `sweep` decided before the hit around the vehicle's
    /// box `area`. Before the hit resets the charge (`Player.onAttack`).
    pub(super) fn vehicle_attack_sounds(
        &mut self,
        area: Aabb,
        sweep: Option<[f64; 3]>,
        magic: Option<i32>,
    ) {
        self.attack_effects(area, sweep, None, false, mace::Spared::Nothing, magic);
    }
    /// `Player.attack` after a target that is not a mob took the hit:
    /// the sprint knockback sound, a living target's critical hit (`living`:
    /// its entity id; `crit`: whether `melee_damage` decided the hit critical,
    /// mods' `critical_hit` included), the `sweep` decided before the hit
    /// (`sweep_before_hit`) around the target's box `area` and the strong or weak sound.
    pub(super) fn attack_effects(
        &mut self,
        area: Aabb,
        sweep: Option<[f64; 3]>,
        living: Option<i32>,
        crit: bool,
        spared: mace::Spared,
        magic: Option<i32>,
    ) {
        let scale = self.attack_strength_scale();
        let full = scale > 0.9;
        let player = self.player.as_ref();
        let from = player.map(|p| [p.position.x, p.position.y, p.position.z]);
        if full && player.is_some_and(|p| p.sprinting) {
            self.audio
                .play("minecraft:entity.player.attack.knockback", from, 1.0, 1.0);
        }
        let crit = living.is_some() && crit;
        if let Some(at) = sweep {
            self.sweep_attack(at, from, Some((area, spared)), scale);
        }
        if crit {
            self.audio
                .play("minecraft:entity.player.attack.crit", from, 1.0, 1.0);
            if let Some(id) = living {
                self.area_critical_hit(id, area, false);
            }
        } else if sweep.is_none() {
            let sound = if full {
                "minecraft:entity.player.attack.strong"
            } else {
                "minecraft:entity.player.attack.weak"
            };
            self.audio.play(sound, from, 1.0, 1.0);
        }
        // Player.magicCrit: the enchantments added damage.
        if let Some(id) = magic {
            self.area_critical_hit(id, area, true);
        }
    }
    /// `critical_hit` around the box `area` of entity `id` (`magic`: the
    /// enchanted hit instead of the crit).
    fn area_critical_hit(&mut self, id: i32, area: Aabb, magic: bool) {
        let (feet, size) = (area.bottom_center(), area.size());
        self.critical_hit(
            self.lan_acting,
            id,
            [feet.x, feet.y, feet.z],
            size.x,
            size.y,
            magic,
        );
    }
    /// `Player.canCriticalAttack` for a living target at attack strength
    /// `scale`: falling, off the ground, not in water, not on a ladder, not blind (mobility
    /// restricted), not riding, not sprinting.
    pub(super) fn critical_attack(&self, scale: f32) -> bool {
        let player = self.player.as_ref();
        let falling = player.is_some_and(|p| {
            !p.ground.on_ground
                && p.fall_distance > 0.0
                && !p.fluids.was_touching_water
                && !p.on_climbable
                && !p.sprinting
        });
        let restricted = self.potions.effects.get("blindness").is_some() || self.riding.is_some();
        scale > 0.9 && falling && !restricted
    }
    /// Player.attack with the attack-strength charge, crits and knockback.
    fn attack(&mut self) -> bool {
        if self.spectator() {
            return false;
        }
        // A painting nearer than the targeted block and mob breaks instead.
        if let Some((i, d)) = self.painting_hit()
            && self.mob_hit().is_none_or(|m| d < m.1)
            && self.before_block(d)
        {
            self.break_painting(i, true);
            return true;
        }
        // A ghast's fireball or a wind charge in front: turned back.
        if self.attack_redirectable() {
            return true;
        }
        if self.attack_frame()
            || self.attack_armor_stand()
            || self.attack_cushion()
            || self.attack_boat()
            || self.attack_cart()
            || self.attack_dragon()
        {
            return true;
        }
        let Some((index, distance, _)) = self.mob_hit() else {
            return false;
        };
        if !self.before_block(distance) || !self.held_range_allows(distance) {
            return false;
        }
        self.attack_mob(index);
        // Minecraft.startAttack swings after the attack.
        self.swing_arm();
        true
    }
    /// `Player.attack` on a mob: the damage, knockback, sweep, particles and
    /// sounds.
    pub(super) fn attack_mob(&mut self, index: usize) {
        let attack = self.attack_of(self.player.as_ref().map_or(0, |p| p.entity_id));
        let target_kind = self.mobs[index].kind.id.clone();
        let target_kind = format!("minecraft:{}", target_kind.trim_start_matches("minecraft:"));
        let (amount, scale, crit, magic) = self.melee_damage(&target_kind, true);
        let sweep = self.sweep_attack_position(scale, crit, &target_kind);
        let (knockback, knockback_attack) = self.extra_knockback(&target_kind, scale);
        self.attack_ticks = 0;
        let player = self.player.as_ref();
        let from = player.map(|p| [p.position.x, p.position.y, p.position.z]);
        if knockback_attack {
            self.audio
                .play("minecraft:entity.player.attack.knockback", from, 1.0, 1.0);
        }
        // LivingEntity.setLastHurtMob (tamed wolves join in).
        self.combat.last_hurt_mob = Some((self.mobs[index].id, self.world.game_time));
        let old_health = self.mobs[index].health;
        let source = match from {
            Some(f) => self.attack_source(f),
            None => horizon_client::mobs::Damage::of(horizon_client::mobs::DamageKind::Generic),
        };
        let landed = self.hurt_by_player(index, amount, source);
        if landed && old_health > 0.0 && !self.mobs[index].alive() {
            self.native_kill_event(index);
        }
        if landed {
            let victim = self.mob_entity(&self.mobs[index]);
            self.player_hit(
                &victim,
                creatures::damage_type(source.kind, true),
                None,
                amount,
            );
        }
        if let Some(p) = self.player.as_mut() {
            if p.attack_log.len() >= 64 {
                p.attack_log.remove(0);
            }
            p.attack_log.push(horizon_client::health::AttackRecord {
                game_time: p.health.game_time(),
                target: target_kind.clone(),
                id: self.mobs[index].id,
                killed: landed && !self.mobs[index].alive(),
                amount,
                scale,
                crit,
                landed,
                dealt: old_health - self.mobs[index].health,
            });
        }
        if landed {
            if knockback > 0.0 {
                let (xd, zd) = self.extra_knockback_direction();
                if self.legacy_rules() {
                    let m = &mut self.mobs[index];
                    let v = [m.velocity.x, m.velocity.y, m.velocity.z];
                    let [x, y, z] = horizon_client::legacy_combat::extra_knockback(
                        v,
                        f64::from(knockback),
                        (xd, zd),
                    );
                    m.velocity = Vec3::new(x, y, z);
                    m.needs_sync = true;
                } else {
                    let level = crate::game::entity::damage_level_of!(self, &attack);
                    self.mobs[index].knockback_effect(
                        level,
                        f64::from(knockback),
                        xd,
                        zd,
                        &source,
                        amount,
                    );
                }
                self.stop_sprint_after_extra_knockback();
            }
            let mob = &self.mobs[index];
            // Player.damageStatsAndHearts: the damage the hit actually did.
            let actual = old_health - mob.health;
            // Player.attack: crit emitter and damage indicator hearts.
            let (feet, width, height) = (
                [mob.position.x, mob.position.y, mob.position.z],
                mob.kind.width,
                mob.kind.height,
            );
            self.award_custom_stat("damage_dealt", (actual * 10.0).round() as i32);
            if crit {
                self.critical_hit(
                    self.lan_acting,
                    self.mobs[index].id as i32,
                    feet,
                    width,
                    height,
                    false,
                );
            }
            // Player.attackVisualEffects: magicCrit when the enchantments added damage.
            if magic {
                self.critical_hit(
                    self.lan_acting,
                    self.mobs[index].id as i32,
                    feet,
                    width,
                    height,
                    true,
                );
            }
            if actual > 2.0 {
                let indicator = horizon_client::particles::ParticleOptions::simple(
                    horizon_client::particles::ParticleType::DamageIndicator,
                );
                self.particles.send_particles(
                    &indicator,
                    [feet[0], feet[1] + height * 0.5, feet[2]],
                    [0.1, 0.0, 0.1],
                    0.2,
                    (actual * 0.5) as u32,
                );
                // The players a server serves are sent them too.
                self.lan_send_particles(lan_particles::QueuedParticle {
                    dimension: self.dim.id(),
                    options: indicator,
                    state: None,
                    at: [feet[0], feet[1] + height * 0.5, feet[2]],
                    spread: [0.1, 0.0, 0.1],
                    speed: [0.2; 3],
                    count: (actual * 0.5) as u32 as i32,
                    randomization: 0,
                });
            }
            self.remember_mob_player_credit(self.mobs[index].id);
            let sound = if crit {
                "minecraft:entity.player.attack.crit"
            } else if scale > 0.9 {
                "minecraft:entity.player.attack.strong"
            } else {
                "minecraft:entity.player.attack.weak"
            };
            // Player.playServerSideSound: at the player. A critical hit a mod
            // let sweep has both sounds (attackVisualEffects).
            if let Some(at) = sweep {
                let target = (
                    self.mobs[index].bounds(),
                    mace::Spared::Mob(self.mobs[index].id),
                );
                self.sweep_attack(at, from, Some(target), scale);
            }
            if crit || sweep.is_none() {
                self.audio.play(sound, from, 1.0, 1.0);
            }
            // The hurt or death sound the mob raised (getHurtSound / getDeathSound,
            // its voice pitch from its own random).
            let queued = std::mem::take(&mut self.mobs[index].outbox);
            let mut infested = false;
            for event in queued {
                match event {
                    horizon_client::mobs::MobEvent::Sound(s, at, volume, pitch) => {
                        self.audio
                            .play(&format!("minecraft:{s}"), Some(at), volume, pitch);
                    }
                    horizon_client::mobs::MobEvent::Infested { .. } => infested = true,
                    other => {
                        // Left for the end of the tick.
                        self.mobs[index].outbox.push(other);
                    }
                }
            }
            if infested {
                let id = self.mobs[index].id;
                self.mob_infested(id);
            }
            self.weapon_hit(index);
            // Player.attack: causeFoodExhaustion(0.1F) only when the hit hurt
            // (inside `if (wasHurt)`), after damageStatsAndHearts.
            if let Some(p) = self.player.as_mut()
                && !self.creative
            {
                p.cause_food_exhaustion(0.1);
            }
        } else {
            self.audio
                .play("minecraft:entity.player.attack.nodamage", from, 1.0, 1.0);
        }
    }
    /// ServerExplosion: rays from a 16^3 grid lose (resistance + 0.3) * 0.3 per step.
    /// A mob explosion (`ServerLevel.explode` with `ExplosionInteraction.MOB`:
    /// blocks are kept without mobGriefing, else the mob_explosion_drop_decay
    /// rule picks decay). NeoForge asks `canEntityGrief` for `source`, the
    /// exploding entity (the rule alone without one).
    fn explode(&mut self, source: Option<u32>, center: [f64; 3], radius: f32) -> Result<()> {
        self.explode_with(source, center, radius, false)
    }
    /// `ServerLevel.explode` with `ExplosionInteraction.MOB` and the caller's fire flag.
    fn explode_mob(&mut self, center: [f64; 3], radius: f32, fire: bool) -> Result<()> {
        self.explode_with(None, center, radius, fire)
    }
    fn explode_with(
        &mut self,
        source: Option<u32>,
        center: [f64; 3],
        radius: f32,
        fire: bool,
    ) -> Result<()> {
        let interaction = if !self.can_entity_grief(source) {
            explosion::BlockInteraction::Keep
        } else {
            self.destroy_type(horizon_client::gamerules::vanilla::MOB_EXPLOSION_DROP_DECAY)
        };
        self.server_explode(center, radius, interaction, fire)
    }
    /// `SulfurCube.tickFuse`: `ExplosionInteraction.TNT` (the
    /// tnt_explosion_drop_decay rule) with mobGriefing, `NONE` without.
    fn explode_sulfur_cube(
        &mut self,
        source: Option<u32>,
        center: [f64; 3],
        radius: f32,
    ) -> Result<()> {
        let interaction = if !self.can_entity_grief(source) {
            explosion::BlockInteraction::Keep
        } else {
            self.destroy_type(horizon_client::gamerules::vanilla::TNT_EXPLOSION_DROP_DECAY)
        };
        self.server_explode(center, radius, interaction, false)
    }
    /// Copies of a save batch carrying its live block entities and save data.
    fn with_entities<'a>(
        &self,
        chunks: impl IntoIterator<Item = &'a Arc<horizon_client::chunk::Chunk>>,
    ) -> Vec<Arc<horizon_client::chunk::Chunk>> {
        horizon_client::save::chunks_with_block_entities(
            chunks,
            &self.world.block_entities,
            &self.world.block_names,
        )
        .into_iter()
        .map(|mut c| {
            self.attach_save_data(&mut c);
            Arc::new(c)
        })
        .collect()
    }
    fn tick_mobs(&mut self) -> Result<()> {
        self.sync_mob_death_protection();
        self.tick_creatures()?;
        if !self.entity_attachments.is_empty() {
            self.prune_entity_attachments();
        }
        self.native_mobs_equipment();
        self.tick_equipment_attributes();
        self.position_rider();
        self.tick_player_stat_counters();
        self.tick_scoreboard();
        self.load_bossbars();
        self.tick_worldborder();
        self.tick_subtitles();
        self.spawn_phantoms();
        self.tick_advancements();
        self.tick_player_model();
        Ok(())
    }
    /// SkyRenderer.renderSunMoonAndStars: bodies on a frame rotated Y -90 then X by angle.
    /// Returns the sun and moon quads, and the stars' frame and brightness
    /// (the renderer keeps their geometry, built once). The bodies fade with
    /// the rain (`rainBrightness = 1 - rainLevel`, their colour's alpha).
    fn celestial_geometry(
        &self,
        sky: &horizon_client::environment::Sky,
        rain: f32,
    ) -> (Vec<Vertex>, Option<(glam::Mat4, f32)>) {
        let body_color =
            0x00FF_FFFF | (((1.0 - rain).clamp(0.0, 1.0) * 255.0).round() as u32) << 24;
        let mut out = Vec::new();
        let atlas = &self.gui.atlas;
        let frame = |angle: f32| {
            glam::Mat4::from_rotation_y((-90.0f32).to_radians())
                * glam::Mat4::from_rotation_x(angle.to_radians())
        };
        let mut body = |name: &str, angle: f32, size: f32, moon: bool| {
            let Some(r) = atlas.sprites.get(name).copied() else {
                return;
            };
            // How brightly HDR output lets the body shine (mesher::hdr_tier):
            // the moon at the low tier, not the sun's.
            let tier = if moon {
                horizon_client::mesher::LOW
            } else {
                horizon_client::mesher::VERY_BRIGHT
            };
            let m = frame(angle);
            let corners = [(-size, -size), (size, -size), (size, size), (-size, size)];
            // buildCelestialQuad (sun) vs buildMoonPhases: the moon's UVs run
            // the other way round.
            let uv = if moon {
                [[r.u1, r.v1], [r.u0, r.v1], [r.u0, r.v0], [r.u1, r.v0]]
            } else {
                [[r.u0, r.v0], [r.u1, r.v0], [r.u1, r.v1], [r.u0, r.v1]]
            };
            for ((x, z), uv) in corners.iter().zip(uv) {
                let p = m.transform_point3(glam::Vec3::new(*x, 100.0, *z));
                out.push(Vertex {
                    normal: [0; 4],
                    pos: p.to_array(),
                    uv,
                    layer: 2 | u32::from(tier) << 16,
                    color: body_color,
                    light: 0,

                    glint_uv: [0.0, 0.0],
                });
            }
        };
        body("environment/celestial/sun", sky.sun_angle, 30.0, false);
        let phases = [
            "full_moon",
            "waning_gibbous",
            "third_quarter",
            "waning_crescent",
            "new_moon",
            "waxing_crescent",
            "first_quarter",
            "waxing_gibbous",
        ];
        let phase = phases[(self.view_day_time() / 24000).rem_euclid(8) as usize];
        body(
            &format!("environment/celestial/moon/{phase}"),
            sky.moon_angle,
            20.0,
            true,
        );
        // SkyRenderer.renderStars: the static star buffer on the same frame.
        let stars =
            (sky.star_brightness > 0.0).then(|| (frame(sky.star_angle), sky.star_brightness));
        (out, stars)
    }
    /// `onAbort`: the confirm screen's Back or Escape reopens the Data
    /// Packs screen with the selection it had.
    fn reopen_data_packs(&mut self, event_loop: &dyn EventLoopCtl) -> Result<()> {
        let action = self
            .title
            .as_mut()
            .and_then(|t| t.reopen_data_packs_action());
        if let Some(action) = action {
            self.title_action(event_loop, action)?;
        }
        Ok(())
    }
    /// What the title screen asked for, from a click or Enter.
    fn title_action(&mut self, event_loop: &dyn EventLoopCtl, action: Action) -> Result<()> {
        match action {
            // A splitscreen player's title never ends the process (P1's does).
            Action::Quit if self.options.split_guest => {}
            Action::Quit => event_loop.exit(),
            Action::Options => self.open_options(true),
            Action::Accessibility => {
                self.open_options_at(true, horizon_client::options_ui::Page::Accessibility)
            }
            Action::Language => {
                self.open_options_at(true, horizon_client::options_ui::Page::Language)
            }
            Action::Sounds => self.open_options_at(true, horizon_client::options_ui::Page::Sound),
            Action::Credits => {
                self.open_options_at(true, horizon_client::options_ui::Page::Credits)
            }
            Action::OpenUri(uri) => packs::open_uri(&uri),
            Action::CopyText(text) => {
                use horizon_client::clipboard::Clipboard;
                self.clipboard.set(&text);
            }
            Action::Play { folder } => self.open_world(&folder, None, None, false, None)?,
            Action::Create {
                folder,
                name,
                seed,
                creative,
                options,
            } => self.open_world(&folder, Some(&name), seed, creative, Some(&options))?,
            Action::Connect { address, lan } => {
                self.connect_server_with_options(&address, &self.options.player_name(), None, lan)?
            }
            Action::DataPacks { dir, selected } => {
                self.keys.clear();
                self.set_screen(Screen::Options);
                self.options_from_title = true;
                let resources = self.vanilla_root();
                let additional = self
                    .discover_mod_packs(mod_api::PackType::ServerData)
                    .into_iter()
                    .filter_map(|pack| pack.selection)
                    .collect();
                self.ui
                    .options
                    .open_data_packs(dir, selected, resources, additional);
            }
            Action::None => {}
        }
        Ok(())
    }
    /// Enter a local world from the client: it runs on the client's server thread.
    fn open_world(
        &mut self,
        folder: &str,
        name: Option<&str>,
        seed: Option<i64>,
        creative: bool,
        create: Option<&horizon_client::title::WorldOptions>,
    ) -> Result<()> {
        // openWorldLoadLevelData: an existing world whose level data does not read opens
        // RecoverWorldDataScreen instead.
        if create.is_none()
            && let Some(title) = self.title.as_mut()
            && title.open_recover(folder)
        {
            return Ok(());
        }
        world_loading::world_data_used();
        if self.defer_world_entry(PendingWorldEntry::Local {
            folder: folder.to_owned(),
            name: name.map(str::to_owned),
            seed,
            creative,
            create: create.cloned().map(Box::new),
        })? {
            return Ok(());
        }
        self.join_world_rest()?;
        // Create World's data packs, in the world before it opens.
        if let Some(dir) = create.and_then(|o| o.pack_dir.as_deref()) {
            let world = std::env::current_dir()?.join("saves").join(folder);
            let packs = create.map_or(&[][..], |o| &o.data_packs[..]);
            if let Err(e) = horizon_client::datapack::install(dir, &world, packs) {
                log::warn!("Horizon: data packs for {folder}: {e}");
            }
            let _ = std::fs::remove_dir_all(dir);
        }
        // The world's pack selection (`DataPackConfig`), for the pack stack
        // to read: the picked packs on, the feature packs left out off.
        // Only when `install` did not run (no Data Packs screen opened): it
        // writes the file itself, and its `disabled` must not be lost.
        if let Some(o) = create.filter(|o| o.pack_dir.is_none()) {
            let world = std::env::current_dir()?.join("saves").join(folder);
            if let Err(e) = std::fs::create_dir_all(&world) {
                log::warn!("Horizon: data packs for {folder}: {}: {e}", world.display());
            } else {
                let disabled: Vec<String> =
                    horizon_client::datapack::feature_packs(&self.vanilla_root())
                        .into_iter()
                        .map(|p| p.id)
                        .filter(|id| !o.data_packs.contains(id))
                        .collect();
                let json = serde_json::json!({"enabled": o.data_packs, "disabled": disabled});
                let file = world.join("datapacks.json");
                if let Err(e) = std::fs::write(&file, json.to_string()) {
                    log::warn!("Horizon: data packs for {folder}: {}: {e}", file.display());
                }
            }
        }
        // IntegratedServer.initServer's line, which other launchers read in `latest.log` to
        // offer the world again.
        horizon_client::logging::write_as(
            "Server thread",
            log::Level::Info,
            &format!(
                "Starting integrated minecraft server version {}",
                horizon_core::VANILLA_VERSION
            ),
        );
        self.open_integrated(folder, name, seed, creative, create)
    }

    /// Load authoritative world state synchronously, without client entry or
    /// integrated-server selection. Server startup must use this entry point.
    fn load_world(&mut self, request: world_loading::WorldRequest) -> Result<()> {
        self.join_world_rest()?;
        let prepared = request.prepare(
            &self.res,
            &self.pack,
            &self.options,
            &self.settings,
            &horizon_world::datafix::progress::UpgradeProgress::new(),
        )?;
        self.finish_open_world(prepared)
    }

    fn finish_open_world(&mut self, prepared: world_loading::PreparedWorld) -> Result<()> {
        self.reset_tutorial();
        // Another world: its generator, not the last one's Density line.
        self.debug_density.borrow_mut().clear();
        let world_loading::PreparedWorld {
            save_root,
            shapes,
            level,
            world,
            streaming,
            save,
            session_lock,
            rand_sequences,
            locator,
            loot,
        } = prepared;
        self.server_loot = loot;
        let dim = Dim::from_id(&level.dimension);
        self.level_shapes = shapes;
        self.world_root = save_root.clone();
        // The root moves before the prepared level is installed. Until then a
        // final save must not run: the lock below only marks tag tables.
        self.world_adopted = false;
        // The world's packs over the game's biomes (`Biomes::with_packs`),
        // the table its chunks were built with.
        self.world_biomes = self.packed_biomes();
        self.world_biomes_base = self.world_biomes.clone();
        horizon_client::crash::set_detail(
            "World",
            format!("{} ({})", level.name, save_root.display()),
        );
        self.end.set_locator(locator);
        self.share_spawn_conditions();
        self.score = Default::default();
        let stack = self.reload_datapacks(false, &level.enabled_features)?;
        let root = self.options.resources.clone();
        (self.world_tables, self.health_resources) =
            self.build_world_tables(&stack, false, &root, self.mods.data_switched)?;
        self.damage_tag_generation = self.damage_tag_generation.wrapping_add(1);
        self.resolve_parts()?;
        // Native tag reads use the lock to tell that the tag tables are ready.
        self.session_lock = Some(session_lock);
        // Keep the no-pack preload; a world's packs replace it atomically.
        // A later /reload keeps this session's synchronized registries.
        lan::open_world_sync_data(
            &self.options.resources,
            &self.res.core.item_data,
            &pack_files(&self.world_tables),
            &self.health_resources.damage,
        )
        .map_err(|e| format!("world registries: {e}"))?;
        // Warm the loot data with this world's packs before chunks decode
        // (`save::roll_loaded_loot` reads the cached copy).
        self.load_server_loot();
        // MinecraftServer's constructor reads CustomBossEvents before any
        // server event.
        self.bosses = Default::default();
        self.load_bossbars();
        // ServerLevel's constructor reads its Raids saved data before any
        // server event; their randoms are seeded on the first raid tick.
        if self.remote.is_none() {
            self.read_raids();
        }
        self.native_server_data_loaded(false);
        self.sync_recipe_content();
        reset_world_progress(&mut self.adv, &mut self.stats);
        self.prefetch_advancement_tree(&stack);
        self.restore_player_state = true;
        self.rand_sequences = rand_sequences;
        self.border = Default::default();
        self.set_mode_id(level.mode());
        self.previous_mode = level.previous_game_mode.map(game_mode::mode_from_id);
        let mut inventory = self.load_saved_inventory(&level);
        if self.creative && level.inventory.is_empty() && level.position.is_none() {
            for (i, name) in [
                "minecraft:grass_block",
                "minecraft:stone",
                "minecraft:oak_planks",
                "minecraft:glass",
                "minecraft:torch",
                "minecraft:oak_log",
                "minecraft:chest",
                "minecraft:furnace",
                "minecraft:diamond_pickaxe",
            ]
            .iter()
            .enumerate()
            {
                inventory[i] = Some(ItemStack {
                    item: Arc::from(*name),
                    count: 1,
                    damage: 0,
                    extra: None,
                });
            }
        }
        let (yaw, pitch) = self
            .options
            .look
            .unwrap_or((level.rotation[0], level.rotation[1]));
        self.yaw = yaw;
        self.pitch = pitch;
        self.selected = level.selected_slot.clamp(0, 8) as usize;
        self.streaming = streaming;
        self.world = world;
        self.inventory = inventory;
        self.save = save;
        // The ride the player's file names, once the vehicle is in the world.
        self.pending_vehicle = level.root_vehicle;
        self.level = level;
        self.world_adopted = true;
        // ServerRecipeBook.loadUntrusted: recipes the manager does not have are dropped.
        let index = &world_stations!(self).recipe_index;
        let book = &mut self.level.recipe_book;
        book.recipes.retain(|r| index.contains(r));
        book.to_be_displayed.retain(|r| index.contains(r));
        self.dim = dim;
        self.audio.set_world(dim.id());
        // LevelStorageAccess.getIconFile: this save's icon.png is what the
        // Select World list shows.
        self.world_icon = Some(WorldIcon {
            path: save_root.join("icon.png"),
            last_attempt: None,
        });
        self.world_flat = horizon_client::save::WorldGen::read(&save_root)
            .is_some_and(|w| w.preset == horizon_client::save::WorldPreset::Flat);
        self.world_debug = horizon_client::save::WorldGen::read(&save_root)
            .is_some_and(|w| w.preset == horizon_client::save::WorldPreset::Debug);
        self.block_ticks = Ticks::new(self.level.seed as i32);
        self.tick_chunks.clear();
        self.clear_entities();
        self.player = None;
        self.sleeping = None;
        self.sleep_timer = 0;
        self.player_beds.clear();
        self.spawn = None;
        self.respawn_config = self
            .level
            .respawn_elsewhere
            .as_ref()
            .map(respawn_anchor::RespawnConfig::from_saved);
        self.spawn_origin = None;
        self.spawn_hook_fired = false;
        self.spawn_hook_cancelled = false;
        self.spawn_spiral = Default::default();
        self.player_spawn = None;
        self.creatures.reset_trader();
        self.creatures.cat_tick = 0;
        self.creatures.custom_spawners.clear();
        self.portals = Default::default();
        self.experience = Experience {
            level: self.level.xp_level,
            progress: self.level.xp_progress,
            total: self.level.xp_total,
        };
        self.restore_effects();
        self.mining = None;
        self.container = None;
        let kinds = std::mem::take(&mut self.be_anims.kinds);
        self.be_anims = horizon_client::block_entity_anim::BlockEntityAnims {
            kinds,
            ..Default::default()
        };
        self.carried = None;
        self.using = None;
        self.set_screen(Screen::Playing);
        self.hide_title();
        self.clear_lod();
        self.pending_uploads.clear();
        if let Some(r) = self.renderer.as_mut() {
            r.clear_sections(self.world.layout());
        }
        self.apply_settings();
        self.streaming.sky_light = dim != Dim::Nether;
        self.streaming.nether_shading = dim == Dim::Nether;
        self.load_world_state();
        // The level the world opens in, before any of its chunks.
        self.native_level_opened(dim);
        Ok(())
    }
    /// PortalShape: an obsidian frame around 2..=21 by 3..=21 empty blocks.
    fn portal_shape(&self, start: [i32; 3], x_axis: bool) -> Option<([i32; 3], i32, i32)> {
        let name = |p: [i32; 3]| {
            self.res
                .facts()
                .get(self.world.block(p[0], p[1], p[2]))
                .name
                .clone()
        };
        let empty = |p: [i32; 3]| {
            let n = name(p);
            matches!(
                &*n,
                "minecraft:air"
                    | "minecraft:cave_air"
                    | "minecraft:fire"
                    | "minecraft:soul_fire"
                    | "minecraft:nether_portal"
            )
        };
        let frame = |p: [i32; 3]| &*name(p) == "minecraft:obsidian";
        let d = if x_axis { [1, 0, 0] } else { [0, 0, 1] };
        let mut bottom = start;
        let mut n = 0;
        while empty([bottom[0], bottom[1] - 1, bottom[2]]) && n < 21 {
            bottom[1] -= 1;
            n += 1;
        }
        if !frame([bottom[0], bottom[1] - 1, bottom[2]]) {
            return None;
        }
        let mut left = bottom;
        n = 0;
        while empty([left[0] - d[0], left[1], left[2] - d[2]]) && n < 21 {
            left = [left[0] - d[0], left[1], left[2] - d[2]];
            n += 1;
        }
        if !frame([left[0] - d[0], left[1], left[2] - d[2]]) {
            return None;
        }
        let mut width = 0;
        while width <= 21 {
            let p = [left[0] + d[0] * width, left[1], left[2] + d[2] * width];
            if !empty(p) || !frame([p[0], p[1] - 1, p[2]]) {
                break;
            }
            width += 1;
        }
        if !(2..=21).contains(&width)
            || !frame([left[0] + d[0] * width, left[1], left[2] + d[2] * width])
        {
            return None;
        }
        let mut height = 0;
        'rows: while height <= 21 {
            let y = left[1] + height;
            if !frame([left[0] - d[0], y, left[2] - d[2]])
                || !frame([left[0] + d[0] * width, y, left[2] + d[2] * width])
            {
                break;
            }
            for i in 0..width {
                if !empty([left[0] + d[0] * i, y, left[2] + d[2] * i]) {
                    break 'rows;
                }
            }
            height += 1;
        }
        let top_ok =
            (0..width).all(|i| frame([left[0] + d[0] * i, left[1] + height, left[2] + d[2] * i]));
        ((3..=21).contains(&height) && top_ok).then_some((left, width, height))
    }
    /// BaseFireBlock.onPlace: lighting fire inside a frame creates a portal,
    /// only in the Overworld and the Nether (`inPortalDimension`).
    fn try_light_portal(&mut self, at: [i32; 3]) -> Result<bool> {
        if !matches!(self.dim, Dim::Overworld | Dim::Nether) {
            return Ok(false);
        }
        for x_axis in [true, false] {
            if let Some((origin, width, height)) = self.portal_shape(at, x_axis) {
                let d = if x_axis { [1, 0, 0] } else { [0, 0, 1] };
                let registry = &self.pack.assets.registry;
                let Some(default) = registry.default_state("minecraft:nether_portal") else {
                    return Ok(false);
                };
                let mut portal = registry.state(default).cloned().ok_or("portal state")?;
                portal
                    .properties
                    .insert("axis".into(), if x_axis { "x" } else { "z" }.into());
                let id = registry.resolve(&portal).unwrap_or(default);
                // PortalShape.createPortalBlocks uses BlockPos.betweenClosed:
                // X varies fastest, then Y, then Z.
                let first = BlockPos::new(origin[0], origin[1], origin[2]);
                let last = BlockPos::new(
                    origin[0] + d[0] * (width - 1),
                    origin[1] + height - 1,
                    origin[2] + d[2] * (width - 1),
                );
                for pos in BlockPos::between_closed(first, last).expect("portal size is bounded") {
                    self.level_set_with_flags(
                        [pos.x, pos.y, pos.z],
                        id,
                        level_flags::UPDATE_CLIENTS | level_flags::UPDATE_KNOWN_SHAPE,
                    )?;
                }
                return Ok(true);
            }
        }
        Ok(false)
    }
    /// Leaving the game with a world open (window closed, Quit, `quit`):
    /// menu items go back (Player.closeContainer), then the world is saved,
    /// once.
    pub(super) fn save_for_exit(&mut self) -> Result<()> {
        // Native application termination can bypass CloseRequested. Finish any
        // save screen synchronously here, before the process can go away.
        while self.pending_save.is_some() {
            self.poll_closing_save()?;
            if self.pending_save.is_some() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        // A repro recording is finished and bundled before the process may go away.
        self.repro_on_exit();
        if self.title.is_some() || self.saved_for_exit {
            return Ok(());
        }
        // exitWorldAndClose disconnects the client from its world.
        self.client_logging_out();
        // Native application termination can bypass the close request, so make the same
        // client-side worker abort explicit before the game-owned streaming state is dropped.
        self.streaming.abort_generation();
        if let Some(server) = self.integrated.take() {
            // The server thread saves the world.
            server.stop()?;
            self.saved_for_exit = true;
            return Ok(());
        }
        if let Some(mut remote) = self.remote.take() {
            // Minecraft.disconnect closes the connection, so the host sees a clean leave.
            remote.session.disconnect();
            self.saved_for_exit = true;
            return Ok(());
        }
        self.close_container();
        if self.remote.is_none() {
            self.award_custom_stat("leave_game", 1);
        }
        self.lan_close(false);
        self.free_player_beds()?;
        self.save_game()?;
        self.saved_for_exit = true;
        Ok(())
    }
    fn close_world(&mut self) -> Result<()> {
        // A recording cannot cross into another world, and a replay feeding the
        // title screen is nonsense.
        self.repro_world_closed();
        self.clear_waypoints();
        self.post_effects.clear();
        self.spectated_post_effect = None;
        if self.external_server() {
            return self.leave_server(None);
        }
        self.begin_closing_save(false)
    }
    fn finish_close_world(&mut self) -> Result<()> {
        self.ambience.clear(&mut self.audio);
        // updateLevelInEngines(null): every sound stops; the music manager
        // lets go of its track.
        self.music.stop_playing(None, &mut self.audio);
        self.audio.stop_all();
        if self.remote.is_some() {
            self.world_icon = None;
            return self.leave_server(None);
        }
        // The save screen has finished every write before releasing storage.
        self.ui.dispatcher = Default::default();
        self.server_loot.set(None);
        self.server_items = None;
        self.session_lock = None;
        mod_api::__private::clear_game_parts();
        self.world_adopted = false;
        self.world_icon = None;
        horizon_client::crash::clear_detail("World");
        self.debug_charts.reset();
        self.boss_overlay.reset();
        // GameRenderer.resetData (Minecraft.clearClientLevel).
        self.item_activation.reset();
        self.kept_block_entities.get_mut().clear();
        self.mod_view_distances.take();
        self.bosses = Default::default();
        self.streaming.paused = true;
        self.world = World::new(self.res.facts().air, 0);
        self.player = None;
        // `LocalPlayer`'s constructor: `KeyMapping.resetToggleKeys`, so a sneak or sprint toggled
        // in this world is not still on in the next one.
        self.reset_toggle_keys();
        self.sleeping = None;
        self.sleep_timer = 0;
        self.player_beds.clear();
        self.clear_entities();
        self.set_screen(Screen::Playing);
        self.clear_lod();
        self.pending_uploads.clear();
        if let Some(r) = self.renderer.as_mut() {
            r.clear_sections(self.world.layout());
        }
        // Clear sync data (registries, dialog tags) so dialogs don't leak
        // between worlds.
        lan::forget_sync_data();
        lan::forget_server_sources();
        let mut title = Title::new(
            std::env::current_dir()?.join("saves"),
            &self.options.resources,
        )
        .with_user(|| self.options.player_name());
        title.last_server = self.settings.file.string("lastServer");
        title.skip_multiplayer_warning = self.settings.file.bool("skipMultiplayerWarning");
        self.show_title(title);
        self.capture(false);
        Ok(())
    }
    /// Let go of every entity of the level (saved first): the next world
    /// opened must not inherit them, nor save them as its own.
    pub(super) fn clear_entities(&mut self) {
        self.mobs.clear();
        self.dropped.clear();
        self.orbs.clear();
        self.creatures.projectiles.clear();
        self.falling.clear();
        self.transient_blocks.clear();
        self.tnts.clear();
        self.boats.clear();
        self.carts.clear();
        self.armor_stands.clear();
        self.cushions.clear();
        self.item_frames.clear();
        self.paintings.clear();
        self.others.clear();
        self.dragon = None;
        self.crystals.clear();
        self.dragon_fireballs.clear();
        self.breath_clouds.clear();
        self.tridents.clear();
        self.rockets.clear();
        self.bolts.clear();
        self.clear_client_particles();
        self.riding = None;
        self.persist = Default::default();
        self.missing_professions.clear();
        self.entity_attachments.clear();
        self.clear_remote_objects();
        self.clear_remote_projectiles();
        self.levels.clear();
        self.entity_vertices.clear();
        self.see_through_vertices.clear();
        self.glowing_vertices.get_mut().clear();
        self.shadow_only_vertices.get_mut().clear();
        self.entity_spans.borrow_mut().clear();
        self.entity_motion.borrow_mut().clear();
        self.entity_motion_posed.borrow_mut().clear();
        self.skin_models
            .borrow_mut()
            .restart(&mut self.skin_frame.borrow_mut());
        self.skin_frame.borrow_mut().clear_draws();
    }
    /// Equipment attribute modifiers (LivingEntity.collectEquipmentChanges).
    fn apply_equipment(&mut self) {
        let Some(player) = self.player.as_mut() else {
            return;
        };
        for (attribute, id) in std::mem::take(&mut self.applied_modifiers) {
            let _ = player.set_modifier_unclamped(&attribute, &id, 0.0, "add_value", false);
        }
        let slots = ["head", "chest", "legs", "feet"];
        let mut equipped: Vec<(Option<&ItemStack>, &str)> = self
            .armor
            .iter()
            .zip(slots)
            .map(|(s, n)| (s.as_ref(), n))
            .collect();
        equipped.push((self.inventory[self.selected].as_ref(), "mainhand"));
        equipped.push((self.offhand.as_ref(), "offhand"));
        for (stack, slot) in equipped {
            let Some(stack) = stack else {
                continue;
            };
            for horizon_items::item_data::ModifierRow {
                attribute,
                id,
                amount,
                operation: op,
                slot: group,
            } in horizon_client::item_attributes::attribute_modifiers(stack, world_items!(self))
                .iter()
            {
                // The attack code still reads the held item's attack damage and
                // speed directly; the map copy is what `/attribute` reports.
                if !horizon_client::living_attributes::modifier_applies(group, slot) {
                    continue;
                }
                if player
                    .set_modifier_unclamped(attribute, id, *amount, op, true)
                    .is_ok()
                {
                    self.applied_modifiers.push((attribute.clone(), id.clone()));
                }
            }
        }
        let _ = player.clamp_health_to_max();
        self.apply_enchantment_attributes();
    }
    /// The armor hits among a tick's motion events (`Player.hurtArmor` from
    /// `getDamageAfterArmorAbsorb`), whatever the damage: a mob, an explosion,
    /// a cactus, lava, a campfire.
    fn wear_armor(&mut self, events: &[MotionEvent]) {
        for event in events {
            if let MotionEvent::Health(horizon_client::health::HealthEvent::ArmorHurt {
                source,
                amount,
            }) = event
            {
                self.hurt_armor(*source, *amount);
            }
        }
    }
    /// `Player.hurtArmor` -> `LivingEntity.doHurtEquipment`: each worn piece that
    /// wears when its wearer is hurt loses max(1, damage / 4) durability, except
    /// to a fire source when the piece resists fire (`ItemStack.canBeHurtBy`).
    fn hurt_armor(&mut self, source: horizon_world::damage_source::DamageTypeId, damage: f32) {
        if damage <= 0.0 {
            return;
        }
        let amount = ((damage / 4.0) as u32).max(1);
        let name = self
            .health_resources
            .damage
            .definition(source)
            .map(|d| d.name.clone())
            .unwrap_or_default();
        let fire = self
            .health_resources
            .damage
            .in_tag(source, "minecraft:is_fire");
        let mut broke = false;
        let mut broken_items = Vec::new();
        // The pieces that may wear (`doHurtEquipment`'s checks).
        let mut eligible: [Option<ItemStack>; 4] = std::array::from_fn(|i| {
            let stack = self.armor[i].clone()?;
            let info = world_items!(self).get(&stack.item)?;
            let component = |id| stack.data().and_then(|data| data.other(id));
            // ItemStack.isDamageableItem: component removal and
            // unbreakable are checked before enchantment RNG or hooks.
            if stack.count == 0
                || !info.equip_damage_on_hurt
                || component("minecraft:unbreakable").is_some()
                || component("!minecraft:max_damage").is_some()
                || component("!minecraft:damage").is_some()
                || (fire && world_items!(self).resists_fire(&stack, &name))
            {
                return None;
            }
            let max = match component("minecraft:max_damage") {
                Some(horizon_core::nbt::Tag::Int(value)) => u32::try_from(*value).ok()?,
                _ => info.max_damage,
            };
            if max == 0 {
                return None;
            }
            Some(stack)
        });
        // ArmorHurtEvent: once for the pieces that would wear, a server player's only.
        let mut amounts = [amount; 4];
        if mod_api::__private::listened(mod_api::HookId::ArmorHurt)
            && let (Some(uuid), Some(player)) = (self.lan_acting_uuid, self.lan_acting_entity)
        {
            let mut entries: Vec<mod_api::NativeArmorHurt> = eligible
                .iter()
                .enumerate()
                .filter_map(|(i, e)| {
                    Some(mod_api::NativeArmorHurt {
                        slot: armor_slot(i),
                        stack: e.as_ref()?.clone(),
                        damage: durability_damage(damage),
                        new_damage: durability_damage(damage),
                    })
                })
                .collect();
            let hit = horizon_client::health::HurtSource {
                damage_type: &name,
                causing: 0,
                direct: 0,
                position: None,
            };
            let mut cancel = false;
            self.native_armor_hurt(
                Some(uuid),
                player,
                "minecraft:player",
                &hit,
                &mut entries,
                &mut cancel,
            );
            if cancel {
                return;
            }
            for (i, amount) in amounts.iter_mut().enumerate() {
                *amount = answered_wear(&entries, armor_slot(i), *amount);
            }
        }
        let updates = std::array::from_fn::<_, 4, _>(|i| {
            let stack = eligible[i].take()?;
            // A mod may have put another stack in the slot: it is not worn by
            // the piece that was decided on.
            if amounts[i] == 0 || self.armor[i].as_ref() != Some(&stack) {
                return None;
            }
            let (updated, broken) =
                self.damage_stack(&stack, amounts[i].min(i32::MAX as u32) as i32, true);
            Some((stack, updated, broken))
        });
        for (index, update) in updates.into_iter().enumerate() {
            if let Some((before, updated, broken)) = update
                && updated.as_ref() != Some(&before)
                && self.armor[index].as_ref() == Some(&before)
            {
                self.armor[index] = updated;
                if let Some(stack) = broken {
                    broken_items.push((stack, index));
                    broke = true;
                }
            }
        }
        if broke {
            // LivingEntity.onEquippedItemBroken for each armour piece that broke.
            for (stack, index) in &broken_items {
                self.player_equipment_broke(armor_slot(*index), stack);
                self.award_stat("broken", &stack.item, 1);
            }
            self.apply_equipment();
        }
    }
    /// Voxel traversal that also stops at fluid source blocks (bucket pick).
    fn pick_fluid(&self) -> Option<Hit> {
        let (origin, dir) = self.aim()?;
        let mut t = 0.0;
        let mut last = origin.map(|v| v.floor() as i32);
        let mut first = true;
        while t <= self.reach() {
            let p = [
                origin[0] + dir[0] * t,
                origin[1] + dir[1] * t,
                origin[2] + dir[2] * t,
            ];
            let b = p.map(|v| v.floor() as i32);
            // `BlockGetter.traverseBlocks` visits the block the ray starts in
            // first: the eyes' own water, under water (#3787).
            if first || b != last {
                let state = self.world.block(b[0], b[1], b[2]);
                let f = self.res.facts().get(state);
                let face = [last[0] - b[0], last[1] - b[1], last[2] - b[2]];
                if f.fluid != Fluid::None
                    && f.fluid_level == 8
                    && !f.fluid_falling
                    && !f.waterlogged
                    && f.render_shape != horizon_client::blocks::RenderShape::Model
                {
                    if !first {
                        return Some(Hit {
                            pos: b,
                            face,
                            point: p,
                            inside: false,
                        });
                    }
                    let above = self.world.block(b[0], b[1] + 1, b[2]);
                    let same_above = self.res.facts().get(above).fluid == f.fluid;
                    if let Some((face, point)) =
                        clip_start_source(origin, dir, self.reach(), b, same_above)
                    {
                        return Some(Hit {
                            pos: b,
                            face,
                            point,
                            // A start in the shape hits at the eyes.
                            inside: point == origin,
                        });
                    }
                } else if !first && !self.res.facts().is_air(state) && f.fluid == Fluid::None {
                    // `ClipContext.Block.OUTLINE`: the ray stops at the block's
                    // outline, not its cell; it passes through the empty part
                    // of a torch, a fence or a flower to the water behind.
                    if let Some(hit) =
                        outline_stop(origin, dir, self.reach(), b, face, || self.outline_boxes(b))
                    {
                        return Some(hit);
                    }
                }
                last = b;
                first = false;
            }
            t += 0.05;
        }
        None
    }
    /// `getPlayerPOVHitResult(level, player, SOURCE_ONLY)` for an empty
    /// bucket: the block outline or a source fluid, whichever is nearer. A
    /// waterlogged block is a block hit (its outline is the block's, its
    /// fluid a source); flowing fluid has no shape to pick.
    pub(super) fn bucket_hit(&self) -> Option<Hit> {
        let origin = self.aim().map_or([0.0; 3], |(origin, _)| origin);
        let distance = |p: [f64; 3]| {
            (p[0] - origin[0]).powi(2) + (p[1] - origin[1]).powi(2) + (p[2] - origin[2]).powi(2)
        };
        match (self.pick(), self.pick_fluid()) {
            (Some(block), Some(fluid)) => Some(if distance(block.point) <= distance(fluid.point) {
                block
            } else {
                fluid
            }),
            (block, fluid) => block.or(fluid),
        }
    }
    /// `BucketItem.use`'s place position: a water bucket fills the clicked
    /// block itself when that block can hold it (`canBlockContainFluid`: a
    /// waterloggable stair, slab, fence, ..., but not a double slab, kelp or
    /// seagrass); every other bucket empties into the block the clicked face
    /// leads to. `SolidBucketItem` places like a block item, into a
    /// replaceable clicked block.
    pub(super) fn bucket_target(&self, content: BucketContent, hit: &Hit) -> [i32; 3] {
        let clicked = self.world.block(hit.pos[0], hit.pos[1], hit.pos[2]);
        let behaviours = self.res.behaviours();
        if content == BucketContent::Powder {
            if self.res.facts().get(clicked).replaceable {
                return hit.pos;
            }
        } else if content.is_water()
            && behaviours.is_liquid_container(clicked)
            && behaviours.can_place_liquid(
                clicked,
                horizon_world::fluid::FluidKind::Water,
                self.creative,
            )
        {
            return hit.pos;
        }
        hit.beside()
    }
    /// The cauldron at `pos`, and whether water is above it
    /// (`CauldronInteractions.isUnderWater`).
    pub(super) fn cauldron_at(
        &self,
        pos: [i32; 3],
    ) -> Option<(horizon_world::chunk::BlockState, bool)> {
        let registry = self.pack.assets.registry;
        let id = self.world.block(pos[0], pos[1], pos[2]);
        let state = registry
            .state(id)
            .filter(|s| s.name.ends_with("cauldron"))
            .cloned()?;
        let above = self.world.block(pos[0], pos[1] + 1, pos[2]);
        let under_water = registry
            .state(above)
            .is_some_and(|s| s.name == "minecraft:water");
        Some((state, under_water))
    }
    /// What the cauldron at `pos` answers to `held` on a client, which only
    /// asks (`CauldronInteractions` acts `if (!level.isClientSide())`). A
    /// sneak with something in hand skips `useItemOn`
    /// (`Player.isSecondaryUseActive`).
    pub(super) fn cauldron_outcome(
        &self,
        pos: [i32; 3],
        held: Option<&ItemStack>,
    ) -> horizon_client::cauldron::Outcome {
        use horizon_client::cauldron::{self, Outcome};
        let Some(held) = held else {
            return Outcome::Pass;
        };
        if self.keys.held(Mapping::Sneak) {
            return Outcome::Pass;
        }
        let Some((state, under_water)) = self.cauldron_at(pos) else {
            return Outcome::Pass;
        };
        let removes_dye = cauldron::removes_dye(self.items(), &held.item);
        cauldron::outcome(&state, under_water, removes_dye, held)
    }
    /// `CauldronInteractions` for the held item aimed at a cauldron.
    fn use_on_cauldron(&mut self, pos: [i32; 3]) -> Result<bool> {
        use horizon_client::cauldron::{self, Outcome};
        let registry = self.pack.assets.registry;
        let Some((state, under_water)) = self.cauldron_at(pos) else {
            return Ok(false);
        };
        let Some(held) = self.held() else {
            return Ok(false);
        };
        let removes_dye = cauldron::removes_dye(self.items(), &held.item);
        let slot = self.held_slot();
        let player = self.native_acting_id();
        if self.transactions.is_empty() {
            use transactions::DeferredEffect;
            let mut hand = powder_snow::HeldSlot {
                game: self,
                slot,
                dropper: None,
            };
            let result = cauldron::interact(&state, under_water, removes_dye, &mut hand);
            if result.outcome == Outcome::Pass {
                return Ok(false);
            }
            if let Some(stat) = result.stat {
                self.defer_effect(DeferredEffect::CustomStat {
                    player,
                    key: stat,
                    amount: 1,
                });
            }
            if let Some(block) = result.block
                && let Ok(new) = registry.resolve(&block)
            {
                self.level_set(pos, new)?;
                if result.lower_fill_level {
                    self.post_block_game_event("minecraft:block_change", pos, None, Some(new));
                }
            }
            if let Some(sound) = result.sound {
                self.defer_effect(DeferredEffect::Sound {
                    sound,
                    at: pos.map(|v| f64::from(v) + 0.5),
                    volume: 1.0,
                    pitch: 1.0,
                });
            }
            return Ok(true);
        }
