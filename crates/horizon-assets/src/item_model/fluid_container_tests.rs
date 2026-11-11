use super::*;

fn node(json: &str) -> Result<ItemModelNode> {
    ItemModelNode::decode(&serde_json::from_str(json).unwrap())
}
fn container(json: &str) -> FluidContainer {
    match node(json).unwrap() {
        ItemModelNode::FluidContainer(container) => container,
        other => panic!("not a fluid container: {other:?}"),
    }
}

/// The tint this fluid's still sprite is multiplied by: `0xFF223344`.
const GAS_TINT: i32 = 0xFF22_3344u32 as i32;

/// What the host answers for a stack: the fluid it holds and, for `mod:gas` alone,
/// its still sprite and tint.
struct Fluids {
    held: Option<&'static str>,
}
impl ItemEnv for Fluids {
    fn stack_fluid(&self, _stack: &StackView) -> Option<String> {
        self.held.map(str::to_owned)
    }
    fn fluid_appearance(&self, fluid: &str) -> Option<FluidAppearance> {
        (fluid == "mod:gas").then(|| FluidAppearance {
            still: "mod:block/gas_still".into(),
            tint: GAS_TINT,
        })
    }
}

fn layers(definition: &str, env: &dyn ItemEnv) -> Vec<Layer> {
    let mut models = ItemModels::default();
    models.definitions.insert(
        "mod:flask".into(),
        ClientItem::from_json(&format!(r#"{{"model":{definition}}}"#)).unwrap(),
    );
    Arc::make_mut(&mut models.prototypes).insert(
        "mod:flask".into(),
        BTreeMap::from([(
            "minecraft:item_model".into(),
            Value::String("mod:flask".into()),
        )]),
    );
    let context = Context {
        display: DisplayContext::Gui,
        owner: None,
        level: None,
        seed: 0,
    };
    models
        .resolve(&StackView::new("mod:flask", 1), &context, env)
        .layers
}
fn sprite(sprite: &str, mask: Option<&str>) -> LayerContent {
    LayerContent::Sprite {
        sprite: sprite.into(),
        mask: mask.map(Into::into),
    }
}

#[test]
fn decodes_every_layer_and_the_declared_fluid() {
    let decoded = container(
        r#"{"type":"neoforge:fluid_container","fluid":"minecraft:water",
            "textures":{"particle":"mod:item/flask","base":"mod:item/flask",
                        "fluid":"neoforge:item/mask/bucket_fluid","cover":"mod:item/flask_cover"}}"#,
    );
    assert_eq!(decoded.fluid, "minecraft:water");
    assert!(decoded.cover_is_mask);
    assert_eq!(
        decoded.textures,
        FluidContainerTextures {
            base: Some("mod:item/flask".into()),
            fluid: Some("neoforge:item/mask/bucket_fluid".into()),
            cover: Some("mod:item/flask_cover".into()),
        }
    );
}

#[test]
fn optional_textures_and_cover_is_mask() {
    let decoded = container(
        r#"{"type":"neoforge:fluid_container","fluid":"minecraft:empty","cover_is_mask":false,
            "textures":{"base":"mod:item/jar"}}"#,
    );
    assert_eq!(
        decoded.textures,
        FluidContainerTextures {
            base: Some("mod:item/jar".into()),
            fluid: None,
            cover: None,
        }
    );
    assert!(!decoded.cover_is_mask);
    // `particle` alone satisfies the codec's at-least-one-texture rule.
    let decoded = container(
        r#"{"type":"neoforge:fluid_container","fluid":"minecraft:lava",
            "textures":{"particle":"mod:item/can"}}"#,
    );
    assert_eq!(decoded.textures.base, None);
    assert_eq!(decoded.textures.fluid, None);
    assert_eq!(decoded.textures.cover, None);
}

#[test]
fn bad_shapes_error_instead_of_guessing() {
    for json in [
        // textures is required, and must be an object.
        r#"{"type":"neoforge:fluid_container","fluid":"minecraft:water"}"#,
        r#"{"type":"neoforge:fluid_container","fluid":"minecraft:water","textures":"mod:item/jar"}"#,
        // ... with at least one of its four textures; a misspelt one is not one.
        r#"{"type":"neoforge:fluid_container","fluid":"minecraft:water","textures":{}}"#,
        r#"{"type":"neoforge:fluid_container","fluid":"minecraft:water","textures":{"bse":"mod:item/jar"}}"#,
        // fluid is required and is an identifier.
        r#"{"type":"neoforge:fluid_container","textures":{"base":"mod:item/jar"}}"#,
        r#"{"type":"neoforge:fluid_container","fluid":"Minecraft:Water","textures":{"base":"mod:item/jar"}}"#,
        // cover_is_mask is a boolean.
        r#"{"type":"neoforge:fluid_container","fluid":"minecraft:water","cover_is_mask":"yes","textures":{"base":"mod:item/jar"}}"#,
    ] {
        assert!(node(json).is_err(), "{json}");
    }
    // `flip_gas` and the other booleans the type does not draw with are accepted.
    assert!(
        node(
            r#"{"type":"neoforge:fluid_container","fluid":"minecraft:water","flip_gas":true,
                "apply_fluid_luminosity":false,"force_opaque_fluid":false,
                "textures":{"base":"mod:item/jar"}}"#
        )
        .is_ok()
    );
}

