use super::*;
use horizon_client::{chunk::Chunk, motion::PlayerMotion};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

struct MovementPanic {
    id: u32,
    remaining: AtomicUsize,
    calls: AtomicUsize,
}
struct Hooks(Arc<MovementPanic>);
#[mod_api::__subscriptions]
impl mod_api::Hooks for Hooks {
    fn living_speed(
        &self,
        _: &dyn mod_api::LoopContext,
        event: &mut mod_api::LivingSpeedEvent<'_>,
    ) {
        if event.entity == self.0.id
            && self
                .0
                .remaining
                .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
                .is_ok()
        {
            self.0.calls.fetch_add(1, Ordering::SeqCst);
            std::panic::panic_any(89_u32);
        }
    }
}
fn prepare(game: &mut Game<'_>) {
    game.world_root = PathBuf::from(std::env::var_os("HORIZON_HOME").unwrap())
        .join(format!("retained-{}", std::process::id()));
    game.options.dedicated = Some(Vec::new());
    game.load_server_loot();
    game.world.health_rules.difficulty = horizon_world::food::Difficulty::Normal;
    game.world.game_time = 1000;
    game.world
        .chunks
        .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
    let stone = game
        .pack
        .assets
        .registry
        .default_state("minecraft:stone")
        .unwrap();
    for x in 0..16 {
        for z in 0..16 {
            game.world.set(x, 79, z, stone).unwrap();
        }
    }
}

#[test]
#[ignore = "needs prepared resources and isolated homes"]
fn same_mob_guest_attack_survives_movement_panic() {
    crate::entry::install_client_hooks();
    let probe = Arc::new(MovementPanic {
        id: 71,
        remaining: AtomicUsize::new(1),
        calls: AtomicUsize::new(0),
    });
    mod_api::testing::with_mods(
        vec![mod_api::ModEntry::new(
            "same-mob",
            Box::new(Hooks(probe.clone())),
        )]
        .into(),
        || {
            crate::game::native_guest_turn_tests::with_guests(|game| {
                prepare(game);
                game.player =
                    Some(PlayerMotion::new(game.motion, Vec3::new(12.0, 80.0, 12.0)).unwrap());
                let host = game.lan.as_mut().unwrap();
                for i in 0..host.hub.guests.len() {
                    host.hub.guests.update(i, |guest| {
                        guest.loaded = true;
                        guest.load_timeout = 0;
                        guest.position = if guest.conn == 12 {
                            [1.5, 80.0, 0.5]
                        } else {
                            [12.0, 80.0, 12.0]
                        };
                    });
                }
                let kind = game
                    .res
                    .mob_types()
                    .get("minecraft:zombie")
                    .unwrap()
                    .clone();
                let mut mob = Mob::new(71, kind, Vec3::new(1.0, 80.0, 0.5), 0.0);
                mob.on_ground = true;
                mob.ai.speed = 0.2;
                mob.ai.target = Some(horizon_client::mobs::Who::Player);
                game.mobs.push(mob);
                let host_before = game.player.as_ref().unwrap().health.health();
                let guest_before = game.lan.as_ref().unwrap().hub.guests[0].health;
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    game.tick_creature_entities()
                }));
                assert_eq!(result.unwrap_err().downcast_ref::<u32>(), Some(&89));
                assert_eq!(probe.calls.load(Ordering::SeqCst), 1);
                let pending = &game.creatures.pending[game.dim.id()];
                assert!(
                    pending.batches.iter().any(|batch| {
                        batch.normal.root.events.iter().any(|e| {
                            matches!(
                                e,
                                MobEvent::AttackPlayer(..)
                                    | MobEvent::AttackPlayerWithEffect { .. }
                            )
                        }) || !batch.guests.is_empty()
                    }),
                    "same mob must produce the real AI attack before movement panics"
                );
                game.mobs[0].no_ai = true;
                game.tick_creature_entities().unwrap();
                assert!(
                    game.lan.as_ref().unwrap().hub.guests[0].health < guest_before,
                    "same-mob retained attack must damage the guest"
                );
                assert_eq!(
                    game.player.as_ref().unwrap().health.health(),
                    host_before,
                    "guest attack must never damage the host"
                );
                let after = game.lan.as_ref().unwrap().hub.guests[0].health;
                game.as_guest(12, |game| {
                    game.player.as_mut().unwrap().health.invulnerable_time = 0
                })
                .unwrap();
                game.tick_creature_entities().unwrap();
                assert_eq!(
                    game.lan.as_ref().unwrap().hub.guests[0].health,
                    after,
                    "entered guest attack must not replay"
                );
            });
        },
    );
}
