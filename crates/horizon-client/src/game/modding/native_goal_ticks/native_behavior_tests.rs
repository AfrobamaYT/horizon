//! Tests for the mod behavior runner (arismoko/horizon#824): the vanilla
//! `Behavior` contract over a real game — the start checks, the level-random
//! duration draw, the timeout, `can_still_use` and its one-tick default,
//! panic containment, and the typed `NoBrain` refusal — driven through the
//! same goal-host view the goal tests use.
use super::super::creatures;
use super::super::native_entity_refs::tests::with_game;
use super::tests::zombie;
use horizon_client::health::DamageLevel;
use horizon_client::mobs::{self, Mob, MobWorld};
use mod_api::{
    Activity, Behavior, BehaviorCx, BehaviorRegistrar, BrainMemory, EntityPart, Hooks,
    MemoryStatus, ModEntry, RegisteredBehavior, Registrar, RegistrationContext,
};
use std::sync::Mutex;

/// The mods of these tests, installed with their parts, as a game's open does.
fn install_game_mods() {
    mod_api::__private::install(vec![ModEntry::new("test", Box::new(Mod))].into());
    crate::game::api_values::install_parts(&[ModEntry::new("test", Box::new(Mod))]).unwrap();
}

/// What one test behavior did, in order.
static CALLS: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
/// The refusals a `register_behaviors` saw.
static REFUSALS: Mutex<Vec<mod_api::BehaviorRefusal>> = Mutex::new(Vec::new());

fn record(call: &'static str) {
    CALLS.lock().unwrap().push(call);
}

/// The behavior every runner test drives: journals its calls and answers
/// `can_still_use` for `still_for` ticks.
struct Probe {
    still_for: i32,
    ticks: i32,
    panic_in_start: bool,
}
impl Probe {
    fn still_for(still_for: i32) -> Self {
        Self {
            still_for,
            ticks: 0,
            panic_in_start: false,
        }
    }
}
impl Behavior for Probe {
    fn name(&self) -> &'static str {
        "Probe"
    }
    fn check_extra_start_conditions(&mut self, _cx: &mut BehaviorCx<'_>) -> bool {
        record("check");
        true
    }
    fn start(&mut self, cx: &mut BehaviorCx<'_>) {
        record("start");
        // The mob's own handle answers through the tick's view.
        assert_eq!(cx.mob().entity().id(), 8);
        if self.panic_in_start {
            panic!("a behavior's bug");
        }
    }
    fn can_still_use(&mut self, _cx: &mut BehaviorCx<'_>) -> bool {
        self.ticks += 1;
        self.ticks <= self.still_for
    }
    fn tick(&mut self, _cx: &mut BehaviorCx<'_>) {
        record("tick");
    }
    fn stop(&mut self, _cx: &mut BehaviorCx<'_>) {
        record("stop");
    }
}

/// One wrapped behavior, as a brain's list holds it.
fn wrapped(min: i32, max: i32, behavior: Box<dyn Behavior>) -> mobs::mod_behaviors::ModBehavior {
    mobs::mod_behaviors::ModBehavior::new(RegisteredBehavior {
        mod_id: "test",
        part: "ZombieBehaviors",
        activity: Activity::Idle,
        priority: 5,
        min,
        max,
        entry: &[],
        behavior,
    })
}

/// The mob's brain-tick shape, as the goal tests lend it (#1077): the mob
/// lifted out of the game, the world with the goal host beside it, and a
/// level random the test owns, all handed to `drive` together.
fn with_mob_world(
    game: &mut super::super::Game<'_>,
    drive: impl FnOnce(&mut Mob, &MobWorld<'_>, &mut DamageLevel<'_>),
) {
    game.options.dedicated = Some(Vec::new());
    let mut mobs = game.mobs.lift();
    let snapshot: Vec<_> = mobs
        .iter()
        .map(|m| creatures::seen(m, game.level.player_uuid))
        .collect();
    let mut w = game.mob_world_base(&snapshot);
    w.goal_host = Some(&*game);
    let mut test_level = horizon_client::health::TestLevel::new(0);
    let mut level = test_level.level();
    drive(&mut mobs[0], &w, &mut level);
    game.mobs = mobs;
}

/// The level random's state, for the draw assertions.
fn random_of(level: &mut DamageLevel<'_>) -> (u64, Option<f64>) {
    (&*level.random()).into()
}

/// The mods of the `NoBrain` test: a part on a zombie whose one behavior add
/// records its refusal.
struct ZombieBehaviors;
#[mod_api::horizon_part]
impl EntityPart for ZombieBehaviors {
    fn register_behaviors(&self, behaviors: &mut BehaviorRegistrar<'_>) {
        if let Err(e) = behaviors.add(Activity::Idle, 5, 40, 80, &[], Probe::still_for(1)) {
            REFUSALS.lock().unwrap().push(e);
        }
    }
}
struct Mod;
#[mod_api::__subscriptions]
impl Hooks for Mod {
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        r.entity_part("minecraft:zombie", ZombieBehaviors);
    }
}

