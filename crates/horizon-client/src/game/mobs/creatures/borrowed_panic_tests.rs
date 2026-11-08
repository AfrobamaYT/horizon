use super::*;
use horizon_client::{
    chunk::Chunk,
    join_seq::residency::{
        Change, Journal, Kind, Owner,
        table::{EntityLookup, EntityTable, SourceEntity},
    },
};
use horizon_core::random::LegacyRandom;
use horizon_mod_runtime::attachments::{AttachmentType, Holder};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    Damage,
    Shield,
    Totem,
    Movement,
    Breathing,
    Goal,
    Knockback,
    Landing,
}
struct Probe {
    family: Family,
    calls: AtomicUsize,
    posts: AtomicUsize,
    armed: AtomicBool,
}
impl Probe {
    fn hear(&self, family: Family, target: bool) {
        if self.family == family && target {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.armed.swap(false, Ordering::SeqCst) {
                eprintln!("BORROWED_NATIVE_REACHED:{family:?}");
                panic!("actual borrowed mob callback panicked");
            }
        }
    }
}
struct NativeGoal {
    probe: Arc<Probe>,
    target: bool,
}
impl mod_api::Goal for NativeGoal {
    fn name(&self) -> &'static str {
        if self.target {
            "BorrowedTarget"
        } else {
            "BorrowedProbe"
        }
    }
    fn flags(&self) -> mod_api::GoalFlags {
        if self.target {
            mod_api::GoalFlags::TARGET
        } else {
            mod_api::GoalFlags::MOVE
        }
    }
    fn can_use(&mut self, _: &mut mod_api::GoalContext<'_>) -> bool {
        true
    }
    fn tick(&mut self, cx: &mut mod_api::GoalContext<'_>) {
        assert_eq!(
            cx.mob().entity().position(cx.host()),
            Some([2.0, 80.0, 0.5])
        );
        if !self.target {
            self.probe.hear(Family::Goal, cx.mob().entity().id() == 72);
        }
    }
}
struct GoalPart(Arc<Probe>);
#[mod_api::horizon_part]
impl mod_api::BlockPart for GoalPart {
    fn fall_on(&self, _: &mut dyn mod_api::LevelRandomHost, event: &mut mod_api::FallOnEvent<'_>) {
        self.0.hear(Family::Landing, event.entity == 72);
    }
}
#[mod_api::horizon_part]
impl mod_api::EntityPart for GoalPart {
    fn register_goals(&self, goals: &mut mod_api::GoalRegistrar<'_>) {
        goals.add_target_goal(
            0,
            NativeGoal {
                probe: self.0.clone(),
                target: true,
            },
        );
        goals.add_goal(
            0,
            NativeGoal {
                probe: self.0.clone(),
                target: false,
            },
        );
    }
}
struct Hooks(Arc<Probe>);
#[mod_api::__subscriptions]
impl mod_api::Hooks for Hooks {
    fn entity_tick_listened(
        &self,
        _: &mut dyn mod_api::ServerHost,
        event: &mut mod_api::EntityTickListenedEvent,
    ) {
        event.listened = true;
    }
    fn entity_tick_post(
        &self,
        _: &mut dyn mod_api::ServerHost,
        _: &mut mod_api::EntityTickPostEvent<'_>,
    ) {
        self.0.posts.fetch_add(1, Ordering::SeqCst);
    }
    fn register(&self, r: &mut dyn mod_api::Registrar, _: &mod_api::RegistrationContext) {
        if self.0.family == Family::Goal {
            r.entity_part("minecraft:zombie", GoalPart(self.0.clone()));
        }
        if self.0.family == Family::Landing {
            let stone = r.block_key("minecraft:stone");
            r.block_part(stone, GoalPart(self.0.clone()));
        }
    }
    fn should_apply_effect_tick_this_tick(
        &self,
        _: &dyn mod_api::Host,
        event: &mut mod_api::ShouldApplyEffectTickThisTickEvent,
    ) {
        if self.0.family == Family::Knockback {
            event.apply = true;
        }
    }
    fn apply_effect_tick(
        &self,
        _: &dyn mod_api::Host,
        event: &mut mod_api::ApplyEffectTickEvent<'_>,
    ) {
        if self.0.family == Family::Knockback && event.entity == Some(72) {
            eprintln!("BORROWED_REAL_EFFECT_HURT");
            event.hurt("minecraft:mob_attack", 1.0);
        }
    }
    fn living_knockback(
        &self,
        _: &mut dyn mod_api::LevelRandomHost,
        event: &mut mod_api::LivingKnockbackEvent<'_>,
    ) -> mod_api::Flow {
        self.0.hear(Family::Knockback, event.entity == 72);
        mod_api::Flow::Continue
    }
    fn living_incoming_damage(
        &self,
        _: &mut dyn mod_api::LevelRandomHost,
        event: &mut mod_api::LivingIncomingDamageEvent<'_>,
    ) -> mod_api::Flow {
        self.0.hear(Family::Damage, event.entity == 72);
        mod_api::Flow::Continue
    }
    fn living_shield_block(
        &self,
        _: &mut dyn mod_api::LevelRandomHost,
        event: &mut mod_api::LivingShieldBlockEvent<'_>,
    ) {
        self.0.hear(Family::Shield, event.entity == 72);
    }
    fn living_speed(
        &self,
        _: &dyn mod_api::LoopContext,
        event: &mut mod_api::LivingSpeedEvent<'_>,
    ) {
        self.0.hear(Family::Movement, event.entity == 72);
    }
    fn living_breathe(
        &self,
        _: &dyn mod_api::LoopContext,
        event: &mut mod_api::LivingBreatheEvent<'_>,
    ) {
        self.0.hear(Family::Breathing, event.entity == 72);
    }
    fn living_use_totem(
        &self,
        _: &mut dyn mod_api::LevelRandomHost,
        event: &mut mod_api::LivingUseTotemEvent<'_>,
    ) -> mod_api::Flow {
        self.0.hear(Family::Totem, event.entity == 72);
        mod_api::Flow::Continue
    }
}
const VALUE: AttachmentType<i32> = AttachmentType {
    namespace: "borrowed-mob-test",
    name: "retained",
    default: || 0,
    codec: None,
    copy_on_death: false,
    sync: false,
};
struct Scratch(std::path::PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Output {
    Quiet,
    Egg,
    Guest,
}
fn run(family: Family) {
    run_output(family, Output::Quiet);
}
fn fixture_game(output: Output, test: impl FnOnce(&mut Game<'_>)) {
    if output == Output::Guest {
        crate::game::native_guest_turn_tests::with_guests(test);
    } else {
        crate::game::native_entity_refs::tests::with_game(test);
    }
}
fn eggs(game: &Game<'_>) -> u32 {
    game.dropped
        .iter()
        .filter(|d| d.stack.item.as_ref() == "minecraft:egg")
        .map(|d| d.stack.count)
        .sum()
}
fn run_output(family: Family, output: Output) {
    crate::entry::install_client_hooks();
    let _ =
        horizon_client::block_contact::HOOKS.set(crate::game::native_hooks::BLOCK_CONTACT_HOOKS);
    static EFFECT: std::sync::Once = std::sync::Once::new();
    EFFECT.call_once(|| {
        horizon_client::brewing::register_mod_effects([(
            "borrowed-panic:hit".into(),
            0,
            horizon_client::brewing::Category::Neutral,
        )]);
    });
    eprintln!("BORROWED_ASSERTIONS_DISABLED:{}", !cfg!(debug_assertions));
    for dim in [Dim::Overworld, Dim::Nether] {
        eprintln!("BORROWED_CASE:{family:?}:{dim:?}");
        let scratch = Scratch(std::env::temp_dir().join(format!(
            "horizon-borrowed-{}-{:?}-{family:?}-{dim:?}",
            std::process::id(),
            std::thread::current().id()
        )));
        let probe = Arc::new(Probe {
            family,
            calls: AtomicUsize::new(0),
            posts: AtomicUsize::new(0),
            armed: AtomicBool::new(true),
        });
        mod_api::testing::with_mods(
            vec![mod_api::ModEntry::new(
                "borrowed-panic",
                Box::new(Hooks(probe.clone())),
            )]
            .into(),
            || {
                crate::game::api_values::install_parts(&[mod_api::ModEntry::new(
                    "borrowed-panic",
                    Box::new(Hooks(probe.clone())),
                )])
                .unwrap();
                fixture_game(output, |game| {
                    game.world_root = scratch.0.clone();
                    game.enter_level(dim).unwrap();
                    game.options.dedicated = Some(Vec::new());
                    if output != Output::Quiet {
                        game.load_server_loot();
                    }
                    if output == Output::Guest {
                        let host = game.lan.as_mut().unwrap();
                        for i in 0..host.hub.guests.len() {
                            host.hub.guests.update(i, |g| {
                                g.dimension = dim.id().to_owned();
                                g.position = if g.conn == 12 {
                                    [1.5, 80.0, 0.5]
                                } else {
                                    [12.0, 80.0, 12.0]
                                };
                                g.loaded = true;
                            });
                        }
                    }
                    game.world
                        .chunks
                        .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
                    assert!(game.entity_ticking_area()(1, 0));
                    let journal = Journal::default();
                    let owner = Owner {
                        dimension: game.dim.key(),
                        kind: Kind::Mob,
                    };
                    game.mobs
                        .bind_residency(&journal, owner, |m| (m.id, m.join));
                    for id in [71, 72] {
                        let kind = game
                            .res
                            .mob_types()
                            .get(if id == 71 {
                                match output {
                                    Output::Egg => "minecraft:chicken",
                                    Output::Guest => "minecraft:zombie",
                                    Output::Quiet => "minecraft:pig",
                                }
                            } else {
                                "minecraft:zombie"
                            })
                            .unwrap()
                            .clone();
                        let mut mob =
                            Mob::new(id, kind, Vec3::new(f64::from(id - 70), 80.0, 0.5), 0.0);
                        mob.no_ai = true;
                        if id == 71 {
                            mob.ai.last_hurt_by_player = 5;
                            if output != Output::Quiet {
                                mob.no_ai = false;
                                mob.on_ground = true;
                            }
                            if output == Output::Egg {
                                mob.extra.egg_time = 1;
                            }
                            if output == Output::Guest {
                                mob.ai.target = Some(horizon_client::mobs::Who::Player);
                            }
                        }
                        if id == 72 {
                            mob.air = 100;
                            if family == Family::Knockback {
                                mob.add_effect(
                                    horizon_client::brewing::effect("borrowed-panic:hit")
                                        .unwrap()
                                        .id,
                                    10,
                                    0,
                                );
                            }
                            if family == Family::Landing {
                                mob.no_ai = false;
                                mob.position.y = 80.1;
                                mob.velocity.y = -1.0;
                                mob.fall_distance = 6.0;
                                mob.on_ground = false;
                            }
                            if matches!(family, Family::Damage | Family::Shield | Family::Totem) {
                                mob.fire_ticks = 20;
                            }
                            if family == Family::Goal {
                                mob.no_ai = false;
                                assert!(mob.ai.goals.has("BorrowedProbe"));
                            }
                            if family == Family::Movement {
                                // NoAI skips aiStep's travel: this must be a
                                // real moving mob, not just a grounded corpse.
                                mob.no_ai = false;
                                mob.on_ground = true;
                                mob.ai.speed = 0.2;
                            }
                            if family == Family::Totem {
                                mob.health = 0.5;
                                mob.extra
                                    .set_main_hand(Some("minecraft:totem_of_undying".into()));
                            }
                        }
                        game.mobs.push(mob);
                    }
                    game.last_hurt_by_player
                        .insert(71, super::super::combat::PlayerCredit(99));
                    let rows: Vec<_> = game.mobs.iter().map(|m| (m.id, m.join)).collect();
                    VALUE
                        .set(&mut game.world.attachments, Holder::Level, 42)
                        .unwrap();
                    game.world.attachment_syncs.insert(Holder::Level);
                    let syncs = game.world.attachment_syncs.clone();
                    game.world.path_writes = Some(Vec::new());
                    let stone = game
                        .pack
                        .assets
                        .registry
                        .default_state("minecraft:stone")
                        .unwrap();
                    game.world.set(4, 80, 4, stone).unwrap();
                    if output != Output::Quiet {
                        for x in 0..5 {
                            for z in 0..5 {
                                game.world.set(x, 79, z, stone).unwrap();
                            }
                        }
                    }
                    if output == Output::Guest {
                        assert_eq!(game.lan_guest_views().len(), 2);
                    }
                    if matches!(family, Family::Movement | Family::Landing) {
                        game.world.set(2, 79, 0, stone).unwrap();
                    }
                    let writes = game.world.path_writes.clone();
                    game.world.random = LegacyRandom::new(0x5eed);
                    let draw = game.world.random.clone().next_long();
                    let guest_health = if output == Output::Guest {
                        game.lan.as_ref().unwrap().hub.guests[0].health
                    } else {
                        0.0
                    };
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        horizon_client::block_contact::in_level(Some(game.dim.id()), || {
                            game.tick_creature_entities()
                        })
                    }));
                    assert_eq!(
                        probe.calls.load(Ordering::SeqCst),
                        1,
                        "real native listener was not reached once"
                    );
                    assert_eq!(
                        probe.posts.load(Ordering::SeqCst),
                        0,
                        "aborted pass fired post callbacks"
                    );
                    let payload = result.expect_err("native callback panic was swallowed");
                    assert_eq!(
                        payload.downcast_ref::<&str>().copied(),
                        Some("actual borrowed mob callback panicked")
                    );
                    if output == Output::Egg {
                        assert!(
                            game.mobs[0].extra.egg_time > 1,
                            "real egg cooldown did not advance"
                        );
                        assert_eq!(eggs(game), 0, "recovery executed egg action");
                        assert!(
                            game.creatures.pending[game.dim.id()]
                                .events
                                .iter()
                                .any(|e| matches!(e, MobEvent::LayEgg { .. })),
                            "real egg action not retained"
                        );
                    }
                    if output == Output::Guest {
                        assert_eq!(
                            game.lan.as_ref().unwrap().hub.guests[0].health,
                            guest_health,
                            "recovery executed guest action"
                        );
                        assert!(
                            game.creatures.pending[game.dim.id()].guests.iter().any(
                                |(conn, mob, e)| *conn == 12
                                    && *mob == 71
                                    && matches!(e, MobEvent::AttackPlayer(..))
                            ),
                            "real guest attack not retained: {:?}",
                            game.creatures.pending[game.dim.id()].guests
                        );
                    }
                    let mut pending = Vec::new();
                    journal.drain_into(&mut pending);
                    let (mut depth, mut lifts) = (0usize, 0usize);
                    for e in pending {
                        assert_eq!(e.owner, owner);
                        match e.change {
                            Change::Lifted => {
                                depth += 1;
                                lifts += 1;
                            }
                            Change::Restored => depth = depth.checked_sub(1).unwrap(),
                            _ => {}
                        }
                        journal.push(e);
                    }
                    assert!(lifts > 0);
                    assert_eq!(
                        depth, 0,
                        "borrowed native callback abandoned actual mob pass"
                    );
                    assert_eq!(
                        game.mobs.iter().map(|m| (m.id, m.join)).collect::<Vec<_>>(),
                        rows
                    );
                    assert!(
                        game.creatures.ticked.contains(&71),
                        "earlier actual mob progress was lost"
                    );
                    assert_eq!(
                        VALUE
                            .get_existing(&mut game.world.attachments, Holder::Level)
                            .unwrap(),
                        Some(42)
                    );
                    assert_eq!(game.last_hurt_by_player.get(&71).map(|c| c.0), Some(99));
                    assert_eq!(game.world.attachment_syncs, syncs);
                    assert_eq!(game.world.path_writes, writes);
                    assert_eq!(game.world.random.clone().next_long(), draw);
                    let mut table = EntityTable::new(journal);
                    for &(id, join) in &rows {
                        let found = table.entity_lookup(&owner.dimension, id, |at| {
                            let m = &game.mobs[game.mobs.locate(at.join)?];
                            Some(SourceEntity {
                                id: m.id,
                                value: m.join,
                            })
                        });
                        assert!(
                            matches!(found, EntityLookup::Found { location, value } if location.owner == owner && value == join)
                        );
                    }
                    table.end_tick().unwrap();
                    if family == Family::Goal {
                        assert!(
                            game.mobs[1].ai.targets.is_running("BorrowedTarget"),
                            "actual native target selector was lost"
                        );
                        assert!(
                            game.mobs[1].ai.goals.is_running("BorrowedProbe"),
                            "actual native goal selector was lost"
                        );
                    }
                    game.tick_creature_entities().unwrap();
                    assert!(
                        probe.posts.load(Ordering::SeqCst) > 0,
                        "real post listener never ran on retry"
                    );
                    table.end_tick().unwrap();
                    if output == Output::Egg {
                        assert_eq!(eggs(game), 1, "retained egg not delivered exactly once");
                        game.tick_creature_entities().unwrap();
                        assert_eq!(eggs(game), 1, "egg action replayed");
                    }
                    if output == Output::Guest {
                        let health = game.lan.as_ref().unwrap().hub.guests[0].health;
                        assert!(health < guest_health, "retained guest attack not delivered");
                        assert_eq!(
                            game.lan.as_ref().unwrap().hub.guests[1].health,
                            guest_health,
                            "attack routed to wrong guest"
                        );
                        game.mobs[0].no_ai = true;
                        game.as_guest(12, |g| {
                            g.player.as_mut().unwrap().health.invulnerable_time = 0
                        })
                        .unwrap();
                        game.tick_creature_entities().unwrap();
                        assert_eq!(
                            game.lan.as_ref().unwrap().hub.guests[0].health,
                            health,
                            "guest action replayed"
                        );
                    }
                })
            },
        );
    }
}
macro_rules! cases {
    ($($name:ident => $family:ident),* $(,)?) => {$(
        #[test]
        #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
        fn $name() { run(Family::$family); }
    )*};
}
cases!(damage => Damage, shield => Shield, totem => Totem, movement => Movement, breathing => Breathing, goal => Goal, knockback => Knockback, landing => Landing);

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn retained_egg() {
    run_output(Family::Damage, Output::Egg);
}
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn retained_guest() {
    run_output(Family::Damage, Output::Guest);
}

