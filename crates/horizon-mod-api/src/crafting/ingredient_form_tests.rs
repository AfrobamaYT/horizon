use super::*;
use horizon_core::nbt::Tag;

#[test]
fn flat_ingredient_codec_keeps_shapes_order_and_typed_metadata() {
    let mut custom = Ingredient::of(vec!["sample:item".into()]);
    custom.custom = Some((
        "sample:kind".into(),
        [(
            "data".into(),
            Tag::compound([("count", Tag::Long(2147483648))]),
        )]
        .into(),
    ));
    let mut root = Ingredient::of(vec!["sample:item".into(), "sample:other".into()]);
    root.children = Some((
        Shape::Difference,
        vec![custom, Ingredient::of(vec!["sample:other".into()])],
    ));
    let form = IngredientForm::from(&root);
    assert_eq!(form.nodes.len(), 3);
    let restored = Ingredient::try_from(form).unwrap();
    assert_eq!(restored.order, ["sample:item", "sample:other"]);
    let (shape, children) = restored.children.unwrap();
    assert!(matches!(shape, Shape::Difference));
    assert_eq!(children[0].order, ["sample:item"]);
    assert_eq!(children[1].order, ["sample:other"]);
    assert_eq!(
        children[0].custom.as_ref().unwrap().1.get(&"data".into()),
        Some(&Tag::compound([("count", Tag::Long(2147483648))]))
    );
}

#[test]
fn flat_ingredient_codec_refuses_non_trees() {
    let root = Ingredient::of(vec!["sample:item".into()]);
    let mut empty = IngredientForm::from(&root);
    empty.nodes.clear();
    assert!(Ingredient::try_from(empty).is_err());
    for indices in [vec![0], vec![999], vec![1, 1]] {
        let mut form = IngredientForm::from(&root);
        form.nodes[0].children = Some((Shape::Compound, indices));
        form.nodes.push(IngredientForm::from(&root).nodes.remove(0));
        assert!(Ingredient::try_from(form).is_err());
    }
    let mut orphaned = IngredientForm::from(&root);
    orphaned
        .nodes
        .push(IngredientForm::from(&root).nodes.remove(0));
    assert!(Ingredient::try_from(orphaned).is_err());
}

#[test]
fn flat_ingredient_codec_depth_edge_is_128_nodes() {
    let leaf = Ingredient::of(vec!["sample:item".into()]);
    let mut form = IngredientForm::from(&leaf);
    for index in 1..128 {
        form.nodes[index - 1].children = Some((Shape::Compound, vec![index as u64]));
        form.nodes.push(IngredientForm::from(&leaf).nodes.remove(0));
    }
    let accepted = Ingredient::try_from(form.clone()).unwrap();
    let mut nodes = 1;
    let mut at = &accepted;
    while let Some((_, children)) = &at.children {
        nodes += 1;
        at = &children[0];
    }
    assert_eq!(nodes, 128);
    form.nodes[127].children = Some((Shape::Compound, vec![128]));
    form.nodes.push(IngredientForm::from(&leaf).nodes.remove(0));
    assert_eq!(
        Ingredient::try_from(form).unwrap_err(),
        IngredientFormError::TooDeep
    );
}

#[test]
fn former_ingredient_json_codec_budget_counts_wrappers_too() {
    // Independent model of the former derived wire record, not a runtime
    // fallback. The generated Kind::Json lift called serde_json::from_str.
    #[derive(Debug, serde::Deserialize)]
    struct FormerIngredient {
        #[serde(rename = "items")]
        _items: std::collections::HashSet<String>,
        #[serde(rename = "order")]
        _order: Vec<String>,
        #[serde(rename = "custom")]
        _custom: Option<(String, serde_json::Map<String, serde_json::Value>)>,
        #[serde(rename = "children")]
        _children: Option<(Shape, Vec<Self>)>,
    }
    let leaf = r#"{"items":[],"order":[],"custom":null,"children":null}"#;
    let mut wire = leaf.to_owned();
    for _ in 1..42 {
        wire =
            format!(r#"{{"items":[],"order":[],"custom":null,"children":["Compound",[{wire}]]}}"#);
    }
    assert!(serde_json::from_str::<FormerIngredient>(&wire).is_ok());
    wire = format!(r#"{{"items":[],"order":[],"custom":null,"children":["Compound",[{wire}]]}}"#);
    assert!(
        serde_json::from_str::<FormerIngredient>(&wire)
            .unwrap_err()
            .to_string()
            .contains("recursion limit exceeded")
    );
}
