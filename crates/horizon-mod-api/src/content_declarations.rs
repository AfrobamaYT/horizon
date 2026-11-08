//! Load-time content declarations. These are drafts: the game admits a namespace atomically
//! using the same checks as content JSON files. Registry-dependent validity is not promised
//! by these records; no engine state is written by constructing or editing them.
use horizon_core::arm_pose::ArmPose;
use std::collections::BTreeMap;

/// BlockProperties in a load-time content declaration.
#[derive(Clone, Debug, PartialEq, Default)]
// exhaustive: declaration drafts mirror the closed content schema and support record construction.
pub struct BlockProperties {
    /// Destroy time in seconds of hand mining (-1: unbreakable).
    pub hardness: Option<f32>,
    /// explosion resistance.
    pub explosion_resistance: Option<f32>,
    /// Light emitted, 0 to 15.
    pub light: Option<u8>,
    /// requires tool.
    pub requires_tool: Option<bool>,
    /// friction.
    pub friction: Option<f32>,
    /// A `MapColor` id, 0 to 61.
    pub map_color: Option<u8>,
    /// A vanilla block whose sounds this block uses.
    pub sounds: Option<String>,
    /// Which states tick randomly, whatever the template does (`isRandomlyTicking`).
    pub random_ticks: Option<RandomTicks>,
    /// How the block is drawn, whatever the template does (`BlockBehaviour.getRenderShape`).
    pub render_shape: Option<RenderShape>,
    /// Vanilla's piston push reaction, or the template's when unset.
    pub push_reaction: Option<PushReaction>,
    /// `false`: the block occludes nothing, so its neighbours' faces are drawn and light passes
    /// it (`BlockBehaviour.Properties.noOcclusion`); unset or `true`: as its template.
    pub occludes: Option<bool>,
    /// `false`: the breaking crack (`destroy_stage_0..9`) is not drawn on the block, as for a
    /// block broken by use; unset or `true`, it is drawn. Only the overlay: the break and the
    /// destroy-progress packets are vanilla's.
    pub destroy_overlay: Option<bool>,
}

/// PushReaction in a load-time content declaration.
#[derive(Clone, Debug, PartialEq)]
// exhaustive: these variants are the closed content declaration vocabulary.
pub enum PushReaction {
    /// Declaration option.
    PushPull,
    /// Declaration option.
    Push,
    /// Declaration option.
    Popped,
    /// Declaration option.
    Immoveable,
    /// Declaration option.
    IgnoreEntity,
}

/// RenderShape in a load-time content declaration.
#[derive(Clone, Debug, PartialEq)]
// exhaustive: these variants are the closed content declaration vocabulary.
pub enum RenderShape {
    /// Declaration option.
    Model,
    /// Declaration option.
    EntityblockAnimated,
    /// Declaration option.
    Invisible,
}

/// RandomTicks in a load-time content declaration.
#[derive(Clone, Debug, PartialEq)]
// exhaustive: these variants are the closed content declaration vocabulary.
pub enum RandomTicks {
    /// Every state (`true`), or none (`false`, for a template that ticks).
    All(bool),
    /// The states whose own state property `property` is one of `values`.
    States {
        /// State property name.
        property: String,
        /// Accepted state values.
        values: Vec<String>,
    },
}

/// BlockShapes in a load-time content declaration.
#[derive(Clone, Debug, PartialEq)]
// exhaustive: declaration drafts mirror the closed content schema and support record construction.
pub struct BlockShapes {
    /// properties.
    pub properties: Vec<String>,
    /// values.
    pub values: BTreeMap<String, Vec<[f64; 6]>>,
    /// default.
    pub default: Option<Vec<[f64; 6]>>,
}

/// BlockStateProperty in a load-time content declaration.
#[derive(Clone, Debug, PartialEq)]
// exhaustive: these variants are the closed content declaration vocabulary.
pub enum BlockStateProperty {
    /// `false` and `true`, in vanilla's spelling.
    Boolean,
    /// A list of values, in the order the state space is built in.
    Enum {
        /// Allowed values in state order.
        values: Vec<String>,
    },
}

