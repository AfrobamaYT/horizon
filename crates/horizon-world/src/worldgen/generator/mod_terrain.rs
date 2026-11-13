//! A level whose terrain a mod's chunk generator writes, with the vanilla stages it asks for
//! ([`VanillaStages`]): the noise generator's pipeline over the level's noise settings and biome
//! source, with the mod's terrain in place of `NoiseBasedChunkGenerator.doFill`. The material
//! rule and the carvers run after it as `buildTerrain` runs them after its fill, and the
//! structures and features as the base `ChunkGenerator` runs them.
use super::*;
use crate::worldgen::mod_generators::{ChunkAccess, ModChunkGenerator, VanillaStages};
use crate::worldgen::terrain::TerrainError;

/// A mod's terrain and the stages its level runs. `stages.structures` is off when the world's
/// structures are.
pub(super) struct ModTerrain {
    generator: Arc<dyn ModChunkGenerator>,
    pub(super) stages: VanillaStages,
}

impl ModTerrain {
    /// The fill of `buildTerrain`: the mod's blocks in `chunk`, which is air, and the
    /// worldgen heightmaps the noise fill keeps (`ChunkStatus.NOISE`'s), which the
    /// material rule and the carvers read.
    pub(super) fn fill(
        &self,
        chunk: &mut TerrainChunk<'_, GenBlockStates>,
        registry: &'static BlockRegistry,
        seed: i64,
    ) -> Result<(), TerrainError> {
        self.generator.build_terrain(&mut Filling {
            chunk,
            registry,
            seed,
        });
        chunk.prime_heightmaps(&[HeightmapType::OceanFloorWg, HeightmapType::WorldSurfaceWg])
    }
}

/// The proto chunk as a mod's generator writes it.
struct Filling<'a, 'r> {
    chunk: &'a mut TerrainChunk<'r, GenBlockStates>,
    registry: &'static BlockRegistry,
    seed: i64,
}

impl ChunkAccess for Filling<'_, '_> {
    fn registry(&self) -> &'static BlockRegistry {
        self.registry
    }
    fn pos(&self) -> ChunkPos {
        self.chunk.position()
    }
    fn seed(&self) -> i64 {
        self.seed
    }
    fn min_y(&self) -> i32 {
        self.chunk.min_y()
    }
    fn height(&self) -> i32 {
        self.chunk.height()
    }
    fn state(&self, name: &str) -> BlockStateId {
        super::super::structure::legacy::block_state(self.registry, name, &[])
    }
    fn state_with(&self, name: &str, properties: &[(&str, &str)]) -> BlockStateId {
        super::super::structure::legacy::block_state(self.registry, name, properties)
    }
    fn set_block(&mut self, [x, y, z]: [i32; 3], state: BlockStateId) -> bool {
        let pos = self.chunk.position();
        x >> 4 == pos.x && z >> 4 == pos.z && self.chunk.set_block_raw(x, y, z, state).is_ok()
    }
}

impl<R: RandomSource + 'static> Pipeline<R>
where
    R::Positional: Clone + Send + Sync,
{
    /// The pipeline of a mod's level: `terrain` in place of the noise, and decoration with the
    /// biomes' placed features only when it asks for them. A stage the world's data cannot
    /// supply (no structure set, no placed feature) refuses the level, naming it.
    pub(super) fn with_mod_terrain(
        mut self,
        terrain: Option<ModTerrain>,
    ) -> Result<Self, GeneratorError> {
        let Some(terrain) = terrain else {
            return Ok(self);
        };
        let stages = terrain.stages;
        if stages.structures && self.structures.is_none() {
            return Err(resource(
                "a mod's chunk generator asks for structures, but the world has no structure sets",
            ));
        }
        if !stages.features {
            // The structure pieces still place in FEATURES, so the decoration stays without them.
            self.decoration = super::super::generator_decoration::Decoration::new_with(
                &self.data,
                &self.source,
                Some(super::super::generator_decoration::FlatDecoration::NONE),
            )?
            .map(Arc::new);
        }
        if self.decoration.is_none() {
            return Err(resource(
                "a mod's chunk generator runs vanilla stages, but the world has no placed features",
            ));
        }
        self.mod_terrain = Some(terrain);
        Ok(self)
    }
}

