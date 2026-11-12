//! `makes_piglins_neutral` answered by item Parts on vanilla items: a leather helmet made to
//! count as gold, golden boots made not to. The answer is what the view of the player the
//! piglins read carries (`gold_armor`, `PiglinAi.isWearingSafeArmor`, which
//! `NEAREST_TARGETABLE_PLAYER_NOT_WEARING_GOLD` reads).
use super::native_entity_refs::tests::with_game;
use horizon_client::items::ItemStack;
use mod_api::{Hooks, Host, ItemPart, MakesPiglinsNeutralEvent, Registrar, RegistrationContext};
use std::sync::atomic::{AtomicU32, Ordering};

/// The entity id of the last `wearer` a part was asked about.
static WEARER: AtomicU32 = AtomicU32::new(u32::MAX);

/// Answers `makes_piglins_neutral` for its item.
#[derive(Clone)]
struct Neutral(bool);

#[mod_api::horizon_part]
impl ItemPart for Neutral {
    fn makes_piglins_neutral(&self, _: &dyn Host, event: &mut MakesPiglinsNeutralEvent<'_>) {
        WEARER.store(event.wearer.id(), Ordering::Relaxed);
        event.neutral = self.0;
    }
}

/// A leather helmet counts as gold; golden boots don't.
struct NeutralMod;

#[mod_api::__subscriptions]
impl Hooks for NeutralMod {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        let helmet = r.item_key("minecraft:leather_helmet");
        r.item_part(helmet, Neutral(true));
        let boots = r.item_key("minecraft:golden_boots");
        r.item_part(boots, Neutral(false));
    }
}

/// The player wears `head` and `feet` and nothing else; whether calm piglins leave them alone.
fn wearing(game: &mut super::Game<'_>, head: Option<&str>, feet: Option<&str>) -> bool {
    game.armor = [
        head.map(|i| ItemStack::new(i, 1)),
        None,
        None,
        feet.map(|i| ItemStack::new(i, 1)),
    ];
    game.player_view().unwrap().gold_armor
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES and an isolated mod registry"]
fn a_part_decides_whether_a_worn_piece_counts_as_gold() {
    crate::game::in_own_process(
        "game::native_makes_piglins_neutral_tests::a_part_decides_whether_a_worn_piece_counts_as_gold",
        a_part_decides_whether_a_worn_piece_counts_as_gold_isolated,
    );
}

fn a_part_decides_whether_a_worn_piece_counts_as_gold_isolated() {
    let mods: mod_api::Mods = vec![mod_api::ModEntry::new("neutral", Box::new(NeutralMod))].into();
    mod_api::testing::with_mods(mods.clone(), || {
        crate::game::api_values::install_parts(&mods).unwrap();
        with_game(|game| {
            game.options.dedicated = Some(Vec::new());
            let player = game.player.as_ref().unwrap().entity_id as u32;
            assert!(wearing(game, Some("minecraft:leather_helmet"), None));
            assert_eq!(
                WEARER.load(Ordering::Relaxed),
                player,
                "the part sees the wearer"
            );
            // Golden boots the part refuses don't count, though the tag lists them.
            assert!(!wearing(game, None, Some("minecraft:golden_boots")));
            // A tagged piece no part is attached to keeps vanilla's answer.
            assert!(wearing(game, Some("minecraft:golden_helmet"), None));
            assert!(!wearing(game, Some("minecraft:iron_helmet"), None));
            // Any one piece that counts is enough.
            assert!(wearing(
                game,
                Some("minecraft:leather_helmet"),
                Some("minecraft:golden_boots")
            ));
        });
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES and an isolated mod registry"]
fn without_parts_the_piglin_safe_armor_tag_decides() {
    crate::game::in_own_process(
        "game::native_makes_piglins_neutral_tests::without_parts_the_piglin_safe_armor_tag_decides",
        without_parts_the_piglin_safe_armor_tag_decides_isolated,
    );
}

fn without_parts_the_piglin_safe_armor_tag_decides_isolated() {
    with_game(|game| {
        game.options.dedicated = Some(Vec::new());
        assert!(!wearing(game, Some("minecraft:leather_helmet"), None));
        assert!(wearing(game, None, Some("minecraft:golden_boots")));
        assert!(!wearing(game, None, None));
    });
}
