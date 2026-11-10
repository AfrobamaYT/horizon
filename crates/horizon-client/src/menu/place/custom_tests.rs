use super::*;
use crate::{
    crafting::{Ingredient, Placement},
    recipe_book::slot_layout::CheckedSlotLayout,
};
use mod_api::RecipeBookLayout;

fn kind() -> Kind {
    Kind::Mod(Box::leak(Box::new(crate::containers::ModMenuType {
        id: "test:custom",
        slots: 10,
        data_slots: 0,
        results: vec![8, 9],
        crafting: None,
        links: None,
    })))
}
fn with_env(test: impl FnOnce(&Env)) {
    let mut ran = false;
    crate::menu::tests::with_menu(Kind::Crafting, |env, _| {
        ran = true;
        test(env);
    });
    assert!(
        ran,
        "prepared resources are required for this explicitly ignored test"
    );
}
fn single(item: &str) -> MenuPlacement {
    let layout = CheckedSlotLayout::new(
        RecipeBookLayout::Furnace {
            input: 4,
            fuel: 7,
            result: 9,
        },
        10,
    )
    .unwrap();
    MenuPlacement::new(
        "test:custom".into(),
        Placement::of(vec![Some(Ingredient::of(vec![item.into()]))], None),
        &layout,
    )
    .unwrap()
}
fn smith() -> MenuPlacement {
    let layout = CheckedSlotLayout::new(
        RecipeBookLayout::Smithing {
            template: 2,
            base: 5,
            addition: 3,
            result: 8,
        },
        10,
    )
    .unwrap();
    MenuPlacement::new(
        "test:custom".into(),
        Placement::of(
            [
                "minecraft:oak_planks",
                "minecraft:cobblestone",
                "minecraft:oak_planks",
            ]
            .into_iter()
            .map(|id| Some(Ingredient::of(vec![id.into()])))
            .collect(),
            None,
        ),
        &layout,
    )
    .unwrap()
}
fn stack(id: &str, count: u32) -> Option<ItemStack> {
    Some(ItemStack::new(id, count))
}

#[test]
#[ignore = "requires prepared resources"]
fn noncontiguous_roles_place_one_increment_and_max_without_granting_a_result() {
    with_env(|env| {
        let mut menu = Menu::new(kind());
        menu.set_player_item(0, stack("minecraft:oak_planks", 10));
        menu.set_player_item(5, stack("minecraft:cobblestone", 4));
        menu.load_slot(7, stack("minecraft:coal", 3));
        let plan = smith();
        for (use_max, expected) in [(false, 1), (false, 2), (true, 4)] {
            assert_eq!(
                menu.place_custom_recipe(env, &plan, use_max, false, || true),
                Ok(PostPlace::Nothing)
            );
            for slot in [2, 5, 3] {
                assert_eq!(menu.slot(slot).map(|s| s.count), Some(expected));
            }
            assert!(menu.slot(8).is_none());
            assert_eq!(menu.slot(7).map(|s| s.count), Some(3));
        }
        assert_eq!(menu.player_item(0).map(|s| s.count), Some(2));
        assert!(menu.player_item(5).is_none());
    });
}
#[test]
#[ignore = "requires prepared resources"]
fn ingredient_shortage_returns_old_inputs_but_never_fills_fuel_or_output() {
    with_env(|env| {
        let mut menu = Menu::new(kind());
        menu.load_slot(4, stack("minecraft:dirt", 1));
        menu.load_slot(7, stack("minecraft:coal", 3));
        assert_eq!(
            menu.place_custom_recipe(env, &single("minecraft:iron_ore"), false, false, || true),
            Ok(PostPlace::Ghost)
        );
        assert!(menu.slot(4).is_none());
        assert!(menu.slot(9).is_none());
        assert_eq!(
            menu.player_item(0).map(|s| (&*s.item, s.count)),
            Some(("minecraft:dirt", 1))
        );
        assert_eq!(menu.slot(7).map(|s| s.count), Some(3));
    });
}
#[test]
#[ignore = "requires prepared resources"]
fn full_inventory_refuses_unchanged_in_survival_and_defers_creative_drops() {
    with_env(|env| {
        let mut menu = Menu::new(kind());
        for slot in 0..36 {
            menu.set_player_item(slot, stack("minecraft:stone", 64));
        }
        menu.load_slot(4, stack("minecraft:dirt", 1));
        let before = menu.container_items();
        assert_eq!(
            menu.place_custom_recipe(env, &single("minecraft:iron_ore"), false, false, || true),
            Err(Refused::NoRoom)
        );
        assert_eq!(menu.container_items(), before);
        assert!(menu.dropped.is_empty());
        assert_eq!(
            menu.place_custom_recipe(env, &single("minecraft:iron_ore"), false, true, || true),
            Ok(PostPlace::Ghost)
        );
        assert_eq!(menu.dropped, vec![ItemStack::new("minecraft:dirt", 1)]);
        assert!((0..36).all(|s| menu.player_item(s).is_some_and(|stack| stack.count == 64)));
    });
}
#[test]
#[ignore = "requires prepared resources"]
fn per_slot_caps_limit_max_and_policy_refusal_is_not_a_missing_ingredient_ghost() {
    with_env(|env| {
        let mut menu = Menu::new(kind());
        menu.set_player_item(0, stack("minecraft:iron_ore", 10));
        let mut limits = vec![None; 10];
        limits[4] = Some(2);
        menu.set_container_limits(limits);
        assert_eq!(
            menu.place_custom_recipe(env, &single("minecraft:iron_ore"), true, false, || true),
            Ok(PostPlace::Nothing)
        );
        assert_eq!(menu.slot(4).map(|s| s.count), Some(2));
        menu.hide_slot(4);
        let before = menu.container_items();
        assert_eq!(
            menu.place_custom_recipe(env, &single("minecraft:iron_ore"), false, false, || true),
            Err(Refused::Denied)
        );
        assert_eq!(menu.container_items(), before);
    });
}
#[test]
#[ignore = "requires prepared resources"]
fn component_distinct_stacks_never_pool_into_one_destination() {
    with_env(|env| {
        let mut menu = Menu::new(kind());
        menu.set_player_item(
            0,
            Some(ItemStack::new("minecraft:paper", 1).with_potion("minecraft:swiftness")),
        );
        menu.set_player_item(
            1,
            Some(ItemStack::new("minecraft:paper", 1).with_potion("minecraft:healing")),
        );
        assert_eq!(
            menu.place_custom_recipe(env, &single("minecraft:paper"), true, false, || true),
            Ok(PostPlace::Nothing)
        );
        assert_eq!(
            menu.slot(4).map(|s| (s.count, s.potion())),
            Some((1, Some("minecraft:swiftness")))
        );
        assert!(menu.player_item(0).is_none());
        assert_eq!(
            menu.player_item(1).map(|s| s.potion()),
            Some(Some("minecraft:healing"))
        );
    });
}
