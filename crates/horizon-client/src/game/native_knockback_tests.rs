//! `living_knockback` heard by a trait mod through the game's own bridge: who the hook names
//! as the attacker, what the attacker holds, what a mod may cancel, and thorns' wearer.
use super::entity::damage_level_of;
use super::native_entity_refs::tests::with_game;
use horizon_client::health::{
    Hurting, Knock, knockback_attacker, knockback_from, knockback_from_holding,
};
use horizon_client::items::ItemStack;
use horizon_client::mobs::{Damage, DamageKind, Mob, Who};
use horizon_core::geometry::Vec3;
use mod_api::{
    self, Hooks, HurtSource, LevelRandomHost, LivingIncomingDamageEvent, LivingKnockbackEvent,
};
use std::sync::{Arc, Mutex};

/// What a knockback heard: the attacker and the stacks it holds (by item id).
type Held = (i32, Option<String>, Option<String>);

#[derive(Default)]
struct Heard {
    knocks: Vec<Held>,
    incoming_causing: Vec<i32>,
    cancel: bool,
}

struct Probe(Arc<Mutex<Heard>>);

#[mod_api::__subscriptions]
impl Hooks for Probe {
    fn living_knockback(
        &self,
        _: &mut dyn LevelRandomHost,
        event: &mut LivingKnockbackEvent<'_>,
    ) -> mod_api::Flow {
        let mut heard = self.0.lock().unwrap();
        let id = |s: Option<&ItemStack>| s.map(|s| s.item.to_string());
        heard.knocks.push((
            event.attacker,
            id(event.attacker_main),
            id(event.attacker_off),
        ));
        event.strength *= 2.0;
        event.x = -event.x;
        if heard.cancel {
            return mod_api::Flow::Cancel;
        }
        mod_api::Flow::Continue
    }
    fn living_incoming_damage(
        &self,
        _: &mut dyn LevelRandomHost,
        event: &mut LivingIncomingDamageEvent<'_>,
    ) -> mod_api::Flow {
        self.0
            .lock()
            .unwrap()
            .incoming_causing
            .push(event.source.causing);
        mod_api::Flow::Continue
    }
}

fn heard(test: impl FnOnce(&mut super::Game<'_>, &Mutex<Heard>)) -> Heard {
    let log = Arc::new(Mutex::new(Heard::default()));
    let mods: mod_api::Mods = vec![mod_api::ModEntry::new(
        "probe",
        Box::new(Probe(log.clone())),
    )]
    .into();
    mod_api::testing::with_mods(mods, || with_game(|game| test(game, &log)));
    std::mem::take(&mut *log.lock().unwrap())
}

fn source(causing: i32) -> HurtSource<'static> {
    HurtSource {
        damage_type: "minecraft:player_attack",
        causing,
        direct: 0,
        position: None,
    }
}

const PUSH: Knock = Knock {
    strength: 0.4,
    x: 1.0,
    z: 0.0,
};

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn the_bridge_names_the_attacker_and_what_it_holds() {
    let heard = heard(|game, _| {
        let mob = {
            let kind = game
                .res
                .mob_types()
                .get("minecraft:zombie")
                .unwrap()
                .clone();
            let mut mob = Mob::new(12, kind, Vec3::ZERO, 0.0);
            mob.ai.last_hurt_by = Some(Who::Mob(99));
            mob
        };
        let by_player = Damage {
            attacker: Some(Who::Player),
            ..Damage::of(DamageKind::Generic)
        };
        let by_mob = Damage {
            attacker: Some(Who::Mob(3)),
            ..by_player
        };
        let by_nobody = Damage {
            attacker: None,
            ..by_player
        };
        let mut push = |pushed: Hurting<'_>| {
            let mut knock = PUSH;
            let canceled = super::native_hooks::knockback(
                damage_level_of!(game).reborrow(),
                pushed,
                &mut knock,
            );
            (canceled, knock)
        };
        {
            let _scope = knockback_from(7);
            for pushed in [
                Hurting::Mob(&mob, &by_player),
                Hurting::Player(12, source(0)),
            ] {
                // The probe doubles the strength and mirrors the direction.
                let (canceled, knock) = push(pushed);
                assert!(!canceled);
                assert_eq!(
                    knock,
                    Knock {
                        strength: 0.8,
                        x: -1.0,
                        z: 0.0
                    }
                );
            }
            push(Hurting::Mob(&mob, &by_mob));
            push(Hurting::Mob(&mob, &by_nobody));
            push(Hurting::Dragon(12, source(5)));
            push(Hurting::ArmorStand(12, source(0)));
        }
        {
            let _held =
                knockback_from_holding(7, [Some(ItemStack::new("minecraft:iron_sword", 1)), None]);
            push(Hurting::Player(12, source(0)));
        }
        push(Hurting::Player(12, source(0)));
    });
    let sword = Some("minecraft:iron_sword".to_string());
    let attackers = [7, 7, 3, 99, 5, 7, 7, 0].map(|attacker| (attacker, None, None));
    let mut expected = attackers.to_vec();
    expected[6].1 = sword;
    assert_eq!(heard.knocks, expected);
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_canceled_push_is_reported() {
    let heard = heard(|game, log| {
        let level = damage_level_of!(game);
        log.lock().unwrap().cancel = true;
        let mut knock = PUSH;
        assert!(super::native_hooks::knockback(
            level.reborrow(),
            Hurting::Player(12, source(7)),
            &mut knock
        ));
    });
    assert_eq!(heard.knocks.len(), 1);
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn thorns_name_the_wearer_not_the_incoming_attacker() {
    use horizon_client::motion::PlayerMotion;
    let heard = heard(|game, _| {
        let mut player = PlayerMotion::new(game.motion, Vec3::ZERO).unwrap();
        player.entity_id = 7;
        game.player = Some(player);
        let mut chest = ItemStack::new("minecraft:diamond_chestplate", 1);
        chest.data_mut().enchantments = vec![("minecraft:thorns".into(), 255)];
        game.armor[1] = Some(chest);
        let kind = game
            .res
            .mob_types()
            .get("minecraft:zombie")
            .unwrap()
            .clone();
        game.mobs.push(Mob::new(42, kind, Vec3::ZERO, 0.0));
        // The mob's own hit on the player is in progress: its attack names it.
        let _incoming = knockback_from(42);
        game.thorns(42);
        assert_eq!(knockback_attacker(), 42);
    });
    assert_eq!(heard.incoming_causing, [7]);
}
