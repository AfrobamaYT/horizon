//! The `block_state_ids` area: host methods the game implements (`game/host/block_state_ids.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `block_state_ids` area, implemented by the game and reached through [`Host`].
pub trait BlockStateIdsHost {
    /// Side: both.
    /// Resolve a registered fluid identifier.
    fn fluid_key(&self, id: &str) -> Option<FluidKey> {
        crate::phase::unprovided(
            "Host::fluid_key",
            "fluid_key",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The state `name` names, read as `/setblock` reads a block
    /// (`BlockStateParser.parseForBlock`): `minecraft:oak_stairs[facing=east]`,
    /// `stone` for `minecraft:stone`, the properties left out at their
    /// default. Its id is vanilla's `Block.getId`. None for an unknown block,
    /// a property it doesn't have or a value it can't take, or block NBT
    /// (`{...}`); and, as `/setblock`'s argument, for anything after the
    /// block.
    fn block_state_id(&self, name: &str) -> Option<BlockStateId> {
        crate::phase::unprovided(
            "Host::block_state_id",
            "block_state_id",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The state `state` as `/setblock` takes it back
    /// (`BlockStateParser.serialize`): its block, then each property in
    /// name order, `minecraft:oak_stairs[facing=east,half=bottom,...]`, or
    /// the block alone for one with no properties. None for an id no state
    /// has (`Block.stateById` gives air for one).
    fn block_state_string(&self, state: BlockStateId) -> Option<String> {
        crate::phase::unprovided(
            "Host::block_state_string",
            "block_state_string",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The default state of `block` (`minecraft:oak_stairs`, or `oak_stairs`):
    /// `Block.defaultBlockState`, the one a block takes when nothing else
    /// says. None for an unknown block.
    fn block_state_default(&self, block: BlockKey) -> Option<BlockStateId> {
        crate::phase::unprovided(
            "Host::block_state_default",
            "block_state_default",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Whether `stack` is in the item tag `tag` (`minecraft:logs` or `logs`,
    /// no `#`):
    /// `ItemStack.is(TagKey)`, over the world's merged tags, its data packs
    /// and content mods included. An empty stack tests `minecraft:air`, an
    /// unknown tag is false. None on a server's client or before the world
    /// has loaded its tags.
    fn item_stack_in_tag(&self, stack: &ItemStack, tag: &str) -> Option<bool> {
        crate::phase::unprovided(
            "Host::item_stack_in_tag",
            "item_stack_in_tag",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Item.getCraftingRemainder`: Horizon currently stores the remainder's
    /// item id, so the result is a single plain stack.
    fn item_crafting_remainder(&self, stack: &ItemStack) -> Option<ItemStack> {
        crate::phase::unprovided(
            "Host::item_crafting_remainder",
            "item_crafting_remainder",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Block.asItem`: the block's item name, including air for a block with
    /// no item; None if the block name is unknown.
    fn block_item(&self, block: BlockKey) -> Option<String> {
        crate::phase::unprovided(
            "Host::block_item",
            "block_item",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' reads of the `block_state_ids` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait BlockStateIdsBackend {
    /// Side: both.
    /// The canonical block id retained by the load-owned registration slot. An invalid slot
    /// is a boundary error, including when a wasm guest supplies it.
    fn block_key_id(&self, key: BlockKey) -> String {
        crate::phase::unprovided(
            "Backend::block_key_id",
            "block_key_id",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The canonical item id retained by the load-owned registration slot.
    fn item_key_id(&self, key: ItemKey) -> String {
        crate::phase::unprovided(
            "Backend::item_key_id",
            "item_key_id",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The canonical effect id retained by the load-owned registration slot.
    fn effect_key_id(&self, key: EffectKey) -> String {
        crate::phase::unprovided(
            "Backend::effect_key_id",
            "effect_key_id",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The canonical block entity type id retained by the load-owned slot.
    fn block_entity_type_key_id(&self, key: BlockEntityTypeKey) -> String {
        crate::phase::unprovided(
            "Backend::block_entity_type_key_id",
            "block_entity_type_key_id",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
