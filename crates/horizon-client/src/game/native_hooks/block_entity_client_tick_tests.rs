#![cfg(test)]
//! `block_entity_client_tick` (#3611): a client's copies of a type that declared client ticks
//! are told once per client tick each, in the ticker list's order, with the data last merged; a
//! type that didn't declare them is never told; a copy whose column the server forgets is not
//! told again; and a server, the world's own ticks, never tells any.
use crate::game::native_entity_refs::tests::{run_fresh_cases, with_game_at};
use crate::game::world_loading::WorldRequest;
use horizon_client::containers::BlockEntity;
use horizon_core::nbt::{Compound, Tag};
use mod_api::{
    BlockEntityClientTickEvent, BlockEntityClientTicksEvent, BlockEntityPart, BlockPart,
    BlocksEvent, ClientHost, Hooks, Registrar, RegistrationContext,
};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const HUM: &str = "client_tick_probe:hum";
const QUIET: &str = "client_tick_probe:quiet";
/// Chunk (100, 100): far from the spawn.
const CHUNK: (i32, i32) = (100, 100);
const HUMS: [[i32; 3]; 2] = [[1610, 70, 1613], [1601, 64, 1602]];
const QUIET_AT: [i32; 3] = [1605, 64, 1605];
/// A hum in the next column, which stays.
const OTHER: [i32; 3] = [1620, 64, 1600];

/// What each call saw: the position, the position its handle reads (a live copy's), its
/// entity's id, its block state's block and its `rpm`.
static CALLS: Mutex<Vec<([i32; 3], [i32; 3], String, String, i32)>> = Mutex::new(Vec::new());

fn record(host: &dyn ClientHost, event: &BlockEntityClientTickEvent<'_>) {
    let rpm = match event.entity.nbt.get(&"rpm".into()) {
        Some(Tag::Int(rpm)) => *rpm,
        _ => -1,
    };
    let block = event.state.block().id(host);
    CALLS.lock().unwrap().push((
        event.pos,
        event.block_entity.position(host).unwrap_or([0; 3]),
        event.entity.id().to_owned(),
        block,
        rpm,
    ));
}

/// Declares client ticks.
struct Hum;
#[mod_api::horizon_part]
impl BlockPart for Hum {}
#[mod_api::horizon_part]
impl BlockEntityPart for Hum {
    fn block_entity_client_ticks(
        &self,
        _: &dyn ClientHost,
        event: &mut BlockEntityClientTicksEvent<'_>,
    ) {
        event.ticks = true;
    }
    fn block_entity_client_tick(
        &self,
        host: &mut dyn ClientHost,
        event: &mut BlockEntityClientTickEvent<'_>,
    ) {
        record(host, event);
    }
}

/// Has a client tick but never declares one.
struct Quiet;
#[mod_api::horizon_part]
impl BlockPart for Quiet {}
#[mod_api::horizon_part]
impl BlockEntityPart for Quiet {
    fn block_entity_client_tick(
        &self,
        host: &mut dyn ClientHost,
        event: &mut BlockEntityClientTickEvent<'_>,
    ) {
        record(host, event);
    }
}

struct Probe;
#[mod_api::__subscriptions]
impl Hooks for Probe {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        let hum = r.block_entity_type("hum").blocks(&[HUM]).build();
        r.block_entity_part(&hum, Hum).unwrap();
        let quiet = r.block_entity_type("quiet").blocks(&[QUIET]).build();
        r.block_entity_part(&quiet, Quiet).unwrap();
    }
    fn blocks(&self, _: &RegistrationContext, event: &mut BlocksEvent<'_>) {
        for block in [HUM, QUIET] {
            event
                .blocks
                .push(mod_api::Block::new(block, "minecraft:stone"));
        }
    }
}

fn install() {
    let mods: mod_api::Mods =
        vec![mod_api::ModEntry::new("client_tick_probe", Box::new(Probe))].into();
    mod_platform::__private::install(mods.clone());
    crate::game::api_values::install_parts(&mods).unwrap();
}

/// The prepared resources with the installed mods' content.
fn prepared_root() -> PathBuf {
    let base = PathBuf::from(std::env::var_os("HORIZON_TEST_RESOURCES").unwrap());
    crate::game::mods::content_root(&base, &Default::default())
        .unwrap()
        .prepared()
        .to_path_buf()
}

fn rpm(rpm: i32) -> Compound {
    let mut nbt = Compound::new();
    nbt.insert("rpm".into(), Tag::Int(rpm));
    nbt
}