/// Block in a load-time content declaration.
#[derive(Clone, Debug, PartialEq, Default)]
// exhaustive: declaration drafts mirror the closed content schema and support record construction.
pub struct Block {
    /// `<mod id>:<path>`.
    pub id: String,
    /// The vanilla block it copies, `minecraft:<path>`.
    pub template: String,
    /// properties.
    pub properties: BlockProperties,
    /// shapes.
    pub shapes: Option<BlockShapes>,
    /// collision shapes.
    pub collision_shapes: Option<BlockShapes>,
    /// State properties of its own, which its template does not have.
    pub state_properties: BTreeMap<String, BlockStateProperty>,
    /// State properties overlaid on the template default.
    pub default_state: BTreeMap<String, String>,
    /// Also add a block item with the same id.
    pub item: bool,
    /// The sound event a fence gate plays opening, instead of its wood's
    /// (`WoodType.fenceGateOpen`); needs a `*_fence_gate` template.
    pub open_sound: Option<String>,
    /// The sound event a fence gate plays closing (`WoodType.fenceGateClose`).
    pub close_sound: Option<String>,
    /// The item this block is, for pick-block and `Block.asItem`, when
    /// it is not a block item of its own: an item of this mod or a vanilla
    /// one, checked to exist when the content loads. Needs `item` false.
    pub block_item: Option<String>,
    /// Its item's component patch, as an [`Item`]'s.
    pub components: Vec<(String, Option<crate::Tag>)>,
    /// A connecting block (template `minecraft:chorus_plant`, a `PipeBlock`
    /// without the chorus plant's rules): the block tag, without `#`, whose
    /// blocks it joins besides itself (a cable's machines).
    pub connects_to: Option<String>,
    /// A connecting block's width in pixels (2 to 14): its shapes, the
    /// chorus plant's (10) when unset.
    pub thickness: Option<f32>,
    /// A sign's `WoodType` (template a vanilla sign), which names its editor's
    /// background: `<namespace>:<path>` a mod's texture, a bare name vanilla's.
    /// The template's when unset.
    pub wood_type: Option<String>,
    /// The block a standing or ceiling sign's item places on a wall
    /// (`StandingAndWallBlockItem.wallBlock`): one of this mod's blocks, whose
    /// template is the matching wall sign and which has no item. The
    /// template's when unset.
    pub wall_block: Option<String>,
    /// A sign's new text colour (`SignBlockEntity.createDefaultSignText`): a `DyeColor` name,
    /// black when unset. Applies to a sign when it is placed, front and back.
    pub sign_text_color: Option<String>,
    /// Whether a new sign's text glows, false when unset.
    pub sign_text_glow: Option<bool>,
    /// The block farmland turns into; the template's base when unset.
    pub base_block: Option<String>,
}

/// Rarity in a load-time content declaration.
#[derive(Clone, Debug, PartialEq)]
// exhaustive: these variants are the closed content declaration vocabulary.
pub enum Rarity {
    /// Declaration option.
    Common,
    /// Declaration option.
    Uncommon,
    /// Declaration option.
    Rare,
    /// Declaration option.
    Epic,
}

/// Item in a load-time content declaration.
#[derive(Clone, Debug, PartialEq, Default)]
// exhaustive: declaration drafts mirror the closed content schema and support record construction.
pub struct Item {
    /// id.
    pub id: String,
    /// The vanilla item it copies, `minecraft:<path>`.
    pub template: String,
    /// max stack size.
    pub max_stack_size: Option<u8>,
    /// rarity.
    pub rarity: Option<Rarity>,
    /// A 26.3 data component patch over the template's: `!minecraft:<type>` removes one.
    pub components: Vec<(String, Option<crate::Tag>)>,
    /// What it leaves in the crafting grid (`Item.Properties.craftRemainder`):
    /// a vanilla item or one of the mod's own.
    pub crafting_remainder: Option<String>,
    /// The block placed by this item, if any.
    pub block: Option<String>,
    /// The arm pose a humanoid takes holding it, in place of vanilla's (`ArmPose`).
    pub arm_pose: Option<ArmPose>,
}

impl Block {
    /// Start a declaration with explicitly namespaced identifiers.
    pub fn new(id: crate::Identifier, template: crate::Identifier) -> Self {
        Self {
            id: id.to_string(),
            template: template.to_string(),
            ..Self::default()
        }
    }
}
impl Item {
    /// Start a declaration with explicitly namespaced identifiers.
    pub fn new(id: crate::Identifier, template: crate::Identifier) -> Self {
        Self {
            id: id.to_string(),
            template: template.to_string(),
            ..Self::default()
        }
    }
}
