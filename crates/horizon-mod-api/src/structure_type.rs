//! Structure types a trait mod registers: vanilla's `StructureType` with its
//! own `Structure.findGenerationPoint` and `StructurePiece`s, as
//! [`Registrar::register_structure_type`](super::Registrar::register_structure_type)
//! hands it over once at startup (NeoForge's registries `STRUCTURE_TYPE` and
//! `STRUCTURE_PIECE`). A data pack structure with `"type": "modid:tower"`
//! then generates by the mod's code.
//!
//! There is no `Host` here: starts build on the worldgen worker threads and
//! pieces place per chunk during generation, so the mod sees only the small
//! contexts below, `Send + Sync` and deterministic — the same seed gives the
//! same pieces, and a type keeps no mutable state because two workers can
//! build neighbouring chunks at once. Everything around the type stays the
//! game's own: the set's spacing, the structure's biomes and its
//! `terrain_adaptation`, and the pieces ride the one piece mechanism the game
//! has, next to a fortress piece's.
use horizon_core::random::LegacyRandom;
use std::sync::Arc;
use {
    horizon_core::bounding_box::BoundingBox,
    horizon_world::worldgen::structure::mod_types::ModStructureType,
};
use {horizon_core::nbt::Compound, horizon_world::worldgen::placement::PlacementQueryError};

/// A piece's placement failed; the generation of that piece stops there, as a
/// piece's own `Result` does in the engine.
#[derive(Debug)]
// exhaustive: mods build it themselves as a piece's error answer
pub struct StructureError(pub String);
impl std::fmt::Display for StructureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for StructureError {}

/// What [`StructureType::find_generation_point`](StructureType::find_generation_point)
/// found: the start's position (vanilla's `Structure.findGenerationPoint`
/// answer) and its pieces (`Structure.generatePieces`), at least one.
// exhaustive: mods build it themselves as find_generation_point's answer
pub struct FoundStructure {
    /// Where the start sits, in world blocks.
    pub position: [i32; 3],
    /// The pieces the start places, at least one, shared with the game, which keeps them in
    /// its start.
    pub pieces: Vec<Arc<dyn StructurePiece>>,
}

/// A mod's structure type: vanilla's `StructureType`, whose code answers for a
/// data pack structure with `"type": "modid:tower"`. Implemented once, kept by
/// the game; deterministic and stateless.
pub trait StructureType: Send + Sync + 'static {
    /// `Structure.findGenerationPoint` with `generatePieces`: the start's
    /// position and its pieces for the chunk `context` names, or `None` for
    /// "no start in this chunk". The height and biome queries the context
    /// answers are the ones vanilla's own types ask of a
    /// `GenerationContext`; the structure's JSON config is `context.config`.
    fn find_generation_point(&self, context: &mut dyn GenerationContext) -> Option<FoundStructure>;
    /// The `StructurePieceType` loader: rebuild a saved piece from its tag
    /// (`StructurePieceType.load`), exactly as it was saved. Required: every
    /// piece type has its loader, and a saved piece is never regenerated.
    fn load_piece(&self, tag: &Compound) -> Option<Arc<dyn StructurePiece>>;
    /// `Structure.afterPlace`, run once per chunk after all pieces were
    /// placed, with every piece's current box. Optional: most types place
    /// nothing here.
    fn after_place(
        &self,
        _context: &mut dyn PieceContext,
        _pieces: &[BoundingBox],
    ) -> Result<(), StructureError> {
        Ok(())
    }
}

/// One piece of a mod's structure: vanilla's `StructurePiece`. Its `save` is
/// the engine's (`StructurePiece.createTag`: the type id, `BB`, `O`, `GD`)
/// around [`add_additional_save_data`](StructurePiece::add_additional_save_data).
pub trait StructurePiece: std::fmt::Debug + Send + Sync {
    /// `StructurePiece.getBoundingBox`: the piece's box, in world blocks.
    fn bounding_box(&self) -> BoundingBox;
    /// `StructurePiece.postProcess`: place the piece's blocks and entities
    /// that fall in the chunk `context` clips to. Writes outside it are
    /// dropped silently, as vanilla's `placeBlock` skips them.
    fn post_process(&self, context: &mut dyn PieceContext) -> Result<(), StructureError>;
    /// `StructurePiece.addAdditionalSaveData`: the piece's own fields, saved
    /// beside `BB`/`O`/`GD` and read back by the type's `load_piece`.
    fn add_additional_save_data(&self, tag: &mut Compound);
}

