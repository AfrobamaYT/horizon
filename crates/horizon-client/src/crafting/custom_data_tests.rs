use horizon_world::native_load_context::{DataLoadOrigin, LoadContext, LoadInputs};
use serde_json::{Value, json};

fn assert_custom_tags(data: &horizon_core::nbt::Compound) {
    use horizon_core::nbt::Tag;
    assert_eq!(data.get(&"integer".into()), Some(&Tag::Int(32768)));
    assert_eq!(data.get(&"long".into()), Some(&Tag::Long(2147483648)));
    assert_eq!(
        data.get(&"list".into()),
        Some(&Tag::List(vec![Tag::Byte(1), Tag::Short(128)]))
    );
    assert_eq!(
        data.get(&"nested".into()),
        Some(&Tag::compound([("enabled", Tag::Byte(1))]))
    );
}

struct TypedIngredient;
#[mod_api::__subscriptions]
impl mod_api::Hooks for TypedIngredient {
    fn ingredient(
        &self,
        _: &dyn mod_api::DataLoadContext,
        event: &mut mod_api::IngredientEvent<'_>,
    ) {
        assert_custom_tags(event.data);
        event.items.push("minecraft:emerald".into());
    }
}

#[test]
fn custom_ingredient_tags_reach_hook_and_matching_record() {
    let items = super::Items::for_tests(Default::default());
    mod_api::testing::with_mods(
        vec![mod_api::ModEntry::new("typed", Box::new(TypedIngredient))].into(),
        || {
            let context = LoadContext::new(
                DataLoadOrigin::ServerDataLoad,
                std::sync::Arc::new(LoadInputs::new(Default::default(), Default::default())),
                [],
            );
            let mut source = json!({"type": "typed:ingredient", "integer": 32768, "long": 2147483648i64,
            "list": [1, 128], "nested": {"enabled": true}});
            super::Ingredient::resolve(&mut source, &context);
            let parsed = super::Ingredient::parse(&source, &items).unwrap();
            let (_, data) = parsed.custom.unwrap();
            assert_custom_tags(&data);
            assert!(!data.contains_key(&"horizon:resolved_items".into()));

            // Refusal leaves the kind unanswered just as an unknown kind does;
            // no hook call should see the malformed metadata.
            source["nested"]["invalid"] = Value::Null;
            super::Ingredient::resolve(&mut source, &context);
            assert!(super::Ingredient::parse(&source, &items).is_none());
        },
    );
}

#[test]
fn ingredient_depth_128_matches_clones_and_drops_on_two_mib_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let load = LoadContext::new(
                DataLoadOrigin::ServerDataLoad,
                std::sync::Arc::new(LoadInputs::new(Default::default(), Default::default())),
                [],
            );
            let context =
                load.loop_context(horizon_world::native_load_context::LoopSide::Server, []);
            let mut ingredient = super::Ingredient::of(vec!["sample:item".into()]);
            for _ in 1..128 {
                let mut parent = super::Ingredient::of(vec!["sample:item".into()]);
                parent.children = Some((super::Shape::Compound, vec![ingredient]));
                ingredient = parent;
            }
            let stack = horizon_items::items::ItemStack {
                item: "sample:item".into(),
                count: 1,
                damage: 0,
                extra: None,
            };
            assert!(ingredient.test_in(&context, &stack));
            let cloned = ingredient.clone();
            assert!(cloned.test_in(&context, &stack));
            drop(cloned);
            drop(ingredient);
        })
        .unwrap()
        .join()
        .unwrap();
}
