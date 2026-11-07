use super::*;
use horizon_world::worldgen::structure::legacy::Result as PieceResult;
use mod_api::Tag;
use mod_api::{FoundStructure, StructureError};

/// A type whose piece is a stone box of a size the config names, from the
/// chunk's corner: what the adapter saves, it loads back.
struct Boxes;
#[derive(Debug)]
struct BoxPiece {
    min: [i32; 3],
    max: [i32; 3],
}
impl StructurePiece for BoxPiece {
    fn bounding_box(&self) -> BoundingBox {
        BoundingBox::new(self.min, self.max)
    }
    fn post_process(&self, _: &mut dyn PieceContext) -> Result<(), StructureError> {
        Ok(())
    }
    fn add_additional_save_data(&self, tag: &mut Compound) {
        tag.insert("Min".into(), Tag::IntArray(self.min.to_vec()));
        tag.insert("Max".into(), Tag::IntArray(self.max.to_vec()));
    }
}
impl StructureType for Boxes {
    fn find_generation_point(&self, context: &mut dyn GenerationContext) -> Option<FoundStructure> {
        let chunk = context.chunk();
        let height = context.base_height(
            chunk.x * 16 + 8,
            chunk.z * 16 + 8,
            HeightmapType::WorldSurfaceWg,
        );
        let min = [chunk.x * 16, height - 1, chunk.z * 16];
        Some(FoundStructure {
            position: min,
            pieces: vec![Arc::new(BoxPiece {
                min,
                max: [min[0] + 8, min[1] + 5, min[2] + 8],
            })],
        })
    }
    fn load_piece(&self, tag: &Compound) -> Option<Arc<dyn StructurePiece>> {
        let ints = |key: &str| match tag.get(&key.into()) {
            Some(Tag::IntArray(a)) if a.len() == 3 => Some([a[0], a[1], a[2]]),
            _ => None,
        };
        Some(Arc::new(BoxPiece {
            min: ints("Min")?,
            max: ints("Max")?,
        }))
    }
}

/// The same seed gives the same pieces: the type's answer depends on
/// nothing but its context, so two worlds of one seed see one structure.
#[test]
fn the_same_context_gives_the_same_pieces() {
    let adapter = Adapter::new("testmod:boxes", Arc::new(Boxes));
    let run = || {
        let mut random = WorldgenRandom::new(LegacyRandom::new(0));
        random.set_large_feature_seed(12345, 3, 7);
        // A context of the queries Boxes asks; the engine's own tests
        // cover the context itself.
        let (position, pieces) = adapter
            .find_generation_point(
                &mut StubContext,
                test_resources(),
                ChunkPos::new(3, 7),
                &Arc::new(Tag::Compound(Compound::new())),
                &mut random,
            )
            .unwrap();
        (position, pieces[0].bounding_box(), pieces.len())
    };
    assert_eq!(run(), run());
}

/// `createTag` + the piece's own data load back into the same piece: a
/// saved tower comes back the same tower, not a rebuilt one.
#[test]
fn a_saved_piece_loads_back_the_same() {
    let adapter = Adapter::new("testmod:boxes", Arc::new(Boxes));
    let mut random = WorldgenRandom::new(LegacyRandom::new(0));
    random.set_large_feature_seed(1, 0, 0);
    let (_, pieces) = adapter
        .find_generation_point(
            &mut StubContext,
            test_resources(),
            ChunkPos::new(0, 0),
            &Arc::new(Tag::Compound(Compound::new())),
            &mut random,
        )
        .unwrap();
    let tag = pieces[0].save().unwrap();
    assert!(matches!(
        tag.get(&"id".into()),
        Some(Tag::String(s)) if s.to_string_lossy() == "testmod:boxes"
    ));
    let loaded = adapter.load_piece(&tag).unwrap();
    assert_eq!(loaded.bounding_box(), pieces[0].bounding_box());
    assert_eq!(loaded.type_name(), "testmod:boxes");
    // And what came back is the saved one, not a fresh generation.
    let again = loaded.save().unwrap();
    assert_eq!(
        again.get(&"Min".into()).unwrap(),
        tag.get(&"Min".into()).unwrap()
    );
}

