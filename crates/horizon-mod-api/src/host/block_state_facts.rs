//! The `block_state_facts` area: host methods the game implements (`game/host/block_state_facts.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `block_state_facts` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait BlockStateFactsBackend {
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
