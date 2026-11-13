//! Levels a mod opens at run time: the table's rules, and a mod's calls against a game.

use super::native_entity_refs::tests::with_game;
use super::runtime_levels::RuntimeTable;
use mod_api::{
    Hooks, HostSide, LevelLoadEvent, LevelRef, LevelTickPostEvent, LevelUnloadEvent, ModEntry,
    RuntimeGenerator, RuntimeLevel, RuntimeLevelError,
};
use std::sync::{Arc, Mutex};

fn flat() -> RuntimeLevel {
    RuntimeLevel::new("minecraft:overworld", RuntimeGenerator::Flat, 1)
}

#[test]
fn the_cap_refuses_the_open_past_it_and_a_removal_frees_a_place() {
    let mut table = RuntimeTable::default();
    table.set_cap(2);
    assert!(table.queue_open("cap:a", flat()).is_ok());
    assert!(table.queue_open("cap:b", flat()).is_ok());
    assert_eq!(
        table.queue_open("cap:c", flat()),
        Err(RuntimeLevelError::CapReached)
    );
    table.remove("cap:a");
    assert!(table.queue_open("cap:c", flat()).is_ok());
}

#[test]
fn the_default_cap_is_64() {
    let mut table = RuntimeTable::default();
    for n in 0..64 {
        assert!(table.queue_open(&format!("cap:l{n}"), flat()).is_ok());
    }
    assert_eq!(
        table.queue_open("cap:l64", flat()),
        Err(RuntimeLevelError::CapReached)
    );
}

#[test]
fn an_id_is_open_once_and_each_open_has_its_own_generation() {
    let mut table = RuntimeTable::default();
    let first = table.queue_open("gen:a", flat()).unwrap();
    assert_eq!(
        table.queue_open("gen:a", flat()),
        Err(RuntimeLevelError::AlreadyOpen)
    );
    assert_eq!(table.generation_of("gen:a"), first);
    table.remove("gen:a");
    let second = table.queue_open("gen:a", flat()).unwrap();
    assert_ne!(first, second);
    assert_eq!(table.generation_of("gen:a"), second);
    assert_eq!(table.generation_of("minecraft:the_nether"), 0);
}

#[test]
fn a_level_is_open_after_its_request_is_applied_and_until_its_close_is() {
    let mut table = RuntimeTable::default();
    table.queue_open("state:a", flat()).unwrap();
    assert!(!table.is_open("state:a"));
    assert_eq!(table.queue_unload("state:a"), Err(RuntimeLevelError::NotOpen));
    table.mark_open("state:a");
    assert!(table.is_open("state:a"));
    assert_eq!(table.queue_unload("state:a"), Ok(()));
    assert!(table.is_open("state:a"), "still open until the close is applied");
    assert_eq!(table.queue_unload("state:a"), Err(RuntimeLevelError::NotOpen));
    assert_eq!(table.queue_unload("state:none"), Err(RuntimeLevelError::NotOpen));
}

type Seen<T> = Arc<Mutex<Vec<T>>>;

/// A mod that opens `id` in `level_tick_post` and says what each call answered.
struct Opener {
    id: &'static str,
    level: RuntimeLevel,
    answers: Seen<Result<LevelRef, RuntimeLevelError>>,
    open_right_after: Seen<bool>,
    loads: Seen<String>,
    unloads: Seen<String>,
}

#[mod_api::__subscriptions]
impl Hooks for Opener {
    fn level_tick_post(&self, host: HostSide, _: &mut LevelTickPostEvent) {
        let HostSide::Server(host) = host else {
            return;
        };
        let answer = host.runtime_levels().open(self.id, self.level.clone());
        self.answers.lock().unwrap().push(answer);
        let open = host.runtime_levels().is_open(self.id);
        self.open_right_after.lock().unwrap().push(open);
    }
    fn level_load(&self, _: &mut dyn mod_api::ServerHost, event: &mut LevelLoadEvent<'_>) {
        self.loads.lock().unwrap().push(event.dimension.to_owned());
    }
    fn level_unload(&self, _: &mut dyn mod_api::ServerHost, event: &mut LevelUnloadEvent<'_>) {
        self.unloads.lock().unwrap().push(event.dimension.to_owned());
    }
}

struct Fixture {
    answers: Seen<Result<LevelRef, RuntimeLevelError>>,
    open_right_after: Seen<bool>,
    loads: Seen<String>,
    unloads: Seen<String>,
}