/// The random a structure type or piece draws from, in vanilla's order: the
/// chunk's `WorldgenRandom` for a start, the piece's `RandomSource` for a piece.
pub trait StructureRandom {
    /// `nextInt(bound)`; 0 for a bound below 1.
    fn next_int_bound(&mut self, bound: i32) -> i32;
    /// `nextLong`.
    fn next_long(&mut self) -> i64;
    /// `nextFloat`.
    fn next_float(&mut self) -> f32;
    /// `nextDouble`.
    fn next_double(&mut self) -> f64;
    /// `nextBoolean`.
    fn next_boolean(&mut self) -> bool;
}

/// The engine's piece random, as a [`StructureRandom`].
struct Draws<'a>(&'a mut dyn PieceRandom);
impl StructureRandom for Draws<'_> {
    fn next_int_bound(&mut self, bound: i32) -> i32 {
        self.0.int(bound)
    }
    fn next_long(&mut self) -> i64 {
        self.0.long()
    }
    fn next_float(&mut self) -> f32 {
        self.0.float()
    }
    fn next_double(&mut self) -> f64 {
        self.0.double()
    }
    fn next_boolean(&mut self) -> bool {
        self.0.boolean()
    }
}

/// The start context a type's
/// [`find_generation_point`](StructureType::find_generation_point) sees:
/// vanilla's `GenerationContext` over one chunk, on the worker thread;
/// deterministic for the seed.
pub trait GenerationContext {
    /// The chunk the start would be in.
    fn chunk(&self) -> ChunkPos;
    /// The world's seed.
    fn seed(&self) -> i64;
    /// The level's sea level.
    fn sea_level(&self) -> i32;
    /// The level's minimum block Y.
    fn min_y(&self) -> i32;
    /// The level's maximum block Y.
    fn max_y(&self) -> i32;
    /// The random vanilla's types draw from, seeded for this chunk
    /// (`WorldgenRandom.setLargeFeatureSeed` of the seed, chunk X and Z).
    fn random(&mut self) -> &mut dyn StructureRandom;
    /// `ChunkGenerator.getBaseHeight`: the first free height at (x, z) by the
    /// heightmap kind.
    fn base_height(&mut self, x: i32, z: i32, kind: HeightmapType) -> i32;
    /// `ChunkGenerator.getBaseColumn` as heightmap opacity, from `min_y` up,
    /// for several kinds from one noise-column pass.
    fn base_columns(&mut self, x: i32, z: i32, kinds: &[HeightmapType]) -> Vec<Vec<bool>>;
    /// `ChunkGenerator.getBaseColumn`'s blocks: the noise column at (x, z).
    fn noise_column(&mut self, x: i32, z: i32) -> NoiseColumn;
    /// Noise biome at a quart position (the resolver's, no zoom).
    fn noise_biome(&mut self, quart: [i32; 3]) -> BiomeId;
    /// `GenerationContext.validBiome` over inclusive block Y bounds of the
    /// column at (x, z): whether any biome the structure names occurs there.
    fn column_has_valid_biome(&mut self, x: i32, z: i32, min_y: i32, max_y: i32) -> bool;
    /// The structure's own JSON config, as the data pack has it.
    fn config(&self) -> Arc<serde_json::Value>;
}

/// The engine's start, as a [`GenerationContext`].
struct Start<'a> {
    inner: &'a mut dyn horizon_world::worldgen::structure::start::StartContext,
    random: &'a mut WorldgenRandom<LegacyRandom>,
    chunk: ChunkPos,
    config: &'a Arc<serde_json::Value>,
}