/// The engine's once-per-chunk `afterPlace`, which it runs over the first
/// piece, reaches the type's `after_place` — the piece has none — with
/// every piece's box.
#[test]
fn the_engines_after_place_call_reaches_the_type() {
    use horizon_world::worldgen::structure::legacy::{PieceContext as EngineContext, SolidRender};
    struct Counting {
        calls: std::sync::Arc<std::sync::Mutex<Vec<usize>>>,
    }
    impl StructureType for Counting {
        fn find_generation_point(
            &self,
            context: &mut dyn GenerationContext,
        ) -> Option<FoundStructure> {
            let chunk = context.chunk();
            let height = context.base_height(
                chunk.x * 16 + 8,
                chunk.z * 16 + 8,
                HeightmapType::WorldSurfaceWg,
            );
            let min = [chunk.x * 16, height - 1, chunk.z * 16];
            Some(FoundStructure {
                position: min,
                pieces: vec![Arc::new(BoxPiece {
                    min,
                    max: [min[0] + 8, min[1] + 5, min[2] + 8],
                })],
            })
        }
        fn load_piece(&self, _: &Compound) -> Option<Arc<dyn StructurePiece>> {
            None
        }
        fn after_place(
            &self,
            _: &mut dyn PieceContext,
            pieces: &[BoundingBox],
        ) -> Result<(), StructureError> {
            self.calls.lock().unwrap().push(pieces.len());
            Ok(())
        }
    }
    let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let adapter = Adapter::new(
        "testmod:counting",
        Arc::new(Counting {
            calls: calls.clone(),
        }),
    );
    let mut random = WorldgenRandom::new(LegacyRandom::new(0));
    random.set_large_feature_seed(2, 1, 1);
    let (_, pieces) = adapter
        .find_generation_point(
            &mut StubContext,
            test_resources(),
            ChunkPos::new(1, 1),
            &Arc::new(Tag::Compound(Compound::new())),
            &mut random,
        )
        .unwrap();
    let boxes: Vec<BoundingBox> = pieces.iter().map(|p| p.bounding_box()).collect();
    let resources = test_resources();
    struct NoSolid;
    impl SolidRender for NoSolid {
        fn solid_render(&self, _: horizon_world::registry::BlockStateId) -> bool {
            false
        }
    }
    let air = resources.registry.default_state("minecraft:air").unwrap();
    let mut world = RecordingWorld {
        air,
        writes: Vec::new(),
    };
    let mut piece_random = LegacyRandom::new(3);
    let mut engine = EngineContext {
        world: &mut world,
        resources,
        blocks: &NoSolid,
        random: &mut piece_random,
        chunk_box: BoundingBox::new([0, -64, 0], [15, 320, 15]),
        chunk: ChunkPos::new(0, 0),
        reference: [8, 64, 8],
    };
    // What the engine does once per chunk: the first piece's call.
    pieces[0].after_place(&mut engine, &boxes).unwrap();
    assert_eq!(*calls.lock().unwrap(), vec![boxes.len()]);
}