impl OverworldGenerator {
    /// A level whose stem names `generator`, a mod's that asks for vanilla stages: its terrain,
    /// then those stages, over the stem's noise settings and biome source. `structures` is the
    /// world's `generate_structures`. A level whose stem names no noise settings, or ones the
    /// world lacks, is refused naming the generator and the settings.
    pub fn for_mod(
        data: Arc<GeneratorData>,
        seed: i64,
        stem: &Value,
        generator: Arc<dyn ModChunkGenerator>,
        structures: bool,
    ) -> Result<Self, GeneratorError> {
        let id = stem["generator"]["type"].as_str().unwrap_or_default();
        match &stem["generator"]["settings"] {
            Value::String(name) if !data.settings.contains_key(name) => {
                return Err(resource(format!(
                    "chunk generator {id} runs vanilla stages over the unknown noise settings {name}"
                )));
            }
            Value::String(_) | Value::Object(_) => {}
            _ => {
                return Err(resource(format!(
                    "chunk generator {id} runs vanilla stages, but its level names no noise settings"
                )));
            }
        }
        let mut stages = generator.vanilla_stages();
        stages.structures &= structures;
        let terrain = ModTerrain { generator, stages };
        Self::from_stem(data, seed, stem, stages.structures, None, Some(terrain))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stone from the level's floor up to y 70 in every column of the chunk.
    struct Slab(VanillaStages);
    impl ModChunkGenerator for Slab {
        fn build_terrain(&self, chunk: &mut dyn ChunkAccess) {
            let (pos, stone) = (chunk.pos(), chunk.state("stone"));
            for y in chunk.min_y()..=70 {
                for x in pos.x * 16..pos.x * 16 + 16 {
                    for z in pos.z * 16..pos.z * 16 + 16 {
                        chunk.set_block([x, y, z], stone);
                    }
                }
            }
        }
        fn vanilla_stages(&self) -> VanillaStages {
            self.0
        }
    }

    /// The worldgen data of the prepared resources, loaded once for these tests.
    fn data() -> Arc<GeneratorData> {
        static DATA: OnceLock<Arc<GeneratorData>> = OnceLock::new();
        DATA.get_or_init(|| {
            let root = std::path::PathBuf::from(
                std::env::var_os("HORIZON_TEST_RESOURCES").expect("resources"),
            );
            let registry = Box::leak(Box::new(crate::level::data::load_registry(&root).unwrap()));
            GeneratorData::load(&root.join("worldgen"), registry).unwrap()
        })
        .clone()
    }

    /// An Overworld-shaped level of plains whose terrain is the slab, over the Overworld's noise
    /// settings, at seed 12345.
    fn level(stages: VanillaStages) -> OverworldGenerator {
        let stem = stem(Value::from("minecraft:overworld"));
        OverworldGenerator::for_mod(data(), 12345, &stem, Arc::new(Slab(stages)), true).unwrap()
    }

    fn stem(settings: Value) -> Value {
        serde_json::json!({
            "type": "minecraft:overworld",
            "generator": {
                "type": "test:slab",
                "settings": settings,
                "biome_source": {"type": "minecraft:fixed", "biome": "minecraft:plains"},
            },
        })
    }

    /// The material rule runs over the mod's stone after its terrain: plains grass and dirt on
    /// top, deepslate below y 8 and the bedrock floor.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn the_surface_covers_the_mods_terrain() {
        let level = level(VanillaStages {
            surface: true,
            ..Default::default()
        });
        let chunk = level.worker().generate_proto(ChunkPos::new(0, 0)).unwrap();
        for (y, block) in [
            (71, "minecraft:air"),
            (70, "minecraft:grass_block"),
            (69, "minecraft:dirt"),
            (67, "minecraft:dirt"),
            (66, "minecraft:stone"),
            (0, "minecraft:deepslate"),
            (-60, "minecraft:deepslate"),
            (-64, "minecraft:bedrock"),
        ] {
            assert_eq!(chunk.block_name(0, y, 0), block, "y {y}");
        }
    }

