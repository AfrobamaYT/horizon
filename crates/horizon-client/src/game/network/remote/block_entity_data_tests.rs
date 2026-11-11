//! End-to-end client receive notices with independent tag values.
use super::*;
use crate::game::native_entity_refs::tests::{run_fresh_cases, with_client_game};
use mod_api::{BlockEntityPart, BlockPart, Hooks, Registrar};
use std::sync::{Arc, Mutex};

struct Observer(Arc<Mutex<Vec<(i32, i32)>>>);
#[mod_api::horizon_part]
impl BlockPart for Observer {}
#[mod_api::horizon_part]
impl BlockEntityPart for Observer {
    fn block_entity_data_received(
        &self,
        host: &dyn mod_api::ClientHost,
        event: &mut mod_api::BlockEntityDataReceivedEvent<'_>,
    ) {
        let Some(Tag::Int(arrived)) = event.tag.get(&"count".into()) else {
            panic!("missing arrived count")
        };
        let data = event.entity.data(host).unwrap();
        let Some(Tag::Int(merged)) = data.get(&"count".into()) else {
            panic!("missing merged count")
        };
        self.0.lock().unwrap().push((*arrived, *merged));
    }
}
struct TestMod(Arc<Mutex<Vec<(i32, i32)>>>);
#[mod_api::__subscriptions]
impl Hooks for TestMod {
    fn register(&self, r: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
        let kind = r.block_entity_type("counter").blocks(&[]).build();
        r.block_entity_part(&kind, Observer(self.0.clone()))
            .unwrap();
        r.block_entity_type("silent").blocks(&[]).build();
    }
}
#[test]
#[ignore = "requires prepared resources; isolated canonical block entity membership"]
fn block_entity_data_received_after_merge() {
    if run_fresh_cases(
        "game::remote::block_entity_data_tests::block_entity_data_received_after_merge",
        "HORIZON_RECEIVED_CHILD",
        &["receive"],
    ) {
        return;
    }
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mods: mod_api::Mods = vec![mod_api::ModEntry::new(
        "notice",
        Box::new(TestMod(seen.clone())),
    )]
    .into();
    mod_api::testing::with_mods(mods.clone(), || {
        crate::game::api_values::install_parts(&mods).unwrap();
        horizon_client::block_entity_types::install(
            horizon_client::block_entity_types::BlockEntityTypes::new(
                [
                    ("minecraft:stone".into(), "notice:counter".into()),
                    ("minecraft:dirt".into(), "notice:silent".into()),
                ]
                .into(),
                vec!["notice:counter".into(), "notice:silent".into()],
            ),
        )
        .unwrap();
        with_client_game(|game| {
            let (pipe, _server) = horizon_protocol::connection::Pipe::pair();
            game.connect_local(pipe, "Notice").unwrap();
            game.remote.as_mut().unwrap().logged_in = true;
            Arc::make_mut(&mut game.remote.as_mut().unwrap().builtin)
                .registries
                .insert(
                    "minecraft:block_entity_type".into(),
                    horizon_protocol::registry::Ids::from_names(vec![
                        "notice:counter".into(),
                        "notice:silent".into(),
                        "minecraft:chest".into(),
                    ]),
                );
            let stone = game
                .pack
                .assets
                .registry
                .default_state("minecraft:stone")
                .unwrap();
            let mut column = horizon_client::chunk::Chunk::empty(0, 0);
            column.set(1, 64, 1, stone.0);
            let tag = |n| Tag::Compound([("count".into(), Tag::Int(n))].into());
            let packet = horizon_protocol::packets::play::LevelChunkWithLight {
                x: 0,
                z: 0,
                chunk: horizon_protocol::chunk::ChunkData {
                    heightmaps: Vec::new(),
                    buffer: Vec::new(),
                    block_entities: vec![horizon_protocol::chunk::BlockEntityInfo {
                        packed_xz: 0x11,
                        y: 64,
                        kind: 0,
                        tag: tag(3),
                    }],
                },
                light: horizon_protocol::chunk::LightData {
                    sky_mask: Vec::new(),
                    block_mask: Vec::new(),
                    empty_sky_mask: Vec::new(),
                    empty_block_mask: Vec::new(),
                    sky: Vec::new(),
                    block: Vec::new(),
                },
            };
            let format = game.remote.as_ref().unwrap().chunk_format.clone();
            game.remote_chunk(&packet, Some((&format, Ok(column))));
            assert_eq!(*seen.lock().unwrap(), [(3, 3)]);
            game.remote_block_entity_data([1, 64, 1], 0, &tag(5));
            assert_eq!(*seen.lock().unwrap(), [(3, 3), (5, 5)]);
            let dirt = game
                .pack
                .assets
                .registry
                .default_state("minecraft:dirt")
                .unwrap();
            game.world.set(2, 64, 2, dirt).unwrap();
            game.sync_block_entity((2, 64, 2), None, dirt);
            game.remote_block_entity_data([2, 64, 2], 1, &tag(7));
            assert_eq!(*seen.lock().unwrap(), [(3, 3), (5, 5)]);
            let vanilla = Tag::Compound(
                [
                    ("id".into(), Tag::String("minecraft:chest".into())),
                    ("x".into(), Tag::Int(4)),
                    ("y".into(), Tag::Int(64)),
                    ("z".into(), Tag::Int(4)),
                ]
                .into(),
            );
            let (_, chest) = horizon_client::save::decode_block_entity(&vanilla).unwrap();
            game.world.block_entities.insert((4, 64, 4), chest);
            game.remote_block_entity_data([4, 64, 4], 2, &tag(13));
            game.remote_block_entity_data([1, 64, 1], 0, &Tag::End);
            game.remote_block_entity_data([3, 64, 3], 0, &tag(9));
            game.remote_block_entity_data([1, 64, 1], 1, &tag(11));
            assert_eq!(*seen.lock().unwrap(), [(3, 3), (5, 5)]);
        });
    });
}