/// A mod named `name` that opens `id` each time a level ticks, in a game, for `test`.
fn with_opener(
    name: &'static str,
    id: &'static str,
    level: RuntimeLevel,
    test: impl FnOnce(&mut super::Game<'_>, &Fixture),
) {
    let fixture = Fixture {
        answers: Seen::default(),
        open_right_after: Seen::default(),
        loads: Seen::default(),
        unloads: Seen::default(),
    };
    let opener = Opener {
        id,
        level,
        answers: fixture.answers.clone(),
        open_right_after: fixture.open_right_after.clone(),
        loads: fixture.loads.clone(),
        unloads: fixture.unloads.clone(),
    };
    let mods = vec![ModEntry::new(name, Box::new(opener))].into();
    mod_platform::testing::with_mods(mods, || {
        with_game(|game| {
            let _phase = mod_api::__private::PhaseOwner::scope(mod_api::__private::ModPhase::World);
            test(game, &fixture);
        });
    });
}

/// `RuntimeLevels::unload` as the mod `name` calls it.
fn unload_as(
    game: &mut super::Game<'_>,
    name: &'static str,
    id: &str,
) -> Result<(), RuntimeLevelError> {
    let _call = mod_api::content::crash::enter_native_mod(mod_api::content::crash::ModCall {
        id: name,
        hook: "level_tick_post",
        part: None,
    });
    game.runtime_level_unload(id)
}

fn tick_post(game: &mut super::Game<'_>) {
    let home = game.level("minecraft:overworld").unwrap();
    game.native_level_tick_post(home);
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_mod_opens_only_ids_in_its_own_namespace() {
    for (id, expected) in [
        ("other:x", Err(RuntimeLevelError::OutsideNamespace)),
        ("mymod:../x", Err(RuntimeLevelError::InvalidId)),
        ("mymod:a//b", Err(RuntimeLevelError::InvalidId)),
        ("minecraft:overworld", Err(RuntimeLevelError::OutsideNamespace)),
        ("mymod:instance/ab", Ok(())),
    ] {
        with_opener("mymod", id, flat(), |game, seen| {
            tick_post(game);
            let answer = seen.answers.lock().unwrap()[0].map(|_| ());
            assert_eq!(answer, expected, "{id}");
        });
    }
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn nothing_opens_inside_a_tick_and_the_queue_is_applied_after_the_levels() {
    with_opener("phase", "phase:a", flat(), |game, seen| {
        tick_post(game);
        assert!(seen.answers.lock().unwrap()[0].is_ok());
        assert_eq!(*seen.open_right_after.lock().unwrap(), [false]);
        assert!(!game.runtime_level_is_open("phase:a"));
        assert!(seen.loads.lock().unwrap().is_empty());
        game.tick_server(false).unwrap();
        assert!(game.runtime_level_is_open("phase:a"));
        assert_eq!(*seen.loads.lock().unwrap(), ["phase:a"]);
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_handle_from_before_an_unload_never_names_the_reopened_level() {
    with_opener("stale", "stale:a", flat(), |game, seen| {
        tick_post(game);
        game.apply_runtime_levels();
        let first = seen.answers.lock().unwrap()[0].unwrap();
        assert!(game.level_handle_current(first));
        assert!(game.native_level_world(first).is_some());
        unload_as(game, "stale", "stale:a").unwrap();
        game.apply_runtime_levels();
        assert!(!game.runtime_level_is_open("stale:a"));
        assert!(game.native_level_world(first).is_none());
        // The same id opens again: a new open, a new handle.
        seen.answers.lock().unwrap().clear();
        tick_post(game);
        game.apply_runtime_levels();
        let second = seen.answers.lock().unwrap()[0].unwrap();
        assert_ne!(first, second);
        assert!(game.native_level_world(second).is_some());
        assert!(!game.level_handle_current(first));
        assert!(game.native_level_world(first).is_none());
        assert_eq!(*seen.loads.lock().unwrap(), ["stale:a", "stale:a"]);
        assert_eq!(*seen.unloads.lock().unwrap(), ["stale:a"]);
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_level_with_a_player_in_it_is_not_unloaded() {
    with_opener("crowd", "crowd:a", flat(), |game, seen| {
        tick_post(game);
        game.apply_runtime_levels();
        let dim = super::Dim::extra("crowd:a").unwrap();
        game.enter_level(dim).unwrap();
        // The player stands in the level in place.
        game.player = Some(
            horizon_client::motion::PlayerMotion::new(
                game.motion,
                horizon_core::geometry::Vec3::new(0.5, 80.0, 0.5),
            )
            .unwrap(),
        );
        assert_eq!(
            unload_as(game, "crowd", "crowd:a"),
            Err(RuntimeLevelError::PlayersInside)
        );
        game.enter_level(super::Dim::Overworld).unwrap();
        assert_eq!(unload_as(game, "crowd", "crowd:a"), Ok(()));
        game.apply_runtime_levels();
        assert!(!game.runtime_level_is_open("crowd:a"));
        assert_eq!(*seen.unloads.lock().unwrap(), ["crowd:a"]);
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_reopened_level_comes_back_from_its_folder_with_its_block() {
    with_opener("keep", "keep:a", flat(), |game, seen| {
        // The test game saves under its working directory: start and end without a folder.
        let folder = game.world_root.join("dimensions/keep");
        let _ = std::fs::remove_dir_all(&folder);
        let stone = game.pack.assets.registry.default_state("minecraft:stone").unwrap();
        tick_post(game);
        game.apply_runtime_levels();
        let dim = super::Dim::extra("keep:a").unwrap();
        game.enter_level(dim).unwrap();
        game.streaming.forced.insert((0, 0));
        for _ in 0..2000 {
            game.update_world();
            if game.world.chunks.contains_key(&(0, 0)) {
                break;
            }
            game.streaming.wait(std::time::Duration::from_millis(5));
        }
        assert!(game.world.chunks.contains_key(&(0, 0)), "the chunk generated");
        game.world.set(1, -50, 1, stone).unwrap();
        game.enter_level(super::Dim::Overworld).unwrap();
        unload_as(game, "keep", "keep:a").unwrap();
        game.apply_runtime_levels();
        assert!(game.world_root.join("dimensions/keep/a").is_dir(), "the folder stays");

        seen.answers.lock().unwrap().clear();
        tick_post(game);
        game.apply_runtime_levels();
        let dim = super::Dim::extra("keep:a").unwrap();
        game.enter_level(dim).unwrap();
        game.streaming.forced.insert((0, 0));
        for _ in 0..2000 {
            game.update_world();
            if game.world.chunks.contains_key(&(0, 0)) {
                break;
            }
            game.streaming.wait(std::time::Duration::from_millis(5));
        }
        assert_eq!(game.world.block(1, -50, 1), stone);
        game.enter_level(super::Dim::Overworld).unwrap();
        unload_as(game, "keep", "keep:a").unwrap();
        game.apply_runtime_levels();
        let _ = std::fs::remove_dir_all(&folder);
    });
}