/// The start context the stub answers: heights from a flat 64.
struct StubContext;
impl horizon_world::worldgen::structure::jigsaw::JigsawContext for StubContext {
    fn min_y(&self) -> i32 {
        -64
    }
    fn max_y(&self) -> i32 {
        320
    }
    fn base_height(&mut self, _: i32, _: i32, _: horizon_world::heightmap::HeightmapType) -> i32 {
        65
    }
    fn column_has_valid_biome(&mut self, _: i32, _: i32, _: i32, _: i32) -> bool {
        true
    }
}
impl horizon_world::worldgen::structure::start::StartContext for StubContext {
    fn seed(&self) -> i64 {
        1
    }
    fn sea_level(&self) -> i32 {
        63
    }
    fn base_column(
        &mut self,
        _: i32,
        _: i32,
        _: horizon_world::heightmap::HeightmapType,
    ) -> Vec<bool> {
        Vec::new()
    }
    fn base_columns_by(
        &mut self,
        _: i32,
        _: i32,
        _: usize,
        _: &dyn Fn(usize) -> horizon_world::heightmap::HeightmapType,
    ) -> Vec<Vec<bool>> {
        Vec::new()
    }
    fn noise_column_into(
        &mut self,
        _: i32,
        _: i32,
        column: &mut dyn horizon_world::worldgen::structure::start::NoiseColumnTarget,
    ) {
        column.resize(0, 0);
    }
    fn cold_enough_to_snow(
        &mut self,
        _: horizon_world::worldgen::biome::BiomeId,
        _: [i32; 3],
    ) -> bool {
        false
    }
    fn generation(&self) -> horizon_world::worldgen::anchor::WorldGenerationContext {
        horizon_world::worldgen::anchor::WorldGenerationContext::new(-64, 384, 63)
    }
    fn noise_biome(&mut self, _: [i32; 3]) -> horizon_world::worldgen::biome::BiomeId {
        horizon_world::worldgen::biome::BiomeId(0)
    }
    fn noise_biomes(
        &mut self,
        quarts: &[[i32; 3]],
    ) -> Vec<horizon_world::worldgen::biome::BiomeId> {
        vec![horizon_world::worldgen::biome::BiomeId(0); quarts.len()]
    }
    fn set_valid_biomes(
        &mut self,
        _: Option<&std::collections::BTreeSet<horizon_world::worldgen::biome::BiomeId>>,
    ) {
    }
}

/// The piece world of the clipping test: every write recorded, and a
/// write outside the test's expectations fails it.
struct RecordingWorld {
    air: horizon_world::registry::BlockStateId,
    writes: Vec<[i32; 3]>,
}
impl horizon_world::worldgen::structure::legacy::PieceWorld for RecordingWorld {
    fn block(&mut self, _: [i32; 3]) -> PieceResult<horizon_world::registry::BlockStateId> {
        Ok(self.air)
    }
    fn set(
        &mut self,
        pos: [i32; 3],
        _: horizon_world::registry::BlockStateId,
        _: i32,
    ) -> PieceResult<bool> {
        self.writes.push(pos);
        Ok(true)
    }
    fn fluid(&mut self, _: [i32; 3]) -> PieceResult<horizon_world::fluid::FluidState> {
        Ok(horizon_world::fluid::FluidState::EMPTY)
    }
    fn height(
        &mut self,
        _: horizon_world::heightmap::HeightmapType,
        _: i32,
        _: i32,
    ) -> PieceResult<i32> {
        Ok(64)
    }
    fn min_y(&mut self) -> PieceResult<i32> {
        Ok(-64)
    }
    fn max_y(&mut self) -> PieceResult<i32> {
        Ok(320)
    }
    fn block_entity(&mut self, _: [i32; 3], _: Compound) -> PieceResult<()> {
        Ok(())
    }
    fn entity(&mut self, _: Compound) -> PieceResult<()> {
        Ok(())
    }
    fn post_process(&mut self, _: [i32; 3]) -> PieceResult<()> {
        Ok(())
    }
    fn fluid_tick(
        &mut self,
        _: [i32; 3],
        _: horizon_world::fluid::FluidKind,
        _: i32,
    ) -> PieceResult<()> {
        Ok(())
    }
    fn seed(&mut self) -> PieceResult<i64> {
        Ok(0)
    }
    fn level_boolean(&mut self) -> PieceResult<bool> {
        Ok(false)
    }
    fn level_float(&mut self) -> PieceResult<f32> {
        Ok(0.0)
    }
    fn level_int(&mut self, _: i32) -> PieceResult<i32> {
        Ok(0)
    }
    fn sea_level(&mut self) -> PieceResult<i32> {
        Ok(63)
    }
    fn face_full(
        &mut self,
        _: [i32; 3],
        _: horizon_core::geometry::Direction,
    ) -> PieceResult<bool> {
        Ok(false)
    }
    fn face_sturdy(
        &mut self,
        _: [i32; 3],
        _: horizon_core::geometry::Direction,
    ) -> PieceResult<bool> {
        Ok(false)
    }
    fn center_sturdy(
        &mut self,
        _: [i32; 3],
        _: horizon_core::geometry::Direction,
    ) -> PieceResult<bool> {
        Ok(false)
    }
    fn biome(&mut self, _: [i32; 3]) -> PieceResult<horizon_world::worldgen::biome::BiomeId> {
        Err(PlacementQueryError::Unavailable)
    }
    fn place_template(
        &mut self,
        _: &horizon_world::worldgen::structure::StructureResources,
        _: &horizon_world::worldgen::structure::StructureTemplate,
        _: [i32; 3],
        _: [i32; 3],
        _: &mut horizon_world::worldgen::structure::processor::PlaceSettings<'_>,
        _: &mut dyn horizon_world::worldgen::structure::processor::ProcessorRandom,
        _: i32,
    ) -> PieceResult<bool> {
        Ok(true)
    }
}

