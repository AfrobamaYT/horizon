use super::*;
use horizon_core::nbt::Tag;
use horizon_world::native_load_context::{DataLoadOrigin, LoadContext, LoadInputs, LoopSide};

fn load() -> LoadContext {
    LoadContext::new(
        DataLoadOrigin::ServerDataLoad,
        std::sync::Arc::new(LoadInputs::new(Default::default(), Default::default())),
        [],
    )
}
fn parse(source: &str, items: &Items) -> Ingredient {
    Ingredient::parse(&serde_json::from_str(source).unwrap(), items).unwrap()
}
fn resources() -> Items {
    let root = std::path::PathBuf::from(
        std::env::var_os("HORIZON_TEST_RESOURCES").expect("prepared resources"),
    );
    let defaults = horizon_world::item_defaults::read(&root).unwrap();
    Items::catalog(&root, &horizon_world::loot::PackOverlay::NONE, &defaults).unwrap()
}
#[test]
#[ignore = "requires prepared resources"]
fn components_match_damage_ignore_count_and_refuse_empty() {
    let items = resources();
    let ingredient = parse(
        r#"{"type":"neoforge:components","items":"minecraft:diamond_sword","components":{"minecraft:damage":5}}"#,
        &items,
    );
    let load = load();
    let context = load.loop_context(LoopSide::Server, []);
    let mut sword = ItemStack::new("minecraft:diamond_sword", 3);
    sword.damage = 5;
    assert!(ingredient.test_in(&context, &sword));
    sword.damage = 0;
    assert!(!ingredient.test_in(&context, &sword));
    sword.damage = 5;
    sword.count = 0;
    assert!(!ingredient.test_in(&context, &sword));
    assert!(!ingredient.test_in(&context, &ItemStack::new("minecraft:air", 1)));
    assert!(!ingredient.is_simple());
}
#[test]
#[ignore = "requires prepared resources"]
fn components_strict_counts_max_damage_default_and_custom_name() {
    let items = resources();
    let ingredient = parse(
        r#"{"type":"neoforge:components","items":"minecraft:diamond_sword","components":{"minecraft:damage":5},"strict":true}"#,
        &items,
    );
    let load = load();
    let context = load.loop_context(LoopSide::Server, []);
    let mut sword = ItemStack::new("minecraft:diamond_sword", 1);
    sword.damage = 5;
    // MAX_DAMAGE is a real prototype component, absent from the literal patch.
    assert!(
        items.component_defaults["minecraft:diamond_sword"]
            .contains_key(&"minecraft:max_damage".into())
    );
    assert!(!ingredient.test_in(&context, &sword));
    sword.data_mut().set_loot_component(
        "minecraft:custom_name",
        &Some(Tag::String("Named sword".into())),
    );
    assert!(!ingredient.test_in(&context, &sword));

    // A literal supported-only prototype isolates strict extra-component behavior.
    let mut supported = Items::empty();
    supported.component_defaults = std::sync::Arc::new(std::collections::HashMap::from([
        ("minecraft:diamond_sword".into(), crate::json_nbt::json_to_compound(
            serde_json::json!({"minecraft:damage":0,"minecraft:max_damage":1561,"minecraft:max_stack_size":1}).as_object().unwrap()).unwrap())
    ]));
    let complete = parse(
        r#"{"type":"neoforge:components","items":"minecraft:diamond_sword","components":{"minecraft:damage":5,"minecraft:max_damage":1561,"minecraft:max_stack_size":1},"strict":true}"#,
        &supported,
    );
    let mut plain = ItemStack::new("minecraft:diamond_sword", 1);
    plain.damage = 5;
    assert!(complete.test_in(&context, &plain));
    assert!(!complete.test_in(&context, &sword));
    let missing_max = parse(
        r#"{"type":"neoforge:components","items":"minecraft:diamond_sword","components":{"minecraft:damage":5,"minecraft:max_stack_size":1},"strict":true}"#,
        &supported,
    );
    assert!(
        !missing_max.test_in(&context, &plain),
        "the literal max_damage default must be named"
    );
}
#[test]
#[ignore = "requires prepared resources"]
fn components_removal_requires_absence_including_defaults() {
    let items = resources();
    let ingredient = parse(
        r#"{"type":"neoforge:components","items":"minecraft:diamond_sword","components":{"!minecraft:enchantments":{}}}"#,
        &items,
    );
    assert!(!ingredient.shown.as_ref().unwrap()[0].carries_component(
        items.get("minecraft:diamond_sword"),
        "minecraft:enchantments"
    ));
    let load = load();
    let context = load.loop_context(LoopSide::Server, []);
    let mut sword = ItemStack::new("minecraft:diamond_sword", 1);
    assert!(!ingredient.test_in(&context, &sword));
    let patch = Tag::compound([("!minecraft:enchantments", Tag::Compound(Default::default()))]);
    (_, sword.extra) = crate::item_data::decode_components(Some(&patch));
    assert!(ingredient.test_in(&context, &sword));
    sword = ItemStack::new("minecraft:diamond_sword", 1);
    sword.data_mut().set_loot_component(
        "minecraft:enchantments",
        &Some(Tag::Compound(Default::default())),
    );
    assert!(!ingredient.test_in(&context, &sword));
}
#[test]
#[ignore = "requires prepared resources"]
fn block_tag_logs_uses_block_items_and_rejects_hash() {
    let items = resources();
    let ingredient = parse(
        r#"{"type":"neoforge:block_tag","tag":"minecraft:logs"}"#,
        &items,
    );
    let load = load();
    let context = load.loop_context(LoopSide::Server, []);
    assert!(ingredient.test_in(&context, &ItemStack::new("minecraft:oak_log", 1)));
    assert!(!ingredient.test_in(&context, &ItemStack::new("minecraft:stone", 1)));
    assert!(ingredient.is_simple());
    let mut alias_items = items.clone();
    alias_items.block_tag_order.insert(
        "fixture:wall_signs".into(),
        vec!["minecraft:oak_wall_sign".into()],
    );
    let wall = parse(
        r#"{"type":"neoforge:block_tag","tag":"fixture:wall_signs"}"#,
        &alias_items,
    );
    assert_eq!(wall.order, ["minecraft:oak_sign"]);
    assert!(
        Ingredient::parse(
            &serde_json::json!({"type":"neoforge:block_tag","tag":"#minecraft:logs"}),
            &items
        )
        .is_none()
    );
}
#[test]
fn custom_display_matches_base_but_shows_another_item() {
    let items = Items::empty();
    let ingredient = parse(
        r#"{"type":"neoforge:custom_display","base":"minecraft:apple","display":{"type":"minecraft:item","item":"minecraft:diamond"}}"#,
        &items,
    );
    let load = load();
    let context = load.loop_context(LoopSide::Server, []);
    assert!(ingredient.test_in(&context, &ItemStack::new("minecraft:apple", 1)));
    assert!(!ingredient.test_in(&context, &ItemStack::new("minecraft:diamond", 1)));
    assert!(ingredient.is_simple());
    assert_eq!(
        crate::recipe_book::Book::default().items_of(&ingredient),
        ["minecraft:diamond"]
    );
    assert_eq!(ingredient.order, ["minecraft:apple"]);
    let placement = crate::crafting::Placement::of(vec![Some(ingredient)], None);
    assert_eq!(placement.ingredients[0].order, ["minecraft:apple"]);
    // 26.3 TagSlotDisplay carries an item HolderSet, including the # prefix.
    let tags = Items::for_tests(std::collections::HashMap::from([(
        "minecraft:planks".into(),
        std::collections::HashSet::from(["minecraft:oak_planks".into()]),
    )]));
    let tagged = parse(
        r##"{"type":"neoforge:custom_display","base":"minecraft:apple","display":{"type":"minecraft:tag","tag":"#minecraft:planks"}}"##,
        &tags,
    );
    assert_eq!(
        crate::recipe_book::Book::default().items_of(&tagged),
        ["minecraft:oak_planks"]
    );
}
#[test]
fn simplicity_follows_all_children_and_custom_display_base() {
    let items = Items::empty();
    for source in [
        r#"{"type":"neoforge:compound","children":["minecraft:apple","minecraft:stone"]}"#,
        r#"{"type":"neoforge:intersection","children":["minecraft:apple","minecraft:apple"]}"#,
        r#"{"type":"neoforge:difference","base":["minecraft:apple","minecraft:stone"],"subtracted":"minecraft:stone"}"#,
    ] {
        assert!(parse(source, &items).is_simple());
    }
    let source = r#"{"type":"neoforge:custom_display","base":{"type":"neoforge:components","items":"minecraft:apple","components":{}},"display":{"type":"minecraft:item","item":"minecraft:diamond"}}"#;
    assert!(!parse(source, &items).is_simple());
    let mut custom = Ingredient::of(vec!["minecraft:apple".into()]);
    custom.custom = Some(("fixture:custom".into(), Default::default()));
    assert!(!custom.is_simple());
}