/// `DynamicFluidContainerModel.bakeModelForFluid` passes the cover as the mask and the base as
/// the drawn material (`bakeItemMaskQuads(baker, 0, cover, base, ...)`, offsets 631-639): the
/// cover picks out the parts of the base drawn again over the fluid.
#[test]
fn a_full_container_draws_base_fluid_and_base_through_the_cover_mask() {
    let drawn = layers(
        r#"{"type":"neoforge:fluid_container","fluid":"minecraft:empty",
            "textures":{"base":"mod:item/flask","fluid":"neoforge:item/mask/bucket_fluid",
                        "cover":"mod:item/flask_cover"}}"#,
        &Fluids {
            held: Some("mod:gas"),
        },
    );
    let contents: Vec<LayerContent> = drawn.iter().map(|l| l.content.clone()).collect();
    assert_eq!(
        contents,
        vec![
            sprite("mod:item/flask", None),
            sprite(
                "mod:block/gas_still",
                Some("neoforge:item/mask/bucket_fluid")
            ),
            sprite("mod:item/flask", Some("mod:item/flask_cover")),
        ]
    );
    assert_eq!(drawn[0].tints, Vec::<i32>::new());
    // The fluid's own tint applies to the fluid layer only.
    assert_eq!(drawn[1].tints, vec![GAS_TINT]);
    assert_eq!(drawn[2].tints, Vec::<i32>::new());
    assert!(drawn.iter().all(|l| l.foil == FoilType::None));
}

#[test]
fn an_empty_container_draws_its_declared_fluid_or_none() {
    let empty = r#"{"type":"neoforge:fluid_container","fluid":"minecraft:empty",
        "textures":{"base":"mod:item/flask","fluid":"neoforge:item/mask/bucket_fluid"}}"#;
    let drawn = layers(empty, &Fluids { held: None });
    let contents: Vec<LayerContent> = drawn.iter().map(|l| l.content.clone()).collect();
    assert_eq!(contents, vec![sprite("mod:item/flask", None)]);

    // NeoForge falls back to the model's `fluid`: a container with no contents still
    // draws the fluid it declares.
    let declared = r#"{"type":"neoforge:fluid_container","fluid":"mod:gas",
        "textures":{"base":"mod:item/flask","fluid":"neoforge:item/mask/bucket_fluid"}}"#;
    let drawn = layers(declared, &Fluids { held: None });
    let contents: Vec<LayerContent> = drawn.iter().map(|l| l.content.clone()).collect();
    assert_eq!(
        contents,
        vec![
            sprite("mod:item/flask", None),
            sprite(
                "mod:block/gas_still",
                Some("neoforge:item/mask/bucket_fluid")
            ),
        ]
    );
}

#[test]
fn an_unknown_fluid_or_no_mask_draws_no_fluid_layer() {
    let definition = r#"{"type":"neoforge:fluid_container","fluid":"mod:gas",
        "textures":{"base":"mod:item/flask","fluid":"neoforge:item/mask/bucket_fluid"}}"#;
    // The host knows the held fluid but not how it draws (NeoForge's null `still`).
    let drawn = layers(
        definition,
        &Fluids {
            held: Some("mod:unknown"),
        },
    );
    let contents: Vec<LayerContent> = drawn.iter().map(|l| l.content.clone()).collect();
    assert_eq!(contents, vec![sprite("mod:item/flask", None)]);

    // The type's `fluid` texture is the mask; without it there is no fluid layer.
    let drawn = layers(
        r#"{"type":"neoforge:fluid_container","fluid":"mod:gas",
            "textures":{"base":"mod:item/flask"}}"#,
        &Fluids {
            held: Some("mod:gas"),
        },
    );
    let contents: Vec<LayerContent> = drawn.iter().map(|l| l.content.clone()).collect();
    assert_eq!(contents, vec![sprite("mod:item/flask", None)]);
}

#[test]
fn a_cover_is_a_mask_only_over_a_base() {
    // Without `cover_is_mask` the cover is drawn through itself, with or without a base
    // (`coverIsMask ? base : cover` at offsets 598-615, the cover still the mask)...
    let drawn = layers(
        r#"{"type":"neoforge:fluid_container","fluid":"minecraft:empty","cover_is_mask":false,
            "textures":{"cover":"mod:item/flask_cover"}}"#,
        &Fluids { held: None },
    );
    let contents: Vec<LayerContent> = drawn.iter().map(|l| l.content.clone()).collect();
    assert_eq!(
        contents,
        vec![sprite("mod:item/flask_cover", Some("mod:item/flask_cover"))]
    );
    let drawn = layers(
        r#"{"type":"neoforge:fluid_container","fluid":"minecraft:empty","cover_is_mask":false,
            "textures":{"base":"mod:item/flask","cover":"mod:item/flask_cover"}}"#,
        &Fluids { held: None },
    );
    let contents: Vec<LayerContent> = drawn.iter().map(|l| l.content.clone()).collect();
    assert_eq!(
        contents,
        vec![
            sprite("mod:item/flask", None),
            sprite("mod:item/flask_cover", Some("mod:item/flask_cover")),
        ]
    );

    // ... while a mask cover needs the base it masks and is left out without one.
    let drawn = layers(
        r#"{"type":"neoforge:fluid_container","fluid":"minecraft:empty",
            "textures":{"cover":"mod:item/flask_cover"}}"#,
        &Fluids { held: None },
    );
    assert!(drawn.is_empty());
}