/// The engine's own placement tests build their registry this way
/// (`legacy::tests::registry`): two facts rows, held once for the
/// `&'static` the engine's resources take — no game resources needed.
fn test_resources() -> &'static horizon_world::worldgen::structure::StructureResources {
    mod_api::__private::part_tests::install_parts_fixture(&[]).unwrap();
    let facts = serde_json::json!({
        "states": [
            ["air", true, false, "", {"id":"minecraft:air","properties":{}}],
            ["stone", false, false, "", {"id":"minecraft:stone","properties":{}}]
        ],
        "defaults":{"minecraft:air":0,"minecraft:stone":1}
    });
    let registry =
        horizon_world::registry::BlockRegistry::from_facts_json(facts.to_string().as_bytes())
            .unwrap();
    crate::game::api_values::resolve_block_definitions(&registry).unwrap();
    Box::leak(Box::new(
        horizon_world::worldgen::structure::StructureResources::new(
            Box::leak(Box::new(registry)),
            horizon_world::worldgen::structure::StateTransforms::default(),
            std::path::Path::new("/nonexistent"),
            std::collections::BTreeMap::new(),
            &std::collections::BTreeMap::new(),
        ),
    ))
}

/// `placeBlock`'s clipping: a write outside `chunk_box` answers `false`
/// and touches nothing, silently, as vanilla's `placeBlock` skips it; one
/// inside writes.
#[test]
fn a_write_outside_the_chunk_box_is_dropped_silently() {
    use horizon_world::worldgen::structure::legacy::{PieceContext as EngineContext, SolidRender};
    let resources = test_resources();
    struct Air;
    impl SolidRender for Air {
        fn solid_render(&self, _: horizon_world::registry::BlockStateId) -> bool {
            false
        }
    }
    let air = resources.registry.default_state("minecraft:air").unwrap();
    let mut world = RecordingWorld {
        air,
        writes: Vec::new(),
    };
    let mut random = horizon_core::random::LegacyRandom::new(7);
    {
        let mut engine = EngineContext {
            world: &mut world,
            resources,
            blocks: &Air,
            random: &mut random,
            // Chunk (0, 0)'s writable box.
            chunk_box: BoundingBox::new([0, -64, 0], [15, 320, 15]),
            chunk: ChunkPos::new(0, 0),
            reference: [8, 64, 8],
        };
        let mut context = Placing::new(&mut engine).unwrap();
        let stone = context.state(mod_api::__private::block_key("minecraft:stone").unwrap());
        // Outside on every axis: dropped, silently, nothing recorded.
        for outside in [[16, 64, 0], [0, 64, 16], [-1, 64, 0], [0, 64, -1]] {
            assert!(!context.set_block(outside, stone));
        }
        // Inside: written.
        assert!(context.set_block([15, 64, 15], stone));
    }
    assert_eq!(world.writes, [[15, 64, 15]]);
}
