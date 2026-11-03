//! Isolated process: the load callbacks and custom-component IDs install once.
use glam::Mat4;
use horizon_assets::item_model::{DisplayContext, NoEnv, StackView};
use horizon_client_lib::{
    self as client,
    hud::Rect,
    item_render::{ItemRenderer, NoHost},
};
use horizon_world::{
    loot::{LootData, PackOverlay},
    native_load_hooks::{self, NativeLoadHooks},
};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    sync::atomic::{AtomicUsize, Ordering},
};

static CALLS: AtomicUsize = AtomicUsize::new(0);

/// The before-snapshot baseline includes the producer, gameplay/enchantment
/// metadata, both render loaders, banners, loot and the content handshake.
#[test]
fn default_components_reader_baseline() -> client::Result<()> {
    let root = std::env::temp_dir().join(format!("horizon-default-reader-{}", std::process::id()));
    for directory in [
        "assets/probe/items",
        "assets/probe/models/item",
        "assets/probe/textures/item",
        "data/probe",
    ] {
        fs::create_dir_all(root.join(directory))?;
    }
    for (path, contents) in [
        (
            "items.json",
            r#"{"probe:token":{"minecraft:max_stack_size":16,"minecraft:item_model":"probe:token","probe:marker":"kept"}}"#,
        ),
        ("facts.json", r#"{"states":[]}"#),
        (
            "registries.json",
            r#"{"minecraft:item":{"entries":{"probe:token":{"protocol_id":0}}},"minecraft:data_component_type":{"entries":{}}}"#,
        ),
        (
            "assets/probe/items/token.json",
            r#"{"model":{"type":"minecraft:model","model":"probe:item/token","tints":[{"type":"minecraft:constant","value":16711680}]}}"#,
        ),
        (
            "assets/probe/models/item/token.json",
            r##"{"textures":{"particle":"probe:item/token","all":"probe:item/token"},"elements":[{"from":[0,0,0],"to":[16,16,16],"faces":{"north":{"texture":"#all","tintindex":0},"south":{"texture":"#all","tintindex":0},"east":{"texture":"#all","tintindex":0},"west":{"texture":"#all","tintindex":0},"up":{"texture":"#all","tintindex":0},"down":{"texture":"#all","tintindex":0}}}]}"##,
        ),
    ] {
        fs::write(root.join(path), contents)?;
    }
    // An original one-pixel white texture, not a Minecraft asset.
    fs::write(
        root.join("assets/probe/textures/item/token.png"),
        [
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
            8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 11, 73, 68, 65, 84, 120, 156, 99, 248, 15, 4,
            0, 9, 251, 3, 253, 251, 94, 107, 43, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
        ],
    )?;
    assert!(
        native_load_hooks::install(NativeLoadHooks {
            loot_table_load: |_, _| {},
            modify_default_components: |item, _| {
                if item == "probe:token" {
                    CALLS.fetch_add(1, Ordering::Relaxed);
                }
            },
            loot_function: |_, _, _, _| {},
            loot_condition: |_, _, _, _| {},
        })
        .is_ok()
    );
    horizon_protocol::item::custom::install(0, vec!["probe:marker".into()]).unwrap();

    let defaults = horizon_world::item_defaults::read(&root)?;
    let items = horizon_items::items::Items::catalog(&root, &PackOverlay::NONE)?;
    assert_eq!(items.get("probe:token").unwrap().max_stack, 16);
    let library = client::model_library::ParsedModels::load(&root).into_item_library();
    let client = ItemRenderer::load_with_library(&root, library.clone())?;
    let server = ItemRenderer::load_for_server(&root, library)?;
    let _banners = client::banner::Banners::load(&root);
    let _loot = LootData::load(&root, &PackOverlay::NONE)?;
    let identity = client::content_identity::identity(&root)?;
    assert_eq!(CALLS.load(Ordering::Relaxed), 8);
    assert_eq!(client.models.prototypes, server.models.prototypes);
    assert_eq!(
        client.models.prototypes["probe:token"]["minecraft:max_stack_size"],
        defaults["probe:token"]["minecraft:max_stack_size"]
    );

    let state = client.resolve(
        &StackView::new("probe:token", 1),
        DisplayContext::Gui,
        None,
        None,
        0,
        &NoEnv,
    );
    let sprites = HashMap::from([(
        "probe:item/token".to_owned(),
        Rect {
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
            w: 1,
            h: 1,
        },
    )]);
    let mut quads = Vec::new();
    state.submit(&client, &sprites, &NoHost, Mat4::IDENTITY, &mut quads);
    assert_eq!(
        quads.len(),
        6,
        "parity must compare actual submitted geometry"
    );
    println!("baseline calls={}", CALLS.load(Ordering::Relaxed));
    println!(
        "baseline prototypes_sha256={:x}",
        Sha256::digest(serde_json::to_vec(&client.models.prototypes)?)
    );
    println!(
        "baseline rendered_sha256={:x}",
        Sha256::digest(format!("{quads:?}"))
    );
    println!(
        "baseline identity={}",
        identity
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    fs::remove_dir_all(&root)?;
    Ok(())
}
