//! `ItemDrop`'s item-entity state (#3423): `thrower`, `target`, `no_gravity` and `age` set by a
//! drop hook reach the item entity that joins the level; a drop left as it was joins as before.
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use horizon_client::{entities::ItemEntity, items::ItemStack, lan::playerdata::uuid_ints};
use horizon_core::geometry::Vec3;
use mod_api::{self, Flow, Hooks, ItemDrop, LivingDropsEvent, ServerHost};

use super::spill::{BlockItemDrop, DropState, prepare_item_random};

const KILLER: u128 = 0x1234;
const THROWER: u128 = 0x5678;

fn drop_with(state: DropState, random: bool) -> BlockItemDrop {
    BlockItemDrop {
        random: random.then(|| prepare_item_random(None)),
        stack: ItemStack::new("minecraft:rotten_flesh", 1),
        position: [0.5, 64.0, 0.5],
        velocity: [0.1, 0.2, 0.0],
        pickup_delay: 10,
        state,
    }
}

fn set() -> DropState {
    DropState {
        thrower: Some(THROWER),
        target: Some(KILLER),
        no_gravity: true,
        age: ItemDrop::UNLIMITED_LIFETIME,
    }
}

#[test]
fn an_unedited_drop_keeps_its_random_as_it_was() {
    let drawn = drop_with(DropState::default(), true);
    assert_eq!(drawn.random_with_state(), drawn.random);
    assert_eq!(
        drop_with(DropState::default(), false).random_with_state(),
        None
    );
}

#[test]
fn a_drop_with_state_carries_it_into_its_random() {
    let drawn = drop_with(set(), true);
    let random = drawn.random_with_state().unwrap();
    assert_eq!(random.state, set());
    assert_eq!(
        random.uuid,
        drawn.random.unwrap().uuid,
        "the drawn random is kept"
    );
    // An edited drop has no random yet: one is drawn with the drop's own motion.
    let random = drop_with(set(), false).random_with_state().unwrap();
    assert_eq!((random.state, random.velocity), (set(), [0.1, 0.2, 0.0]));
}

#[test]
fn the_state_reaches_the_item_entity() {
    let fresh = || {
        ItemEntity::new(
            1,
            ItemStack::new("minecraft:stone", 1),
            Vec3::ZERO,
            Vec3::ZERO,
            0.0,
        )
    };
    let item = set().apply(fresh());
    assert_eq!(item.thrower, Some(uuid_ints(THROWER)));
    assert_eq!(item.target, Some(uuid_ints(KILLER)));
    assert!(item.no_gravity);
    assert_eq!(item.age, ItemDrop::UNLIMITED_LIFETIME);
    // The default state is a fresh item entity's.
    let plain = DropState::default().apply(fresh());
    let new = fresh();
    assert_eq!(
        (plain.thrower, plain.target, plain.no_gravity, plain.age),
        (new.thrower, new.target, new.no_gravity, new.age)
    );
}

/// A `living_drops` listener that gives every drop the state of [`set`], or leaves them.
struct Marker(Arc<AtomicBool>);
#[mod_api::__subscriptions]
impl Hooks for Marker {
    fn living_drops(&self, _: &mut dyn ServerHost, event: &mut LivingDropsEvent<'_>) -> Flow {
        if self.0.load(Ordering::SeqCst) {
            for drop in event.drops.iter_mut() {
                *drop = drop
                    .clone()
                    .with_thrower(Some(THROWER))
                    .with_target(Some(KILLER))
                    .with_no_gravity(true)
                    .with_age(ItemDrop::UNLIMITED_LIFETIME);
            }
        }
        Flow::Continue
    }
}

/// A copper golem's death drops (its preserved diamond included) join with the state the
/// listener set, through the API drop and back; with the listener leaving them alone they join
/// as fresh item entities.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn living_drops_state_reaches_the_dropped_item_entities() {
    let mark = Arc::new(AtomicBool::new(true));
    let mods: mod_api::Mods = vec![mod_api::ModEntry::new(
        "drop-marker",
        Box::new(Marker(mark.clone())),
    )]
    .into();
    mod_platform::testing::with_mods(mods, || {
        super::native_entity_refs::tests::with_game(|game| {
            game.load_server_loot();
            let golem_type = game
                .res
                .mob_types()
                .get("minecraft:copper_golem")
                .unwrap()
                .clone();
            let die = |game: &mut super::Game<'_>| {
                game.next_entity += 1;
                let mut golem = horizon_client::mobs::Mob::new(
                    game.next_entity,
                    golem_type.clone(),
                    Vec3::new(0.5, 64.0, 0.5),
                    0.0,
                );
                horizon_client::mobs::transport_items::set_held(
                    &mut golem,
                    Some(ItemStack::new("minecraft:diamond", 1)),
                );
                golem.health = 0.0;
                game.mobs.push(golem.clone());
                let before = game.dropped.len();
                game.mob_died(&golem);
                game.dropped[before..].to_vec()
            };
            let marked = die(game);
            assert!(!marked.is_empty());
            for item in &marked {
                assert_eq!(item.thrower, Some(uuid_ints(THROWER)));
                assert_eq!(item.target, Some(uuid_ints(KILLER)));
                assert!(item.no_gravity);
                assert_eq!(item.age, ItemDrop::UNLIMITED_LIFETIME);
            }
            mark.store(false, Ordering::SeqCst);
            let plain = die(game);
            assert!(!plain.is_empty());
            for item in &plain {
                assert_eq!((item.thrower, item.target), (None, None));
                assert!(!item.no_gravity);
                assert_eq!(item.age, 0);
            }
        });
    });
}
