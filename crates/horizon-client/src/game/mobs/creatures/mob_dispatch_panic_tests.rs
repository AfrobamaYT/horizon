use super::*;
use horizon_client::{
    chunk::Chunk,
    join_seq::residency::{Change, Journal, Kind, Owner},
};
use horizon_core::random::{LegacyRandom, RandomSource};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

struct PanicOnDespawn(Arc<AtomicBool>);
#[mod_api::__subscriptions]
impl mod_api::Hooks for PanicOnDespawn {
    fn mob_despawn(&self, _: &dyn mod_api::Host, event: &mut mod_api::MobDespawnEvent<'_>) {
        if !self.0.swap(true, Ordering::SeqCst) {
            // An unwound result must not become the next attempt's answer.
            *event.result = mod_api::NativeDespawn::Allow;
            panic!("actual mob despawn listener panicked");
        }
    }
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn mob_despawn_listener_panic_preserves_owned_rows_and_lent_random() {
    eprintln!("MOB_PANIC_ASSERTIONS_DISABLED:{}", !cfg!(debug_assertions));
    for dimension in [Dim::Overworld, Dim::Nether] {
        let reached = Arc::new(AtomicBool::new(false));
        let mods = vec![mod_api::ModEntry::new(
            "panic-on-mob-despawn",
            Box::new(PanicOnDespawn(reached.clone())),
        )]
        .into();
        let root = std::env::temp_dir().join(format!(
            "horizon-mob-panic-{}-{:?}-{dimension:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        mod_api::testing::with_mods(mods, || {
            crate::game::native_entity_refs::tests::with_game(|game| {
                game.world_root = root.clone();
                game.enter_level(dimension).unwrap();
                game.options.dedicated = Some(Vec::new());
                game.world
                    .chunks
                    .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
                let journal = Journal::default();
                let owner = Owner {
                    dimension: game.dim.key(),
                    kind: Kind::Mob,
                };
                game.mobs
                    .bind_residency(&journal, owner, |mob| (mob.id, mob.join));
                let kind = game.res.mob_types().get("minecraft:pig").unwrap().clone();
                for id in [71, 72, 73] {
                    let mut mob = Mob::new(
                        id,
                        kind.clone(),
                        Vec3::new(f64::from(id - 70), 80.0, 0.5),
                        0.0,
                    );
                    mob.no_ai = true;
                    game.mobs.push(mob);
                }
                let expected: Vec<_> = game.mobs.iter().map(|mob| (mob.id, mob.join)).collect();
                game.world.random = LegacyRandom::new(0x5eed);
                let expected_draw = game.world.random.clone().next_long();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    game.tick_creature_entities()
                }));
                assert!(
                    reached.load(Ordering::SeqCst),
                    "actual listener never reached"
                );
                assert!(result.is_err(), "listener panic did not unwind");
                let mut events = Vec::new();
                journal.drain_into(&mut events);
                let mut depth = 0usize;
                for event in events {
                    assert_eq!(event.owner, owner);
                    match event.change {
                        Change::Lifted => depth += 1,
                        Change::Restored => {
                            depth = depth.checked_sub(1).expect("unmatched mob restore")
                        }
                        _ => {}
                    }
                }
                assert_eq!(depth, 0, "mob listener abandoned the real lifted pass");
                assert_eq!(
                    game.mobs
                        .iter()
                        .map(|mob| (mob.id, mob.join))
                        .collect::<Vec<_>>(),
                    expected,
                    "mob listener lost or reordered original rows"
                );
                assert_eq!(
                    game.world.random.clone().next_long(),
                    expected_draw,
                    "mob listener lost the lent world random"
                );
                game.tick_creature_entities().unwrap();
                assert_eq!(
                    game.mobs
                        .iter()
                        .map(|mob| (mob.id, mob.join))
                        .collect::<Vec<_>>(),
                    expected,
                    "failed listener result leaked into the next despawn check"
                );
            });
        });
        let _ = std::fs::remove_dir_all(root);
    }
}