impl GenerationContext for Start<'_> {
    fn chunk(&self) -> ChunkPos {
        self.chunk
    }
    fn seed(&self) -> i64 {
        self.inner.seed()
    }
    fn sea_level(&self) -> i32 {
        self.inner.sea_level()
    }
    fn min_y(&self) -> i32 {
        self.inner.min_y()
    }
    fn max_y(&self) -> i32 {
        self.inner.max_y()
    }
    fn random(&mut self) -> &mut dyn StructureRandom {
        self.random
    }
    fn base_height(&mut self, x: i32, z: i32, kind: HeightmapType) -> i32 {
        self.inner.base_height(x, z, kind)
    }
    fn base_columns(&mut self, x: i32, z: i32, kinds: &[HeightmapType]) -> Vec<Vec<bool>> {
        self.inner.base_columns(x, z, kinds)
    }
    fn noise_column(&mut self, x: i32, z: i32) -> NoiseColumn {
        self.inner.noise_column(x, z)
    }
    fn noise_biome(&mut self, quart: [i32; 3]) -> BiomeId {
        self.inner.noise_biome(quart)
    }
    fn column_has_valid_biome(&mut self, x: i32, z: i32, min_y: i32, max_y: i32) -> bool {
        self.inner.column_has_valid_biome(x, z, min_y, max_y)
    }
    fn config(&self) -> Arc<serde_json::Value> {
        self.config.clone()
    }
}

/// The chunk's random a start draws from.
impl StructureRandom for WorldgenRandom<LegacyRandom> {
    fn next_int_bound(&mut self, bound: i32) -> i32 {
        ProcessorRandom::int(self, bound)
    }
    fn next_long(&mut self) -> i64 {
        RandomSource::next_long(self)
    }
    fn next_float(&mut self) -> f32 {
        RandomSource::next_float(self)
    }
    fn next_double(&mut self) -> f64 {
        RandomSource::next_double(self)
    }
    fn next_boolean(&mut self) -> bool {
        RandomSource::next_boolean(self)
    }
}

/// The types the contexts' own signatures answer, re-exported so a mod's
/// code names them through the API, never the engine's crates.
pub use horizon_core::position::ChunkPos;
use horizon_core::random::{RandomSource, WorldgenRandom};
/// The noise column a [`GenerationContext`](GenerationContext) reads:
/// `ChunkGenerator.getBaseColumn`'s blocks, `Air` past its ends.
pub use horizon_world::worldgen::structure::start::{NoiseBlock, NoiseColumn};
use horizon_world::worldgen::structure::{legacy::PieceRandom, processor::ProcessorRandom};
/// The heightmap kinds a [`GenerationContext`](GenerationContext) queries and
/// the noise biome a chunk's column holds.
pub use horizon_world::{heightmap::HeightmapType, worldgen::biome::BiomeId};

/// The placement context a piece's
/// [`post_process`](StructurePiece::post_process) and a type's
/// [`after_place`](StructureType::after_place) see: the blocks and entities of
/// one chunk, the piece's random and what the blocks around it are made of.
pub trait PieceContext {
    /// The chunk being placed.
    fn chunk(&self) -> ChunkPos;
    /// The writable box: a `set_block` outside it is dropped silently.
    fn chunk_box(&self) -> BoundingBox;
    /// The piece's random (`RandomSource` of `postProcess`).
    fn random(&mut self) -> &mut dyn StructureRandom;
    /// A block state by block name (`stone`, `minecraft:oak_log`), with its
    /// default properties.
    fn state(&self, name: &str) -> horizon_world::registry::BlockStateId;
    /// A block state by block name with properties (`oak_log` with
    /// `axis=y`); missing properties keep defaults.
    fn state_with(
        &self,
        name: &str,
        properties: &[(&str, &str)],
    ) -> horizon_world::registry::BlockStateId;
    /// The block's name (`minecraft:stone`), or "" for an unknown state.
    fn name(&self, state: horizon_world::registry::BlockStateId) -> String;
    /// `BlockState.isAir`.
    fn is_air(&self, state: horizon_world::registry::BlockStateId) -> bool;
    /// The block at `pos`, or `None` outside the region.
    fn block(&mut self, pos: [i32; 3]) -> Option<horizon_world::registry::BlockStateId>;
    /// `StructurePiece.placeBlock`: write `state` at `pos`, answering whether
    /// it was written. A position outside `chunk_box` is skipped silently, as
    /// vanilla's `placeBlock` skips it.
    fn set_block(&mut self, pos: [i32; 3], state: horizon_world::registry::BlockStateId) -> bool;
    /// `WorldGenLevel.getHeight` of the heightmap kind at (x, z).
    fn height(&mut self, kind: HeightmapType, x: i32, z: i32) -> Option<i32>;
    /// `BlockState.isSolidRender` of a state: whether the block draws as a
    /// solid block, the way `StructurePiece.reorient` asks of its neighbours.
    fn solid_render(&self, state: horizon_world::registry::BlockStateId) -> bool;
    /// `WorldGenLevel.addFreshEntity`: an entity from its saved NBT — its
    /// `id`, `Pos` and fields — which the generating chunk holds and the game
    /// finishes as it does a structure mob's when the chunk joins the world
    /// (`Mob.finalizeSpawn` with `STRUCTURE`; keep the
    /// `"horizon:finalize"` byte a structure template's entities carry to ask
    /// for it).
    fn entity(&mut self, nbt: Compound);
    /// `WorldGenLevel.setBlockEntity` at `pos`: the block entity's saved NBT
    /// (its `id` and fields).
    fn block_entity(&mut self, pos: [i32; 3], nbt: Compound);
}