fn take_calls() -> Vec<([i32; 3], [i32; 3], String, String, i32)> {
    std::mem::take(&mut *CALLS.lock().unwrap())
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_client_ticks_its_declared_copies_until_their_column_goes() {
    use crate::game::execution_role::ExecutionRole;
    use horizon_client::{
        chunk::Chunk,
        containers::{ModBlockEntity, ModBlockEntityFields},
    };
    use horizon_protocol::connection::Pipe;
    if run_fresh_cases(
        "game::native_hooks::block_entity_client_tick_tests::a_client_ticks_its_declared_copies_until_their_column_goes",
        "HORIZON_BLOCK_ENTITY_CLIENT_TICK_CHILD",
        &["game"],
    ) {
        return;
    }
    install();
    with_game_at(prepared_root(), |game| {
        game.options.execution_role = ExecutionRole::Client;
        game.options.dedicated = None;

        // Not connected yet: no client level, nothing told.
        game.client_tick_mod_block_entities();
        assert!(take_calls().is_empty(), "told without a client level");

        let (client, _server) = Pipe::pair();
        game.connect_local(client, "ClientTickProbe").unwrap();
        game.remote.as_mut().unwrap().logged_in = true;
        for (x, z) in [CHUNK, (101, 100)] {
            game.streaming
                .insert_remote(&mut game.world, Chunk::empty(x, z));
        }
        let registry = game.pack.assets.registry;
        let hum = registry.default_state(HUM).unwrap();
        let quiet = registry.default_state(QUIET).unwrap();
        // Arrival order: the second hum, the quiet one, the first hum, the other column's.
        let placed = [
            (HUMS[1], hum, HUM, 10),
            (QUIET_AT, quiet, QUIET, 20),
            (HUMS[0], hum, HUM, 30),
            (OTHER, hum, HUM, 40),
        ];
        for ([x, y, z], state, id, speed) in placed {
            game.world.set(x, y, z, state).unwrap();
            let entity = ModBlockEntity::new(
                id.into(),
                ModBlockEntityFields {
                    nbt: rpm(speed),
                    ..Default::default()
                },
            );
            game.world
                .block_entities
                .insert((x, y, z), BlockEntity::Mod(Box::new(entity)));
        }

        let hums = |rpms: [i32; 3]| {
            [HUMS[1], HUMS[0], OTHER]
                .into_iter()
                .zip(rpms)
                .map(|(pos, rpm)| (pos, pos, HUM.to_owned(), HUM.to_owned(), rpm))
                .collect::<Vec<_>>()
        };
        for _ in 0..3 {
            game.client_tick_mod_block_entities();
            assert_eq!(
                take_calls(),
                hums([10, 30, 40]),
                "once per tick each, in arrival order, the quiet type never"
            );
        }

        // An update merged into a copy reaches its next tick.
        let [x, y, z] = HUMS[0];
        let Some(BlockEntity::Mod(entity)) = game.world.block_entities.get_mut(&(x, y, z)) else {
            panic!("no hum at {x} {y} {z}");
        };
        entity.merge_update_tag(&rpm(31));
        game.client_tick_mod_block_entities();
        assert_eq!(take_calls(), hums([10, 31, 40]));

        // A copy removed is not told again; its column's other hum still is.
        let [x, y, z] = HUMS[1];
        game.world.block_entities.remove(&(x, y, z));
        game.client_tick_mod_block_entities();
        assert_eq!(
            take_calls(),
            vec![
                (HUMS[0], HUMS[0], HUM.to_owned(), HUM.to_owned(), 31),
                (OTHER, OTHER, HUM.to_owned(), HUM.to_owned(), 40)
            ]
        );

        // The server forgets the column: its hums are not told again, the other column's is.
        game.forget_remote_columns(&mut [CHUNK].into_iter().collect(), true);
        for _ in 0..2 {
            game.client_tick_mod_block_entities();
            assert_eq!(
                take_calls(),
                vec![(OTHER, OTHER, HUM.to_owned(), HUM.to_owned(), 40)]
            );
        }

        // The level going leaves nothing to tell.
        game.forget_all_remote_columns();
        game.client_tick_mod_block_entities();
        assert!(take_calls().is_empty(), "told after the level went");
    });
}

fn request(world: &Path) -> WorldRequest {
    let rules = std::sync::Arc::new(horizon_client::gamerules::RuleRegistry::vanilla());
    let options = horizon_client::title::WorldOptions {
        world_type: horizon_client::save::WorldPreset::Flat,
        rules: horizon_client::gamerules::GameRules::with_registry(rules.clone(), &[]),
        ..Default::default()
    };
    let mut request = WorldRequest::new(
        "client_tick",
        Some("client_tick"),
        Some(1),
        true,
        Some(&options),
        rules,
    )
    .unwrap();
    request.save_root = world.to_owned();
    request
}

/// A world's own ticks (a server's) never tell a client tick, a declared type's included.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_server_never_ticks_a_client_ticker() {
    if run_fresh_cases(
        "game::native_hooks::block_entity_client_tick_tests::a_server_never_ticks_a_client_ticker",
        "HORIZON_BLOCK_ENTITY_CLIENT_TICK_SERVER_CHILD",
        &["game"],
    ) {
        return;
    }
    install();
    let saves = std::env::current_dir()
        .unwrap()
        .join("target")
        .join(format!("block-entity-client-tick-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&saves);
    with_game_at(prepared_root(), |game| {
        game.load_world(request(&saves)).unwrap();
        game.set_allow_commands(true);
        assert!(game.run_command("forceload add 1600 1600"));
        let deadline = Instant::now() + Duration::from_secs(120);
        while !game.world.chunks.contains_key(&CHUNK) || game.streaming.busy() {
            assert!(Instant::now() < deadline, "chunk {CHUNK:?} did not load");
            game.streaming.wait(Duration::from_millis(5));
            game.update_world();
        }
        game.tick_world(true).unwrap();
        for ([x, y, z], block) in [(HUMS[0], HUM), (QUIET_AT, QUIET)] {
            assert!(game.run_command(&format!("setblock {x} {y} {z} {block}")));
        }
        for _ in 0..5 {
            game.tick_world(true).unwrap();
        }
        let [x, y, z] = HUMS[0];
        assert!(
            matches!(
                game.world.block_entities.get(&(x, y, z)),
                Some(BlockEntity::Mod(_))
            ),
            "the hum was placed"
        );
        assert!(take_calls().is_empty(), "a server told a client tick");
    });
    let _ = std::fs::remove_dir_all(&saves);
}
