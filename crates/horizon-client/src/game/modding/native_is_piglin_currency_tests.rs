//! `is_piglin_currency` answered by item Parts on vanilla items: an emerald made currency is
//! taken from the hand, admired and bartered; a gold ingot refused as currency is no longer
//! taken from the hand, and once picked up from the ground (it stays `#piglin_loved`) is admired
//! and kept instead of bartered. Expected values follow vanilla's `PiglinAi` flow by hand.
use super::native_entity_refs::tests::with_game;
use horizon_client::first_person::Hand;
use horizon_client::items::ItemStack;
use horizon_client::mobs::{Mob, MobEvent, piglin};
use horizon_core::geometry::Vec3;
use mod_api::{Hooks, Host, IsPiglinCurrencyEvent, ItemPart, Registrar, RegistrationContext};
use std::sync::Arc;

/// Answers `is_piglin_currency` for its item.
#[derive(Clone)]
struct Currency(bool);

#[mod_api::horizon_part]
impl ItemPart for Currency {
    fn is_piglin_currency(&self, _: &dyn Host, event: &mut IsPiglinCurrencyEvent<'_>) {
        event.currency = self.0;
    }
}

/// An emerald is currency; a gold ingot isn't.
struct CurrencyMod;

#[mod_api::__subscriptions]
impl Hooks for CurrencyMod {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        let emerald = r.item_key("minecraft:emerald");
        r.item_part(emerald, Currency(true));
        let gold = r.item_key("minecraft:gold_ingot");
        r.item_part(gold, Currency(false));
    }
}

const EMERALD: &str = "minecraft:emerald";
const GOLD_INGOT: &str = "minecraft:gold_ingot";
const GOLDEN_SWORD: &str = "minecraft:golden_sword";

/// A serving level with one chunk of air, and a survival player.
fn field(game: &mut super::Game<'_>) {
    game.options.dedicated = Some(Vec::new());
    game.world = horizon_client::world::World::new(game.res.facts().air, 1);
    game.world
        .chunks
        .insert((0, 0), Arc::new(horizon_client::chunk::Chunk::empty(0, 0)));
    game.creative = false;
}

/// An adult piglin at `(x, 64, 0.5)` holding `main` in its main hand; its index.
fn add_piglin(game: &mut super::Game<'_>, x: f64, main: Option<&str>) -> usize {
    let kind = game
        .res
        .mob_types()
        .get("minecraft:piglin")
        .unwrap()
        .clone();
    game.next_entity += 1;
    let mut m = Mob::new(game.next_entity, kind, Vec3::new(x, 64.0, 0.5), 0.0);
    m.extra.set_main_hand(main.map(Into::into));
    game.mobs.push(m);
    game.mobs.len() - 1
}

/// Right click on the piglin at `index` holding `held`; whether the click was taken.
fn click(game: &mut super::Game<'_>, index: usize, held: ItemStack) -> bool {
    game.inventory[game.selected] = Some(held);
    let id = game.mobs[index].id as i32;
    game.interact_mob(Some(crate::game::InteractTarget {
        id,
        location: [0.0; 3],
    }))
    .unwrap()
}

/// The admiring of the piglin at `index` runs out (the 119th tick raises `FinishAdmiring`).
fn admiring_runs_out(game: &mut super::Game<'_>, index: usize) {
    assert_eq!(game.mobs[index].extra.using_item, piglin::ADMIRE);
    let mob = game.mobs[index].id;
    game.handle_mob_events(vec![MobEvent::FinishAdmiring { mob }])
        .unwrap();
}

/// A player's hit stops the admiring: `ai::after_hurt` and `piglin_hurt` both stop it, and the
/// game applies what they raised.
fn hit(game: &mut super::Game<'_>, index: usize) {
    piglin::stop_admiring(&mut game.mobs[index]);
    piglin::stop_admiring(&mut game.mobs[index]);
    let raised = std::mem::take(&mut game.mobs[index].outbox);
    game.handle_mob_events(raised).unwrap();
}

