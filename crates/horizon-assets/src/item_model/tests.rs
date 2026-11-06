use super::*;

#[test]
fn identifier_adds_default_namespace() {
    assert_eq!(identifier("stone").unwrap(), "minecraft:stone");
    assert_eq!(identifier("a:b").unwrap(), "a:b");
}

#[test]
fn positive_modulo_wraps_negatives() {
    assert!((positive_modulo(-0.25, 1.0) - 0.75).abs() < 1e-6);
    assert!((positive_modulo(1.5, 1.0) - 0.5).abs() < 1e-6);
}

#[test]
fn display_context_names_round_trip() {
    for name in ["gui", "ground", "fixed", "head"] {
        assert_eq!(DisplayContext::by_name(name).unwrap().name(), name);
    }
}

#[test]
fn custom_condition_is_evaluated_per_stack_at_branch_selection() {
    struct Env;
    impl ItemEnv for Env {
        fn item_model_property(&self, property: &str, stack: &StackView) -> bool {
            assert_eq!(property, "example:charged");
            stack.count > 1 && stack.patch.get("example:charged") == Some(&Some(Value::Bool(true)))
        }
    }
    let definition = ClientItem::from_json(
        r#"{"model":{
        "type":"minecraft:condition", "property":"example:charged",
        "on_true":{"type":"minecraft:model","model":"example:item/charged"},
        "on_false":{"type":"minecraft:model","model":"example:item/idle"}
    }}"#,
    )
    .unwrap();
    let mut models = ItemModels::default();
    models.definitions.insert("example:tool".into(), definition);
    Arc::make_mut(&mut models.prototypes).insert(
        "example:tool".into(),
        BTreeMap::from([(
            "minecraft:item_model".into(),
            Value::String("example:tool".into()),
        )]),
    );
    let context = Context {
        display: DisplayContext::Gui,
        owner: None,
        level: None,
        seed: 0,
    };
    let mut stack = StackView::new("example:tool", 1);
    let chosen = |stack: &StackView, env: &dyn ItemEnv| {
        let resolved = models.resolve(stack, &context, env);
        assert_eq!(resolved.layers.len(), 1);
        let LayerContent::Model { model } = &resolved.layers[0].content else {
            panic!("expected model leaf")
        };
        model.clone()
    };
    assert_eq!(chosen(&stack, &NoEnv), "example:item/idle");
    assert_eq!(chosen(&stack, &Env), "example:item/idle");
    stack
        .patch
        .insert("example:charged".into(), Some(Value::Bool(true)));
    assert_eq!(chosen(&stack, &Env), "example:item/idle");
    stack.count = 2;
    assert_eq!(chosen(&stack, &Env), "example:item/charged");
    assert_eq!(chosen(&stack, &NoEnv), "example:item/idle");
    stack.patch.insert("example:charged".into(), None);
    assert_eq!(chosen(&stack, &Env), "example:item/idle");
}

#[test]
fn custom_condition_validates_its_name_and_both_branches() {
    let node = |property: &str, on_true: Value| {
        ItemModelNode::decode(&serde_json::json!({
            "type": "minecraft:condition", "property": property,
            "on_true": on_true, "on_false": {"type": "minecraft:empty"}
        }))
    };
    let empty = serde_json::json!({"type": "minecraft:empty"});
    for name in [
        "minecraft:unknown",
        "unknown",
        "example:Bad",
        "bad name:charged",
    ] {
        assert!(node(name, empty.clone()).is_err(), "{name}");
    }
    assert!(node("example:charged", Value::Null).is_err());
    assert!(matches!(
        node("minecraft:damaged", empty.clone()).unwrap(),
        ItemModelNode::Condition {
            property: ConditionProperty::Damaged,
            ..
        }
    ));
    assert!(matches!(node("example:charged", empty).unwrap(),
        ItemModelNode::Condition { property: ConditionProperty::Custom(name), .. } if name == "example:charged"));
}

#[test]
fn a_mods_tint_source_is_untinted_and_an_unknown_vanilla_one_fails() {
    let tint = |json: &str| TintSource::decode(&serde_json::from_str(json).unwrap());
    assert_eq!(
        tint(r#"{"type":"ae2:storage_cell_state"}"#).unwrap(),
        TintSource::Constant(-1)
    );
    assert!(tint(r#"{"type":"minecraft:nonsense"}"#).is_err());
}

#[test]
fn registered_range_property_thresholds_on_the_mods_number_per_stack() {
    struct Env;
    impl ItemEnv for Env {
        fn item_model_range_property(&self, property: &str, stack: &StackView) -> f32 {
            assert_eq!(property, "rangetest:fill");
            stack.count as f32 / 10.0
        }
    }
    let kept = register_range_properties(vec![
        "rangetest:fill".into(),
        "minecraft:damage".into(),
        "unnamespaced".into(),
        "rangetest:Bad".into(),
    ]);
    assert_eq!(kept, ["rangetest:fill"]);
    assert!(register_range_properties(vec!["rangetest:fill".into()]).is_empty());
    let json = r#"{"model":{
        "type":"minecraft:range_dispatch", "property":"rangetest:fill", "scale":2.0,
        "entries":[
            {"threshold":1.0,"model":{"type":"minecraft:model","model":"rangetest:item/full"}},
            {"threshold":0.5,"model":{"type":"minecraft:model","model":"rangetest:item/half"}}
        ],
        "fallback":{"type":"minecraft:model","model":"rangetest:item/empty"}
    }}"#;
    let context = Context {
        display: DisplayContext::Gui,
        owner: None,
        level: None,
        seed: 0,
    };
    // A resource reload decodes the definition again; the property stays the mod's.
    for _reload in 0..2 {
        let mut models = ItemModels::default();
        models.definitions.insert(
            "rangetest:cell".into(),
            ClientItem::from_json(json).unwrap(),
        );
        Arc::make_mut(&mut models.prototypes).insert(
            "rangetest:cell".into(),
            BTreeMap::from([(
                "minecraft:item_model".into(),
                Value::String("rangetest:cell".into()),
            )]),
        );
        let chosen = |count: i32, env: &dyn ItemEnv| {
            let resolved = models.resolve(&StackView::new("rangetest:cell", count), &context, env);
            let LayerContent::Model { model } = &resolved.layers[0].content else {
                panic!("expected model leaf")
            };
            model.clone()
        };
        // Scaled by 2: 0.2 -> 0.4, 0.3 -> 0.6, 0.5 -> 1.0.
        assert_eq!(chosen(2, &Env), "rangetest:item/empty");
        assert_eq!(chosen(3, &Env), "rangetest:item/half");
        assert_eq!(chosen(5, &Env), "rangetest:item/full");
        assert_eq!(chosen(5, &NoEnv), "rangetest:item/empty");
    }
}
