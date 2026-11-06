//! The `block_states` area: host methods the game implements (`game/host/block_states.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `block_states` area, implemented by the game and reached through [`Host`].
pub trait BlockStatesHost {
    /// Side: both.
    /// Read a loaded, light-emitting block. No implicit chunk loading.
    fn lit_block_name(&self, pos: [i32; 3]) -> Option<&str> {
        crate::phase::unprovided(
            "Host::lit_block_name",
            "lit_block_name",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Property `key` of block state `state` (a machine's `facing`, from the
    /// `state` `native_block_entity_submit` gets), if it has one: no lookup in
    /// the world, so cheap each frame.
    fn block_state_property(&self, state: BlockStateId, key: &str) -> Option<&str> {
        crate::phase::unprovided(
            "Host::block_state_property",
            "block_state_property",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
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
/// The server host's methods of the `block_states` area, implemented by the game and reached through [`ServerHost`].
pub trait BlockStatesServerHost {
    /// Side: server.
    /// Sets property `key` of the block at `pos` to `value`, as a furnace's
    /// tick sets its `lit` (`Level.setBlock` with `UPDATE_ALL`: neighbours
    /// and clients are told); true when the block changed, false when the
    /// block has no such property or value, or the property already has
    /// `value` (as `Level.setBlock` for an unchanged state). The block
    /// entity stays.
    fn set_block_property(&mut self, pos: [i32; 3], key: &str, value: &str) -> bool {
        crate::phase::unprovided(
            "ServerHost::set_block_property",
            "set_block_property",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `native_set_block_property` with `Level.setBlock`'s `flags`, as a
    /// brewing stand's tick sets its bottles with `flags::UPDATE_CLIENTS`.
    /// Players get the write when `UPDATE_CLIENTS` is set; false when this
    /// window isn't a server, and in each case `native_set_block_property`
    /// returns false.
    fn set_block_property_with(
        &mut self,
        pos: [i32; 3],
        key: &str,
        value: &str,
        flags: i32,
    ) -> bool {
        crate::phase::unprovided(
            "ServerHost::set_block_property_with",
            "set_block_property_with",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' reads of the `block_states` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait BlockStatesBackend {
    /// Side: both.
    /// `BlockState.canBeReplaced()`: whether placing a block where this state stands
    /// replaces it (air, grass, water, snow layers). None for an unknown state.
    fn block_state_replaceable(&self, state: BlockStateId) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::block_state_replaceable",
            "block_state_replaceable",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `BlockBehaviour.Properties.destroyTime`, as `BlockState.getDestroySpeed` reads it:
    /// stone 1.5, obsidian 50, bedrock -1 (it can't be broken). None for an unknown state.
    fn block_state_destroy_speed(&self, state: BlockStateId) -> Option<f32> {
        crate::phase::unprovided(
            "Backend::block_state_destroy_speed",
            "block_state_destroy_speed",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `BlockState.getSoundType()`: its volume, pitch and the sound events for breaking,
    /// stepping on, placing, hitting and falling on it. None for an unknown state.
    fn block_state_sound(&self, state: BlockStateId) -> Option<BlockSoundType> {
        crate::phase::unprovided(
            "Backend::block_state_sound",
            "block_state_sound",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `BlockState.hasBlockEntity()`. None for an unknown state.
    fn block_state_has_block_entity(&self, state: BlockStateId) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::block_state_has_block_entity",
            "block_state_has_block_entity",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The states of `block` (`StateDefinition.getPossibleStates`), as the consecutive ids the
    /// registry numbers them with. None for an unknown block.
    fn block_states_of(&self, block: BlockKey) -> Option<BlockStateRange> {
        crate::phase::unprovided(
            "Backend::block_states_of",
            "block_states_of",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The properties of a resolved block, copied for the mod as plain data.
    fn block_state_definition(&self, block: BlockKey) -> Option<Vec<StateProperty>> {
        crate::phase::unprovided(
            "Backend::block_state_definition",
            "block_state_definition",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Match against the existing command registry definitions without allocating on the call.
    fn block_state_matches(
        &self,
        state: BlockStateId,
        block: BlockKey,
        constraints: &[StateConstraint],
    ) -> Option<Result<bool, StateMatchError>> {
        crate::phase::unprovided(
            "Backend::block_state_matches",
            "block_state_matches",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