/// The engine's piece context, as a [`PieceContext`].
struct Placing<'a> {
    world: &'a mut dyn horizon_world::worldgen::structure::legacy::PieceWorld,
    resources: &'a horizon_world::worldgen::structure::StructureResources,
    blocks: &'a dyn horizon_world::worldgen::structure::legacy::SolidRender,
    random: Draws<'a>,
    chunk_box: BoundingBox,
    chunk: ChunkPos,
}

impl<'a> Placing<'a> {
    fn new<'b>(
        inner: &'a mut horizon_world::worldgen::structure::legacy::PieceContext<'b>,
    ) -> Self {
        Self {
            world: inner.world,
            resources: inner.resources,
            blocks: inner.blocks,
            random: Draws(inner.random),
            chunk_box: inner.chunk_box,
            chunk: inner.chunk,
        }
    }
    fn block_name(&self, state: horizon_world::registry::BlockStateId) -> &str {
        self.resources
            .registry
            .state(state)
            .map(|s| s.name.as_str())
            .unwrap_or("")
    }
}

impl PieceContext for Placing<'_> {
    fn chunk(&self) -> ChunkPos {
        self.chunk
    }
    fn chunk_box(&self) -> BoundingBox {
        self.chunk_box
    }
    fn random(&mut self) -> &mut dyn StructureRandom {
        &mut self.random
    }
    fn state(&self, name: &str) -> horizon_world::registry::BlockStateId {
        horizon_world::worldgen::structure::legacy::state(self.resources, name, &[])
    }
    fn state_with(
        &self,
        name: &str,
        properties: &[(&str, &str)],
    ) -> horizon_world::registry::BlockStateId {
        horizon_world::worldgen::structure::legacy::state(self.resources, name, properties)
    }
    fn name(&self, state: horizon_world::registry::BlockStateId) -> String {
        self.block_name(state).to_owned()
    }
    fn is_air(&self, state: horizon_world::registry::BlockStateId) -> bool {
        matches!(
            self.block_name(state),
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        )
    }
    fn block(&mut self, pos: [i32; 3]) -> Option<horizon_world::registry::BlockStateId> {
        self.world.block(pos).ok()
    }
    #[cfg_attr(not(debug_assertions), allow(unused_variables))]
    fn set_block(&mut self, pos: [i32; 3], state: horizon_world::registry::BlockStateId) -> bool {
        if !self.chunk_box.is_inside(pos) {
            #[cfg(debug_assertions)]
            log::debug!(
                "mod structure piece write at {:?} outside the chunk box",
                pos
            );
            return false;
        }
        horizon_world::worldgen::structure::legacy::write_block(
            self.world,
            self.resources,
            pos,
            state,
        )
        .unwrap_or(false)
    }
    fn height(&mut self, kind: HeightmapType, x: i32, z: i32) -> Option<i32> {
        self.world.height(kind, x, z).ok()
    }
    fn solid_render(&self, state: horizon_world::registry::BlockStateId) -> bool {
        self.blocks.solid_render(state)
    }
    fn entity(&mut self, nbt: Compound) {
        if self.world.entity(nbt).is_err() {
            log::error!("Horizon: a mod structure piece's entity was refused");
        }
    }
    fn block_entity(&mut self, pos: [i32; 3], nbt: Compound) {
        if self.world.block_entity(pos, nbt).is_err() {
            log::error!("Horizon: a mod structure piece's block entity was refused");
        }
    }
}