    /// A carver cuts the mod's stone at seed 12345, the aquifer of the Overworld's noise filling
    /// it below sea level; the top stays the mod's stone without the surface.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn the_carvers_cut_the_mods_terrain() {
        let level = level(VanillaStages {
            carvers: true,
            ..Default::default()
        });
        let chunk = level.worker().generate_proto(ChunkPos::new(0, 1)).unwrap();
        assert_eq!(chunk.block_name(0, 40, 25), "minecraft:water");
        assert_eq!(chunk.block_name(0, 70, 16), "minecraft:stone");
    }

    /// At seed 12345 a ruined portal starts in chunk 1 0 and its pieces place there in
    /// FEATURES (not without the structures), and the plains' grass grows on the surface in
    /// chunk 0 0.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn structures_and_features_place_after_the_terrain() {
        let all = VanillaStages {
            surface: true,
            carvers: true,
            structures: true,
            features: true,
        };
        // The decorated chunk `x` 0 of a level with `stages`.
        let decorated = |stages, x: i32| {
            let mut worker = level(stages).worker();
            let mut region: [ProtoChunk; 9] = std::array::from_fn(|i| {
                let i = i as i32;
                worker
                    .generate_proto(ChunkPos::new(x + i % 3 - 1, i / 3 - 1))
                    .unwrap()
            });
            worker.decorate(&mut region).unwrap();
            let [.., center, _, _, _, _] = region;
            (worker, center)
        };
        let (mut worker, chunk) = decorated(all, 0);
        assert_eq!(chunk.block_name(15, 71, 0), "minecraft:short_grass");
        assert_eq!(chunk.block_name(15, 70, 0), "minecraft:grass_block");
        let starts = worker.structure_starts(ChunkPos::new(1, 0));
        let portal = &starts[0];
        assert_eq!(portal.name, "minecraft:ruined_portal");
        assert_eq!(
            (portal.bounding_box.min, portal.bounding_box.max),
            ([15, 12, 1], [21, 18, 5])
        );
        let (_, with) = decorated(all, 1);
        let (_, without) = decorated(
            VanillaStages {
                structures: false,
                ..all
            },
            1,
        );
        for ([x, y, z], piece) in [
            ([16, 12, 1], "minecraft:netherrack"),
            ([16, 12, 3], "minecraft:magma_block"),
            ([16, 13, 1], "minecraft:air"),
        ] {
            assert_eq!(with.block_name(x, y, z), piece, "{x} {y} {z}");
            assert_ne!(without.block_name(x, y, z), piece, "{x} {y} {z}");
        }
    }

    /// A level whose stem names no noise settings, or ones the world lacks, is refused naming
    /// the generator and the settings.
    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn a_level_without_its_noise_settings_is_refused_by_name() {
        let refusal = |settings| {
            let slab = Arc::new(Slab(VanillaStages {
                surface: true,
                ..Default::default()
            }));
            match OverworldGenerator::for_mod(data(), 0, &stem(settings), slab, true) {
                Ok(_) => panic!("opened"),
                Err(e) => e.to_string(),
            }
        };
        assert_eq!(
            refusal(Value::Null),
            "invalid generator resources: chunk generator test:slab runs vanilla stages, but its \
             level names no noise settings"
        );
        assert_eq!(
            refusal(Value::from("test:gone")),
            "invalid generator resources: chunk generator test:slab runs vanilla stages over the \
             unknown noise settings test:gone"
        );
    }
}
