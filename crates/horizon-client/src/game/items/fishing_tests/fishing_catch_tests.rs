// Included as fishing's child so fixtures can construct its private hook state.
use super::*;
use horizon_client::motion::PlayerMotion;
use std::sync::Arc;

struct Catch(u8);
#[mod_api::__subscriptions]
impl mod_api::Hooks for Catch {
    fn item_fished(
        &self,
        host: &mut dyn mod_api::ServerHost,
        event: &mut mod_api::ItemFishedEvent<'_>,
    ) -> mod_api::Flow {
        match self.0 {
            1 => {
                event.drops.clear();
                event.drops.push(
                    mod_api::ItemStack::new(
                        host.item_key(&mod_api::Identifier::parse("minecraft:diamond").unwrap())
                            .unwrap(),
                        2,
                    )
                    .unwrap(),
                );
            }
            2 => {
                event.rod_damage = 5;
                return mod_api::Flow::Cancel;
            }
            3 => event.drops.clear(),
            _ => {}
        }
        mod_api::Flow::Continue
    }
}
fn run(mode: Option<u8>) -> (Vec<(String, u32)>, usize, i32, u32, i64, bool) {
    let mut result = None;
    mod_platform::testing::with_mods(Default::default(), || {
        crate::game::native_entity_refs::tests::with_game(|game| {
            let scratch = tempfile::tempdir().unwrap();
            game.world_root = scratch.path().into();
            game.save = Arc::new(horizon_client::save::SaveDir::new(
                scratch.path().join("save"),
                horizon_client::save::LevelShape::OVERWORLD,
            ));
            game.player = Some(PlayerMotion::new(game.motion, Vec3::new(0.5, 64.0, 0.5)).unwrap());
            game.lan_acting_uuid = Some(7);
            game.next_entity = 1000;
            game.level.seed = 12345;
            *game.world.random = LegacyRandom::new(12345);
            let advancement = scratch.path().join("test/advancement/catch.json");
            std::fs::create_dir_all(advancement.parent().unwrap()).unwrap();
            std::fs::write(
                &advancement,
                r#"{"criteria":{"catch":{"trigger":"minecraft:fishing_rod_hooked"}}}"#,
            )
            .unwrap();
            game.adv.tree = Arc::new(horizon_client::advancements::Tree::load(scratch.path()));
            game.adv.loaded = true;
            let loot = horizon_world::loot::LootData::load(
                &game.options.resources,
                &Default::default(),
                game.res.item_defaults(),
            )
            .unwrap();
            game.server_loot.set(Some(Arc::new(loot)));
            let mut hook = Hook {
                uuid: [0; 4],
                attachment_lifetime: Default::default(),
                join: horizon_client::join_seq::JoinSeq::UNJOINED,
                caster: None,
                caster_generation: horizon_client::join_seq::JoinSeq::UNJOINED,
                id: 999,
                position: Vec3::new(2.0, 64.0, 2.0),
                previous: Vec3::ZERO,
                velocity: Vec3::ZERO,
                state: State::Bobbing,
                last_deflected_by: None,
                needs_sync: false,
                tick_count: 0,
                life: 0,
                on_ground: false,
                nibble: 1,
                time_until_lured: 0,
                time_until_hooked: 0,
                fish_angle: 0.0,
                out_of_water: 0,
                open_water: true,
                luck: 0,
                lure: 0,
                medium: None,
                lerp: HookLerp::default(),
                random: LegacyRandom::new(42),
            };
            let mods = mode
                .map(|m| vec![mod_api::ModEntry::new("catch", Box::new(Catch(m)))])
                .unwrap_or_default()
                .into();
            let before_items = game.dropped.len();
            let before_orbs = game.orbs.len();
            let stat = |game: &Game<'_>| {
                game.stats
                    .values
                    .get("minecraft:custom")
                    .and_then(|v| v.get("minecraft:fish_caught"))
                    .copied()
                    .unwrap_or(0)
            };
            let before_stat = stat(game);
            let damage = mod_platform::testing::with_mods(mods, || {
                game.retrieve_hook_extracted(&ItemStack::new("minecraft:fishing_rod", 1), &mut hook)
            });
            result = Some((
                game.dropped
                    .iter()
                    .skip(before_items)
                    .map(|item| (item.stack.item.to_string(), item.stack.count))
                    .collect(),
                game.orbs.len() - before_orbs,
                stat(game) - before_stat,
                damage,
                hook.random.next_long(),
                game.adv.progress.has("test:catch", "catch"),
            ));
        });
    });
    result.unwrap()
}
#[test]
#[ignore = "requires prepared resources"]
fn fishing_catch_replaces_rolled_stack() {
    let (drops, orbs, stat, damage, _, triggered) = run(Some(1));
    assert_eq!(drops, vec![("minecraft:diamond".into(), 2)]);
    assert_eq!((orbs, stat, damage), (1, 0, 1));
    assert!(triggered);
}
#[test]
#[ignore = "requires prepared resources"]
fn fishing_catch_cancel_keeps_damage() {
    let (drops, orbs, stat, damage, _, triggered) = run(Some(2));
    assert!(drops.is_empty());
    assert_eq!((orbs, stat, damage), (0, 0, 5));
    assert!(!triggered);
}
#[test]
#[ignore = "requires prepared resources"]
fn fishing_catch_empty_keeps_damage() {
    let (drops, orbs, stat, damage, _, triggered) = run(Some(3));
    assert!(drops.is_empty());
    assert_eq!((orbs, stat, damage), (0, 0, 1));
    assert!(triggered);
}
#[test]
#[ignore = "requires prepared resources"]
fn fishing_catch_noop_listener_preserves_randomness() {
    let absent = run(None);
    assert!(!absent.0.is_empty());
    assert_eq!(absent, run(Some(0)));
}