#[test]
fn builtin_kinds_bypass_mod_resolution_and_honor_type_key_priority() {
    let load = load();
    for source in [
        r#"{"type":"neoforge:components","items":"minecraft:diamond_sword","components":{"minecraft:damage":5}}"#,
        r#"{"type":"neoforge:block_tag","tag":"minecraft:logs"}"#,
        r#"{"type":"fixture:wrong","neoforge:ingredient_type":"neoforge:custom_display","base":"minecraft:apple","display":{"type":"minecraft:item","item":"minecraft:diamond"}}"#,
    ] {
        let mut json: Value = serde_json::from_str(source).unwrap();
        let original = json.clone();
        assert!(!Ingredient::resolve(&mut json, &load));
        assert_eq!(json, original);
    }
}

#[test]
fn recipe_book_and_placement_use_component_ingredient_item_lists() {
    use crate::recipe_book::{Book, Category, Display, Entry};
    let items = Items::empty();
    let ingredient = parse(
        r#"{"type":"neoforge:components","items":"minecraft:diamond_sword","components":{"minecraft:damage":5}}"#,
        &items,
    );
    let entry = Entry {
        id: "fixture:components".into(),
        display_id: -1,
        category: Category::CraftingEquipment,
        group: None,
        display: Display::Shapeless(vec![ingredient.clone()]),
        result: ItemStack::new("minecraft:diamond", 1),
        variants: Vec::new(),
        stacks: Vec::new(),
    };
    let mut book = Book::default();
    let placement = crate::crafting::Placement::of(vec![Some(ingredient)], None);
    book.placements.insert(entry.id.clone(), placement.clone());
    let mut stock = crate::menu::place::Stacked::default();
    // An undamaged sword cannot satisfy the concrete recipe, but the book and
    // placement intentionally account only for its item holder.
    stock.account(&std::sync::Arc::from("minecraft:diamond_sword"), 1);
    assert!(book.can_craft(&entry, &stock));
    assert!(stock.can_craft(&placement.ingredients.iter().collect::<Vec<_>>()));
    assert_eq!(
        book.items_of(&placement.ingredients[0]),
        ["minecraft:diamond_sword"]
    );
}