/// The adapter: the game's `ModStructureType` around a mod's `StructureType`,
/// and its `LegacyPiece` around the mod's `StructurePiece`s. One adapter, no
/// second mechanism: the pieces a type returns become the engine's legacy
/// pieces, so the start cache, the saving, the cross-chunk placement and the
/// beardifier treat them like a fortress piece's.
pub(crate) struct Adapter {
    inner: Arc<dyn StructureType>,
    /// The type's namespaced id (`modid:tower`), leaked once at registration
    /// for the piece tags' `"id"`.
    id: &'static str,
}

impl ModStructureType for Adapter {
    fn find_generation_point(
        &self,
        context: &mut dyn horizon_world::worldgen::structure::start::StartContext,
        chunk: ChunkPos,
        config: &Arc<serde_json::Value>,
        random: &mut WorldgenRandom<LegacyRandom>,
    ) -> Option<(
        [i32; 3],
        Vec<Box<dyn horizon_world::worldgen::structure::legacy::LegacyPiece>>,
    )> {
        let mut context = Start {
            inner: context,
            random,
            chunk,
            config,
        };
        let found = self.inner.find_generation_point(&mut context)?;
        let ty = self.inner.clone();
        let id = self.id;
        let pieces = found
            .pieces
            .into_iter()
            .map(|piece| {
                Box::new(ModPiece {
                    piece,
                    ty: ty.clone(),
                    id,
                })
                    as Box<dyn horizon_world::worldgen::structure::legacy::LegacyPiece>
            })
            .collect();
        Some((found.position, pieces))
    }
    fn load_piece(
        &self,
        tag: &Compound,
    ) -> Option<Box<dyn horizon_world::worldgen::structure::legacy::LegacyPiece>> {
        let piece = self.inner.load_piece(tag)?;
        Some(Box::new(ModPiece {
            piece,
            ty: self.inner.clone(),
            id: self.id,
        }))
    }
}

/// The engine's legacy piece around a mod's piece: `createTag` of the type id
/// and the piece's box around the piece's own save data.
struct ModPiece {
    piece: Arc<dyn StructurePiece>,
    /// The type the piece belongs to, whose `after_place` runs for the start.
    ty: Arc<dyn StructureType>,
    id: &'static str,
}
impl horizon_world::worldgen::structure::legacy::LegacyPiece for ModPiece {
    fn bounding_box(&self) -> BoundingBox {
        self.piece.bounding_box()
    }
    fn type_name(&self) -> &'static str {
        self.id
    }
    fn save(&self) -> Option<Compound> {
        let mut tag = horizon_world::worldgen::structure::legacy::piece_tag(
            self.id,
            self.bounding_box(),
            None,
            0,
        );
        self.piece.add_additional_save_data(&mut tag);
        Some(tag)
    }
    fn post_process(
        &self,
        context: &mut horizon_world::worldgen::structure::legacy::PieceContext<'_>,
    ) -> std::result::Result<(), PlacementQueryError> {
        let mut context = Placing::new(context);
        self.piece
            .post_process(&mut context)
            .map_err(|e| PlacementQueryError::Failed(e.0))
    }
    /// The engine runs the first piece's `afterPlace`; it is the type's
    /// [`StructureType::after_place`] the call reaches, with every piece's
    /// box — the piece itself has no `after_place`.
    fn after_place(
        &self,
        context: &mut horizon_world::worldgen::structure::legacy::PieceContext<'_>,
        pieces: &[BoundingBox],
    ) -> std::result::Result<(), PlacementQueryError> {
        let mut context = Placing::new(context);
        self.ty
            .after_place(&mut context, pieces)
            .map_err(|e| PlacementQueryError::Failed(e.0))
    }
}
impl std::fmt::Debug for ModPiece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModPiece")
            .field("id", &self.id)
            .field("piece", &self.piece)
            .finish()
    }
}

impl Adapter {
    /// Wrap `ty` as the engine's type, under the namespaced `id`.
    pub(crate) fn new(id: &str, ty: Arc<dyn StructureType>) -> Self {
        Self {
            inner: ty,
            id: Box::leak(id.to_owned().into_boxed_str()),
        }
    }
    /// The engine's type behind an `Arc`, for the registration table.
    pub(crate) fn into_engine_type(self) -> Arc<dyn ModStructureType> {
        Arc::new(self)
    }
}

