use crate::game::commands::items::Slot;
use crate::game::{Audio, Game, Options, Progress, Settings, load_resources};

fn with_game(check: impl FnOnce(&mut Game<'_>)) {
    let root = std::path::PathBuf::from(std::env::var_os("HORIZON_TEST_RESOURCES").unwrap());
    let scratch = tempfile::tempdir().unwrap();
    let settings = Settings::default();
    let loaded = load_resources(
        horizon_assets::resource_source::ResourceRoot::directory(root.clone()),
        Vec::new(),
        None,
        &root,
        &settings,
        std::time::Instant::now(),
        false,
        crate::game::execution_role::ExecutionRole::IntegratedServer,
        &Progress::new(),
        &mut |_, _| unreachable!(),
    )
    .unwrap();
    let mut options = Options::parse(std::iter::empty()).unwrap().unwrap();
    options.resources = root.clone();
    options.game_dir = Some(scratch.path().to_path_buf());
    options.execution_role = crate::game::execution_role::ExecutionRole::IntegratedServer;
    let mut game = Game::new(
        &loaded.resources,
        loaded.pack,
        loaded.gui.unwrap(),
        options,
        settings,
        Audio::silent(&horizon_assets::resource_source::ResourceSource::directory(
            root.clone(),
        )),
        None,
    )
    .unwrap();
    game.build_registries();

    game.set_allow_commands(true);
    game.world.chunks.insert(
        (0, 0),
        std::sync::Arc::new(horizon_client::chunk::Chunk::empty(0, 0)),
    );
    game.block_ticks
        .add_empty_chunk(horizon_core::position::ChunkPos::new(0, 0))
        .unwrap();
    assert!(game.run_command("forceload add 0 0"));
    check(&mut game);
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn item_replace_jukebox_accepts_music_disc_cat() {
    with_game(|game| {
        assert!(game.run_command("setblock 4 200 4 jukebox"));
        assert!(
            game.run_command(
                "item replace block 4 200 4 container.0 with minecraft:music_disc_cat"
            )
        );
        assert_eq!(
            game.slot_get(&Slot::Block([4, 200, 4], 0))
                .map(|s| (s.item.to_string(), s.count)),
            Some(("minecraft:music_disc_cat".into(), 1))
        );
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn loot_empty_replace_jukebox_preserves_music_disc_13() {
    with_game(|game| {
        assert!(game.run_command("setblock 4 200 4 jukebox"));
        assert!(
            game.run_command("item replace block 4 200 4 container.0 with minecraft:music_disc_13")
        );
        assert_eq!(
            game.slot_get(&Slot::Block([4, 200, 4], 0))
                .unwrap()
                .item
                .as_ref(),
            "minecraft:music_disc_13"
        );
        assert!(game.run_command(r#"loot replace block 4 200 4 container.0 1 loot {"pools":[]}"#));
        assert_eq!(
            game.slot_get(&Slot::Block([4, 200, 4], 0))
                .map(|s| (s.item.to_string(), s.count)),
            Some(("minecraft:music_disc_13".into(), 1))
        );
    });
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn loot_replace_furnace_fuel_refuses_non_fuel() {
    with_game(|game| {
        assert!(game.run_command("setblock 4 200 4 furnace"));
        assert!(game.run_command("item replace block 4 200 4 container.1 with minecraft:coal 3"));
        assert!(game.run_command(r#"loot replace block 4 200 4 container.1 1 loot {"pools":[{"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]}]}"#));
        assert_eq!(
            game.slot_get(&Slot::Block([4, 200, 4], 1))
                .map(|s| (s.item.to_string(), s.count)),
            Some(("minecraft:coal".into(), 3))
        );
    });
}