#[test]
fn potion_duration_scale_decodes_and_matches_float_values() {
    const ID: &str = "minecraft:potion_duration_scale";
    let mut items = Items::empty();
    items.component_defaults = std::sync::Arc::new(std::collections::HashMap::from([(
        "minecraft:potion".into(),
        horizon_core::nbt::Compound::from_iter([(ID.into(), Tag::Double(1.5f64.to_bits()))]),
    )]));
    let load = load();
    let context = load.loop_context(LoopSide::Server, []);
    let positive = parse(
        r#"{"type":"neoforge:components","items":"minecraft:potion","components":{"minecraft:potion_duration_scale":1.5}}"#,
        &items,
    );
    let mut stack = ItemStack::new("minecraft:potion", 1);
    assert!(positive.test_in(&context, &stack), "borrowed item default");
    for value in [
        Tag::Float(1.5f32.to_bits()),
        Tag::Double(1.50000000001f64.to_bits()),
    ] {
        stack.data_mut().set_loot_component(ID, &Some(value));
        assert!(positive.test_in(&context, &stack), "floatValue rounding");
    }
    stack
        .data_mut()
        .set_loot_component(ID, &Some(Tag::Float(1.25f32.to_bits())));
    assert!(!positive.test_in(&context, &stack));

    let one = parse(
        r#"{"type":"neoforge:components","items":"minecraft:potion","components":{"minecraft:potion_duration_scale":1}}"#,
        &items,
    );
    for value in [
        Tag::Byte(1),
        Tag::Short(1),
        Tag::Int(1),
        Tag::Long(1),
        Tag::Float(1.0f32.to_bits()),
        Tag::Double(1.0f64.to_bits()),
    ] {
        assert_eq!(
            horizon_command::item::decoded_component_value(ID, &value, &items.component_registry)
                .unwrap(),
            Tag::Float(0x3f80_0000),
        );
        stack.data_mut().set_loot_component(ID, &Some(value));
        assert!(one.test_in(&context, &stack));
    }
    let large = parse(
        r#"{"type":"neoforge:components","items":"minecraft:potion","components":{"minecraft:potion_duration_scale":4294967296}}"#,
        &items,
    );
    stack
        .data_mut()
        .set_loot_component(ID, &Some(Tag::Long(4_294_967_296)));
    assert!(
        large.test_in(&context, &stack),
        "boxed Long does not truncate"
    );

    let zero = parse(
        r#"{"type":"neoforge:components","items":"minecraft:potion","components":{"minecraft:potion_duration_scale":0}}"#,
        &items,
    );
    // JsonOps converts the integral decimal -0.0 to a zero byte before decoding.
    let json_zero = parse(
        r#"{"type":"neoforge:components","items":"minecraft:potion","components":{"minecraft:potion_duration_scale":-0.0}}"#,
        &items,
    );
    for value in [Tag::Byte(0), Tag::Double(f64::MIN_POSITIVE.to_bits())] {
        assert_eq!(
            horizon_command::item::decoded_component_value(ID, &value, &items.component_registry)
                .unwrap(),
            Tag::Float(0),
        );
        stack.data_mut().set_loot_component(ID, &Some(value));
        assert!(zero.test_in(&context, &stack));
        assert!(json_zero.test_in(&context, &stack));
    }
    stack
        .data_mut()
        .set_loot_component(ID, &Some(Tag::Float((-0.0f32).to_bits())));
    assert!(
        !zero.test_in(&context, &stack),
        "Float.equals distinguishes zero signs"
    );

    for source in [
        r#"{"type":"neoforge:components","items":"minecraft:potion","components":{"minecraft:potion_duration_scale":-1}}"#,
        r#"{"type":"neoforge:components","items":"minecraft:potion","components":{"minecraft:potion_duration_scale":"fast"}}"#,
        r#"{"type":"neoforge:components","items":"minecraft:potion","components":{"minecraft:potion_duration_scale":{}}}"#,
    ] {
        let json: Value = serde_json::from_str(source).unwrap();
        assert!(Ingredient::parse(&json, &items).is_none(), "{source}");
        let rule = IngredientRule::Components {
            patch: crate::json_nbt::json_to_compound(json["components"].as_object().unwrap())
                .unwrap(),
            removed: Vec::new(),
            strict: false,
        };
        let error = components::validate(&rule, &items).unwrap_err();
        assert!(error.contains(ID), "{error}");
    }
    for value in [
        Tag::Float((-0.0f32).to_bits()),
        Tag::Double((-0.0f64).to_bits()),
        Tag::Double((-f64::MIN_POSITIVE).to_bits()),
        Tag::Float(f32::NAN.to_bits()),
        Tag::Float(0x7f80_0001),
        Tag::Double(f64::NAN.to_bits()),
        Tag::Float(f32::INFINITY.to_bits()),
        Tag::Float(f32::NEG_INFINITY.to_bits()),
        Tag::Double(f64::MAX.to_bits()),
    ] {
        let rule = IngredientRule::Components {
            patch: horizon_core::nbt::Compound::from_iter([(ID.into(), value)]),
            removed: Vec::new(),
            strict: false,
        };
        let error = components::validate(&rule, &items).unwrap_err();
        assert!(error.contains(ID), "{error}");
    }
    assert_eq!(
        horizon_command::item::decoded_component_value(
            ID,
            &Tag::Float(f32::MAX.to_bits()),
            &items.component_registry,
        )
        .unwrap(),
        Tag::Float(0x7f7f_ffff),
    );
}
