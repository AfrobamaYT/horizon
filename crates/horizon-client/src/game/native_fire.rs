#![allow(unused_imports)]

use super::native_data_maps::NativeDataMapType;
use super::native_hooks::HitLevel;
use super::{
    Game, GameLevel, LevelDimension, Screen,
    execution_role::ExecutionRole,
    maps::{NativeMapDecoration, NativeMapInfo},
    native_block_entity_draw::BlockEntityDraw,
    native_configs::{NativeConfig, NativeConfigFile},
    native_mob_draw::{NativeMobRender, NativePlayerRender},
    native_panel_recipes::RecipePage,
};
use horizon_assets::item_model::StackView;
use horizon_client::block_contact::Pending;
use horizon_client::creative::Creative;
use horizon_client::{
    atlas_sources::qualified,
    biome::{BiomeColors, SpawnEntry},
    block_entities::Pos,
    chat::ChatSource,
    containers::{
        BlockEntity, MOD_MENUS, ModBlockEntity, ModBlockEntityFields, ModContainer,
        ModEnergyBuffer, ModFluidTanks,
    },
    crafting::{Ingredient, recipe_result},
    extras::Corner,
    extras_hud::Corners,
    fast_hash::FxHashMap,
    first_person::Hand,
    health::{
        Attack, DamageContainer, DamageHooks, DamageLevel, HurtSource, Hurting, Knock, ShieldBlock,
        Victim,
    },
    hud::{HudBatch, rgba},
    item_tooltip::TooltipDisplay,
    items::ItemStack,
    keybinds,
    lan::menu::Slots,
    merchant::MerchantOffer,
    mobs::{Damage, Who},
    mod_list::{self, Installed, NativePage},
    music::{Music, MusicLayer},
    particles::{ParticleOptions, ParticleType},
    text::Text,
};
use horizon_core::{
    geometry::Direction,
    position::BlockPos,
    ticks::{ScheduleResult, TickPriority},
};
use horizon_world::level::{BonemealCheck, PlaceContext};
use horizon_world::{
    consumable::UseAnimation,
    fluid::{FluidKind, FluidState},
    level::{BlockId, LevelAccess, PlacementContext, Ticks},
    nbt::{Compound, Tag},
    registry::{BlockRegistry, BlockStateId},
};
use mod_api::__private::game::IngredientTest;
use mod_api::game::{NativeClock, NativeShake, PhantomSpawnResult, SleepProblem};
use mod_api::{
    self, Answered, BlockPlacement, ClientReloadListeners, EffectTickChange, EnchantmentInfo,
    EntityRef, EquipmentSlot, GameRuleActor, InteractionKey, LivingRef, NativeCamera,
    NativeCommand, NativeDispensed, NativeHandRender, NativeItemDrop, NativeKeyMapping,
    NativeLifecycleContext, NativeNoteBlockChange, NativePlayerFood, NativeProgressType,
    NativeServerPlayer, PathComputationType, Placement, PlayerRef, RecipeBookCategory,
    ScreenOpening, ServerReloadListeners, ServerTickPhase, TicketController, ToolUse, WorldPhase,
};
use mod_api::{
    AttachmentStore, BlockHit, BonemealSource, ChunkRef, FluidStack, GameRuleValue, LandingWrite,
    LegacyRandom, LevelRef, LogicalSide, NativeBlockClick, NativeBlockUpdateFlags,
    NativeInteractionResult, NativeItemClick, NativeTriState, NeighbourUpdate, Suggestion,
    TooltipElement, UsePhase,
};
use mod_api::{
    MobRef, NativeDespawn, NativeDifficulty, NativePermissionLevel, NativeSpawnCheck,
    NativeSpawnReason, NativeSpawner, Recipe,
};
use mod_api::{
    NativeBlockRim, NativeColoredLight, NativeExplosionStart, NativeItemFished, NativeLootDrops,
    NativeMinimap, NativeMod, NativeNoteInstrument, NativePanelEvent, NativePortalShape,
    NativeScreen, NativeScreenView, NativeStack, NativeZoom,
};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use winit::{event::MouseButton, keyboard::KeyCode};

include!(concat!(env!("OUT_DIR"), "/mod_api_fire_generated.rs"));