/// Drop one `item` right beside the piglin at `index`, ready to be picked up, and let that
/// piglin, the only one ticked, pick up what it wants.
fn drop_beside(game: &mut super::Game<'_>, index: usize, item: &str) {
    let at = game.mobs[index].position;
    game.spawn_item(
        ItemStack::new(item, 1),
        [at.x + 0.5, at.y, at.z],
        [0.0; 3],
        0,
    );
    let id = game.mobs[index].id;
    game.creatures.ticked.clear();
    game.creatures.ticked.insert(id);
    game.piglin_pickups();
}

fn holds(m: &Mob, item: &str) -> usize {
    m.extra
        .inventory
        .iter()
        .filter(|s| &*s.item == item)
        .map(|s| s.count as usize)
        .sum()
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES and an isolated mod registry"]
fn a_part_makes_an_emerald_currency_and_refuses_the_gold_ingot() {
    crate::game::in_own_process(
        "game::native_is_piglin_currency_tests::a_part_makes_an_emerald_currency_and_refuses_the_gold_ingot",
        a_part_makes_an_emerald_currency_and_refuses_the_gold_ingot_isolated,
    );
}

fn a_part_makes_an_emerald_currency_and_refuses_the_gold_ingot_isolated() {
    let mods: mod_api::Mods =
        vec![mod_api::ModEntry::new("currency", Box::new(CurrencyMod))].into();
    mod_api::testing::with_mods(mods.clone(), || {
        crate::game::api_values::install_parts(&mods).unwrap();
        with_game(|game| {
            field(game);

            // Hand-over: the click is taken, one emerald leaves the hand for the off hand,
            // and after admiring it is bartered: gone, with barter items thrown.
            let p = add_piglin(game, 0.5, None);
            assert!(click(game, p, ItemStack::new(EMERALD, 3)));
            assert_eq!(game.inventory[game.selected].as_ref().unwrap().count, 2);
            assert_eq!(game.mobs[p].extra.off_hand_id(), Some(EMERALD));
            assert!(game.dropped.is_empty());
            admiring_runs_out(game, p);
            assert_eq!(game.mobs[p].extra.off_hand_id(), None);
            assert_eq!(game.mobs[p].extra.main_hand_id(), None);
            assert_eq!(holds(&game.mobs[p], EMERALD), 0);
            assert!(!game.dropped.is_empty(), "the barter is thrown");
            game.dropped.clear();

            // The client's prediction: an adult that isn't admiring takes the emerald, a baby
            // doesn't.
            let adult = add_piglin(game, 4.5, None);
            game.inventory[game.selected] = Some(ItemStack::new(EMERALD, 1));
            assert!(game.remote_mob_would_take(adult, Hand::Main));
            game.mobs[adult].extra.age = -24000;
            assert!(!game.remote_mob_would_take(adult, Hand::Main));

            // Refusing gold: a piglin with a golden sword doesn't take the ingot from the hand.
            let p = add_piglin(game, 8.5, Some(GOLDEN_SWORD));
            assert!(!click(game, p, ItemStack::new(GOLD_INGOT, 1)));
            assert_eq!(game.inventory[game.selected].as_ref().unwrap().count, 1);
            assert_eq!(game.mobs[p].extra.off_hand_id(), None);
            // A dropped ingot is still picked up and admired (`#piglin_loved`), then kept:
            // the sword stays in the main hand, the ingot goes to the inventory, nothing is
            // bartered.
            drop_beside(game, p, GOLD_INGOT);
            assert_eq!(game.mobs[p].extra.off_hand_id(), Some(GOLD_INGOT));
            assert!(game.dropped.is_empty());
            admiring_runs_out(game, p);
            assert_eq!(game.mobs[p].extra.main_hand_id(), Some(GOLDEN_SWORD));
            assert_eq!(holds(&game.mobs[p], GOLD_INGOT), 1);
            assert!(game.dropped.is_empty(), "no barter");

            // Ground currency needs `#piglin_loved`: an emerald is picked up without admiring,
            // and a wanted sword isn't replaced by it, so it goes to the inventory.
            let p = add_piglin(game, 12.5, Some(GOLDEN_SWORD));
            drop_beside(game, p, EMERALD);
            assert_eq!(game.mobs[p].extra.off_hand_id(), None);
            assert_eq!(game.mobs[p].extra.using_item, 0);
            assert_eq!(game.mobs[p].extra.main_hand_id(), Some(GOLDEN_SWORD));
            assert_eq!(holds(&game.mobs[p], EMERALD), 1);
            assert!(game.dropped.is_empty());

            // Hit: an adult admiring an emerald loses it, nothing thrown or stored.
            let p = add_piglin(game, 16.5, None);
            assert!(click(game, p, ItemStack::new(EMERALD, 1)));
            hit(game, p);
            assert_eq!(game.mobs[p].extra.off_hand_id(), None);
            assert_eq!(game.mobs[p].extra.main_hand_id(), None);
            assert_eq!(holds(&game.mobs[p], EMERALD), 0);
            assert!(game.dropped.is_empty());
            // The hit's stop bars only that one barter: the next hand-over is bartered.
            game.mobs[p].extra.piglin.admire_disabled = 0;
            assert!(click(game, p, ItemStack::new(EMERALD, 1)));
            admiring_runs_out(game, p);
            assert!(!game.dropped.is_empty(), "the barter is thrown");
        });
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES and an isolated mod registry"]
fn without_parts_only_the_gold_ingot_is_currency() {
    crate::game::in_own_process(
        "game::native_is_piglin_currency_tests::without_parts_only_the_gold_ingot_is_currency",
        without_parts_only_the_gold_ingot_is_currency_isolated,
    );
}

fn without_parts_only_the_gold_ingot_is_currency_isolated() {
    with_game(|game| {
        field(game);
        // The emerald click passes and the player keeps 3.
        let p = add_piglin(game, 0.5, None);
        assert!(!click(game, p, ItemStack::new(EMERALD, 3)));
        assert_eq!(game.inventory[game.selected].as_ref().unwrap().count, 3);
        assert_eq!(game.mobs[p].extra.off_hand_id(), None);
        let adult = add_piglin(game, 4.5, None);
        game.inventory[game.selected] = Some(ItemStack::new(EMERALD, 1));
        assert!(!game.remote_mob_would_take(adult, Hand::Main));

        // A gold ingot is taken, and lost to a hit: an adult keeps nothing.
        let p = add_piglin(game, 8.5, None);
        assert!(click(game, p, ItemStack::new(GOLD_INGOT, 1)));
        assert_eq!(game.inventory[game.selected], None);
        hit(game, p);
        assert_eq!(game.mobs[p].extra.off_hand_id(), None);
        assert_eq!(game.mobs[p].extra.main_hand_id(), None);
        assert_eq!(holds(&game.mobs[p], GOLD_INGOT), 0);
        assert!(game.dropped.is_empty());

        // An admired golden sword a hit stops is kept: an empty main hand takes it.
        let p = add_piglin(game, 12.5, None);
        drop_beside(game, p, GOLDEN_SWORD);
        assert_eq!(game.mobs[p].extra.off_hand_id(), Some(GOLDEN_SWORD));
        hit(game, p);
        assert_eq!(game.mobs[p].extra.off_hand_id(), None);
        assert_eq!(game.mobs[p].extra.main_hand_id(), Some(GOLDEN_SWORD));
        assert!(game.dropped.is_empty());

        // A baby admiring a picked-up ingot keeps it when hit (`stopHoldingOffHandItem`'s
        // baby branch): it holds it in its main hand.
        let p = add_piglin(game, 16.5, None);
        game.mobs[p].extra.age = -24000;
        drop_beside(game, p, GOLD_INGOT);
        assert_eq!(game.mobs[p].extra.off_hand_id(), Some(GOLD_INGOT));
        hit(game, p);
        assert_eq!(game.mobs[p].extra.off_hand_id(), None);
        assert_eq!(game.mobs[p].extra.main_hand_id(), Some(GOLD_INGOT));
    });
}