struct ConsumerPanic(Arc<AtomicUsize>);
#[mod_api::__subscriptions]
impl mod_api::Hooks for ConsumerPanic {
    fn item_spawn(&self, _: &mut dyn mod_api::ServerHost, _: &mut mod_api::ItemSpawnEvent<'_>) {
        let entered = self.0.fetch_add(1, Ordering::SeqCst);
        eprintln!("RETAINED_ITEM_CONSUMER:{entered}");
        if entered == 0 {
            panic!("real item consumer");
        }
    }
}
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn retained_consumer_tail() {
    crate::entry::install_client_hooks();
    for dim in [Dim::Overworld, Dim::Nether] {
        let calls = Arc::new(AtomicUsize::new(0));
        let scratch = Scratch(
            std::env::temp_dir().join(format!("horizon-consumer-{}-{dim:?}", std::process::id())),
        );
        mod_api::testing::with_mods(
            vec![mod_api::ModEntry::new(
                "consumer",
                Box::new(ConsumerPanic(calls.clone())),
            )]
            .into(),
            || {
                crate::game::native_entity_refs::tests::with_game(|game| {
                    game.world_root = scratch.0.clone();
                    game.enter_level(dim).unwrap();
                    game.options.dedicated = Some(Vec::new());
                    game.load_server_loot();
                    game.world
                        .chunks
                        .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
                    for id in [71, 72] {
                        let kind = game
                            .res
                            .mob_types()
                            .get("minecraft:chicken")
                            .unwrap()
                            .clone();
                        let mut mob =
                            Mob::new(id, kind, Vec3::new(f64::from(id - 70), 80.0, 0.5), 0.0);
                        mob.extra.egg_time = 1;
                        game.mobs.push(mob);
                    }
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        game.tick_creature_entities()
                    }));
                    assert_eq!(
                        result.unwrap_err().downcast_ref::<&str>(),
                        Some(&"real item consumer")
                    );
                    assert_eq!(calls.load(Ordering::SeqCst), 1);
                    assert_eq!(
                        eggs(game),
                        1,
                        "entered action did not produce its observable item"
                    );
                    assert!(game.mobs.iter().all(|m| m.extra.egg_time > 1));
                    game.tick_creature_entities().unwrap();
                    assert_eq!(
                        eggs(game),
                        2,
                        "unentered tail lost or current action replayed"
                    );
                    assert_eq!(calls.load(Ordering::SeqCst), 2);
                    game.tick_creature_entities().unwrap();
                    assert_eq!(eggs(game), 2);
                    assert_eq!(calls.load(Ordering::SeqCst), 2);
                    eprintln!("RETAINED_CONSUMER_RETRY:{dim:?}");
                });
            },
        );
    }
}