#[test]
#[ignore = "needs prepared resources"]
fn a_start_that_passes_its_checks_draws_once_and_runs() {
    CALLS.lock().unwrap().clear();
    with_game(|game| {
        zombie(game, 8, [3.5, 65.0, 3.5]);
        let mut probe = wrapped(3, 3, Box::new(Probe::still_for(10)));
        with_mob_world(game, |mob, w, level| {
            let before = random_of(level);
            assert!(probe.try_start(mob, w, level.random(), 0));
            assert_ne!(random_of(level), before, "the duration is one draw");
        });
        assert!(probe.running());
        assert_eq!(*CALLS.lock().unwrap(), ["check", "start"]);
    });
}

#[test]
#[ignore = "needs prepared resources"]
fn a_failed_entry_condition_draws_nothing_and_calls_nothing() {
    CALLS.lock().unwrap().clear();
    with_game(|game| {
        zombie(game, 8, [3.5, 65.0, 3.5]);
        // No species' brain keeps a wired memory yet, so `Present` fails.
        let mut probe = mobs::mod_behaviors::ModBehavior::new(RegisteredBehavior {
            mod_id: "test",
            part: "ZombieBehaviors",
            activity: Activity::Idle,
            priority: 5,
            min: 3,
            max: 3,
            entry: &[(BrainMemory::WalkTarget, MemoryStatus::Present)],
            behavior: Box::new(Probe::still_for(10)),
        });
        with_mob_world(game, |mob, w, level| {
            let before = random_of(level);
            assert!(!probe.try_start(mob, w, level.random(), 0));
            assert_eq!(random_of(level), before, "a failed start draws nothing");
        });
        assert!(!probe.running());
        assert!(CALLS.lock().unwrap().is_empty());
    });
}

#[test]
#[ignore = "needs prepared resources"]
fn the_duration_ends_it_and_can_still_use_keeps_it() {
    CALLS.lock().unwrap().clear();
    with_game(|game| {
        zombie(game, 8, [3.5, 65.0, 3.5]);
        // A fixed window of 3: `end = 0 + 3`, stopped at the first tick
        // past it.
        let mut probe = wrapped(3, 3, Box::new(Probe::still_for(10)));
        with_mob_world(game, |mob, w, level| {
            assert!(probe.try_start(mob, w, level.random(), 0));
            probe.tick_or_stop(mob, w, 1);
            probe.tick_or_stop(mob, w, 2);
            probe.tick_or_stop(mob, w, 3);
            assert!(probe.running(), "timestamp > end is the stop");
            probe.tick_or_stop(mob, w, 4);
        });
        assert!(!probe.running());
        assert_eq!(
            *CALLS.lock().unwrap(),
            ["check", "start", "tick", "tick", "tick", "stop"]
        );
    });
}

#[test]
#[ignore = "needs prepared resources"]
fn without_can_still_use_it_lives_one_brain_tick() {
    CALLS.lock().unwrap().clear();
    with_game(|game| {
        zombie(game, 8, [3.5, 65.0, 3.5]);
        // `still_for(0)`: `can_still_use` answers false the first time, as
        // a behavior that does not override it does.
        let mut probe = wrapped(60, 60, Box::new(Probe::still_for(0)));
        with_mob_world(game, |mob, w, level| {
            assert!(probe.try_start(mob, w, level.random(), 0));
            probe.tick_or_stop(mob, w, 0);
        });
        assert!(!probe.running());
        assert_eq!(*CALLS.lock().unwrap(), ["check", "start", "stop"]);
    });
}

#[test]
#[ignore = "needs prepared resources"]
fn a_panic_is_contained_and_the_behavior_is_done() {
    CALLS.lock().unwrap().clear();
    with_game(|game| {
        zombie(game, 8, [3.5, 65.0, 3.5]);
        let mut panicking = Probe::still_for(10);
        panicking.panic_in_start = true;
        let mut probe = wrapped(3, 3, Box::new(panicking));
        with_mob_world(game, |mob, w, level| {
            // The start itself panics; the runner contains it, so the call
            // returns (as a failed start) instead of unwinding.
            assert!(!probe.try_start(mob, w, level.random(), 0));
            assert!(!probe.running());
            // The behavior is gone: no further call reaches it.
            assert!(!probe.try_start(mob, w, level.random(), 1));
        });
        assert_eq!(*CALLS.lock().unwrap(), ["check", "start"]);
    });
}

#[test]
#[ignore = "needs prepared resources"]
fn a_type_with_no_brain_is_refused_at_construction() {
    REFUSALS.lock().unwrap().clear();
    with_game(|game| {
        install_game_mods();
        game.resolve_parts().unwrap();
        zombie(game, 8, [3.5, 65.0, 3.5]);
        assert_eq!(
            *REFUSALS.lock().unwrap(),
            [mod_api::BehaviorRefusal::NoBrain],
            "the part learned at construction that a zombie has no brain"
        );
    });
}
