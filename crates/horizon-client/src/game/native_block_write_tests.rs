use super::{Audio, Game, Options, Progress, Settings, load_resources};
use horizon_client::{
    chunk::Chunk,
    mod_api::{NativeAuthority, NativeBlockUpdateFlags},
};
use std::{sync::Arc, time::Instant};

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn level_block_writes_validate_and_change_world() {
    let root = std::path::PathBuf::from(std::env::var_os("HORIZON_TEST_RESOURCES").unwrap());
    let settings = Settings::default();
    let loaded = load_resources(
        root.clone(),
        None,
        &root,
        &settings,
        Instant::now(),
        false,
        true,
        &Progress::new(),
        &mut |_, _| unreachable!(),
    )
    .unwrap();
    let mut options = Options::parse(std::iter::empty()).unwrap().unwrap();
    options.resources = root.clone();
    options.execution_role = super::execution_role::ExecutionRole::IntegratedServer;
    options.dedicated = Some(Vec::new());
    let mut game = Game::new(
        &loaded.resources,
        loaded.pack,
        loaded.gui.unwrap(),
        options,
        settings,
        Audio::silent(&root),
        None,
    )
    .unwrap();
    game.world
        .chunks
        .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
    game.native_lifecycle
        .begin_open(NativeAuthority::IntegratedServer);
    let level = game.native_level("minecraft:overworld").unwrap();
    let stone = game
        .pack
        .assets
        .registry
        .default_state("minecraft:stone")
        .unwrap();
    let chest = game
        .pack
        .assets
        .registry
        .default_state("minecraft:chest")
        .unwrap();
    let air = game
        .pack
        .assets
        .registry
        .default_state("minecraft:air")
        .unwrap();
    let flags = NativeBlockUpdateFlags::NEIGHBORS;
    assert!(game.native_level_set_block(level, [0, 64, 0], stone, flags));
    assert_eq!(game.world.block(0, 64, 0), stone);
    assert!(!game.native_level_set_block(level, [16, 64, 0], stone, flags));
    assert!(!game.native_level_set_block(level, [0, 10000, 0], stone, flags));
    let stale = horizon_client::mod_api::LevelRef::new(
        horizon_client::mod_api::NativeWorldEpoch::new(level.epoch().get() + 1),
        level.dimension(),
    );
    assert!(!game.native_level_set_block(stale, [0, 64, 0], stone, flags));
    assert!(!game.native_level_destroy_block(stale, [0, 64, 0], true));
    assert!(game.native_level_set_block(level, [0, 64, 0], chest, flags));
    assert!(game.world.block_entities.contains_key(&(0, 64, 0)));
    assert!(game.native_level_set_block(
        level,
        [0, 64, 0],
        air,
        NativeBlockUpdateFlags::SKIP_BLOCK_ENTITY_SIDE_EFFECTS
    ));
    assert!(!game.world.block_entities.contains_key(&(0, 64, 0)));
    assert!(game.native_level_set_block(level, [0, 64, 0], stone, flags));
    assert!(game.native_level_destroy_block(level, [0, 64, 0], false));
    assert_eq!(game.world.block(0, 64, 0), air);
    assert!(!game.native_level_destroy_block(level, [0, 64, 0], true));
    assert!(game.enter_command_level(super::Dim::Nether).is_some());
    assert!(!game.native_level_set_block(level, [0, 64, 0], stone, flags));
    assert!(!game.native_level_destroy_block(level, [0, 64, 0], true));
}
