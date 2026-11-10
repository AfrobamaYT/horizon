//! Concrete authoring extracted verbatim from the vocabulary layer.
use super::*;

// no wire: process-local registration and dispatch storage contains trait objects and owning-mod identities.
/// The game's [`Registrar`]: every mod's registrations, checked as they come.
// exhaustive: its fields are private, so mods cannot build or destructure it
pub struct PartRegistrar {
    pub(crate) menu_recipe_books:
        HashMap<String, (crate::RecipeBookTypeKey, String, crate::RecipeBookLayout)>,
    pub(crate) recipe_book_types: HashMap<String, String>,
    pub(crate) game_rule_categories: Vec<String>,
    pub(crate) game_rules: Vec<crate::GameRuleDecl>,
    pub(crate) mod_id: &'static str,
    pub(crate) mod_index: usize,
    /// Parts registered so far, by every mod: the next part's `order`.
    pub(crate) registered: usize,
    pub(crate) parts: Parts,
    pub(crate) tags: TagParts,
    pub(crate) block_entity_types: Vec<BlockEntityTypeDecl>,
    pub(crate) entity_attributes: Vec<EntityAttributeDeclaration>,
    pub(crate) world_fluids: Vec<(String, crate::WorldFluid)>,
    pub(crate) entity_attribute_modifications: Vec<EntityAttributeModification>,
    pub(crate) attributes: Vec<super::attributes::AttributeDeclaration>,
    pub(crate) spawn_placements: Vec<SpawnPlacementDeclaration>,
    pub(crate) biome_modifiers: Vec<BiomeModifierDeclaration>,
    pub(crate) datapack_registries:
        Vec<crate::world::datapack_registry::DatapackRegistryDeclaration>,
    pub(crate) registry_keys: Rc<RefCell<horizon_mod_runtime::registry_keys::Requests>>,
    pub(crate) effects: OnceLock<horizon_mod_runtime::registry_keys::Effects>,
    pub(crate) controllers: Vec<ControllerInfo>,
    pub(crate) arm_poses: Vec<crate::client::arm_pose::PoseInfo>,
    pub(crate) structure_types: Vec<crate::world::structure_type::StructureTypeInfo>,
    pub(crate) dimensions: Vec<crate::world::dimension::DimensionInfo>,
    pub(crate) dimension_shape: fn(&crate::DimensionSettings) -> bool,
    pub(crate) chunk_generators: Vec<crate::world::chunk_generator::ChunkGeneratorInfo>,
    pub(crate) refused: Vec<String>,
    pub(crate) self_refusal: Option<String>,
}