/// What the game knows about a registered structure type: its namespaced id
/// (`modid:tower`), which a data pack's `"type"` names, and its engine side.
pub(crate) struct StructureTypeInfo {
    pub(crate) id: Box<str>,
    pub(crate) ty: Arc<dyn ModStructureType>,
}

/// The structure types the mods registered at startup, as the engine's
/// `StructureRegistry::new` reads them: `modid:tower` → type. Empty before
/// the executable installs them into the engine.
pub fn installed_structure_types()
-> horizon_world::worldgen::structure::mod_types::ModStructureTypes {
    super::part_table::installed_structure_type_list()
        .iter()
        .map(|info| (info.id.to_string(), info.ty.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_world::worldgen::structure::legacy::Result as PieceResult;
    use mod_api::Tag;

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
                    ChunkPos::new(3, 7),
                    &Arc::new(serde_json::json!({"type": "testmod:boxes"})),
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
                ChunkPos::new(0, 0),
                &Arc::new(serde_json::json!({})),
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
        use horizon_world::worldgen::structure::legacy::{
            PieceContext as EngineContext, SolidRender,
        };
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
                ChunkPos::new(1, 1),
                &Arc::new(serde_json::json!({})),
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
        fn base_height(&mut self, _: i32, _: i32, _: HeightmapType) -> i32 {
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
        fn base_column(&mut self, _: i32, _: i32, _: HeightmapType) -> Vec<bool> {
            Vec::new()
        }
        fn base_columns(&mut self, _: i32, _: i32, _: &[HeightmapType]) -> Vec<Vec<bool>> {
            Vec::new()
        }
        fn noise_column(&mut self, _: i32, _: i32) -> NoiseColumn {
            NoiseColumn::default()
        }
        fn cold_enough_to_snow(&mut self, _: BiomeId, _: [i32; 3]) -> bool {
            false
        }
        fn generation(&self) -> horizon_world::worldgen::anchor::WorldGenerationContext {
            horizon_world::worldgen::anchor::WorldGenerationContext::new(-64, 384, 63)
        }
        fn noise_biome(&mut self, _: [i32; 3]) -> BiomeId {
            BiomeId(0)
        }
        fn noise_biomes(&mut self, quarts: &[[i32; 3]]) -> Vec<BiomeId> {
            vec![BiomeId(0); quarts.len()]
        }
        fn set_valid_biomes(&mut self, _: Option<&std::collections::BTreeSet<BiomeId>>) {}
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
        static RESOURCES: std::sync::OnceLock<
            horizon_world::worldgen::structure::StructureResources,
        > = std::sync::OnceLock::new();
        RESOURCES.get_or_init(|| {
            let rows: Vec<_> = ["minecraft:air", "minecraft:stone"]
                .into_iter()
                .map(|name| {
                    serde_json::json!([name, name.ends_with("air"), false, "",
                        {"id": name, "properties": {}}])
                })
                .collect();
            let facts = serde_json::json!({
                "states": rows,
                "defaults": {"minecraft:air": 0, "minecraft:stone": 1}
            });
            let registry = horizon_world::registry::BlockRegistry::from_facts_json(
                facts.to_string().as_bytes(),
            )
            .unwrap();
            horizon_world::worldgen::structure::StructureResources::new(
                Box::leak(Box::new(registry)),
                horizon_world::worldgen::structure::StateTransforms::default(),
                std::path::Path::new("/nonexistent"),
                std::collections::BTreeMap::new(),
                &std::collections::BTreeMap::new(),
            )
        })
    }

    /// `placeBlock`'s clipping: a write outside `chunk_box` answers `false`
    /// and touches nothing, silently, as vanilla's `placeBlock` skips it; one
    /// inside writes.
    #[test]
    fn a_write_outside_the_chunk_box_is_dropped_silently() {
        use horizon_world::worldgen::structure::legacy::{
            PieceContext as EngineContext, SolidRender,
        };
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
            let mut context = Placing::new(&mut engine);
            let stone = context.state("stone");
            // Outside on every axis: dropped, silently, nothing recorded.
            for outside in [[16, 64, 0], [0, 64, 16], [-1, 64, 0], [0, 64, -1]] {
                assert!(!context.set_block(outside, stone));
            }
            // Inside: written.
            assert!(context.set_block([15, 64, 15], stone));
        }
        assert_eq!(world.writes, [[15, 64, 15]]);
    }
}
