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
    assert!(Ingredient::try_from(IngredientForm { nodes: vec![] }).is_err());
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
