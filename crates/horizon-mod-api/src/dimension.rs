//! Dimensions a native mod registers: a level with its own build height and
//! generator, as [`register_dimension`](super::Registrar::register_dimension)
//! hands it over once at startup. Data packs make dimensions with
//! `data/<ns>/dimension` and `dimension_type` files; a registered one becomes
//! those same two entries, below every data pack, so a pack can replace it.

/// The generator every level has without a mod's own: vanilla's
/// `NoiseBasedChunkGenerator`.
pub(crate) const NOISE_GENERATOR: &str = "minecraft:noise";

/// What a mod's dimension is made of. The default is the Overworld's shape
/// and terrain over plains; the parts of a dimension type not named here are
/// the Overworld's.
#[derive(Clone, Debug, PartialEq, Eq)]
// exhaustive: mods build it themselves as register_dimension's settings
pub struct DimensionSettings {
    /// The lowest block's y: a multiple of 16, from -2032.
    pub min_y: i32,
    /// The build height: a multiple of 16, at least 16, reaching no higher than 2031.
    pub height: i32,
    /// How far up portals and chorus fruit reach, at most `height`.
    pub logical_height: i32,
    /// Whether the sky lights the level.
    pub has_skylight: bool,
    /// Whether the level has a bedrock ceiling, as the Nether does.
    pub has_ceiling: bool,
    /// The chunk generator's id: `minecraft:noise`, or one a mod registered
    /// with `register_chunk_generator`, which takes no noise settings.
    pub generator: String,
    /// The noise settings' id (`minecraft:overworld`, or a data pack's).
    pub noise_settings: String,
    /// The one biome the level has.
    pub biome: String,
}

impl Default for DimensionSettings {
    fn default() -> Self {
        Self {
            min_y: -64,
            height: 384,
            logical_height: 384,
            has_skylight: true,
            has_ceiling: false,
            generator: NOISE_GENERATOR.into(),
            noise_settings: "minecraft:overworld".into(),
            biome: "minecraft:plains".into(),
        }
    }
}

/// A registered dimension: its namespaced id (`modid:pocket`), the mod that
/// registered it and its settings.
pub(crate) struct DimensionInfo {
    pub(crate) mod_id: &'static str,
    pub(crate) id: Box<str>,
    pub(crate) settings: DimensionSettings,
}

/// The dimensions registered at startup, for the game's pack-entry adapter.
// hidden: the game's pack-entry adapter reads the registered dimensions; mods declare them through the registrar.
#[doc(hidden)]
pub fn dimension_declarations() -> Vec<(String, DimensionSettings)> {
    super::parts::installed_dimension_list()
        .iter()
        .map(|dimension| (dimension.id.to_string(), dimension.settings.clone()))
        .collect()
}
