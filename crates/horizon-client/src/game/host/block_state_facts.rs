//! The game's side of the `block_state_facts` area (`horizon-mod-api/src/host/block_state_facts.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl BlockStateFactsBackend for Game<'_> {
    fn block_state_replaceable(&self, state: BlockStateId) -> Option<bool> {
        mod_api::__private::check_access(
            "block_state_replaceable",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_state_replaceable(state)
    }
    fn block_state_destroy_speed(&self, state: BlockStateId) -> Option<f32> {
        mod_api::__private::check_access(
            "block_state_destroy_speed",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_state_destroy_speed(state)
    }
    fn block_state_sound(&self, state: BlockStateId) -> Option<BlockSoundType> {
        mod_api::__private::check_access(
            "block_state_sound",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_state_sound(state)
    }
    fn block_state_has_block_entity(&self, state: BlockStateId) -> Option<bool> {
        mod_api::__private::check_access(
            "block_state_has_block_entity",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_state_has_block_entity(state)
    }
    fn block_states_of(&self, block: BlockKey) -> Option<BlockStateRange> {
        mod_api::__private::check_access(
            "block_states_of",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_states_of(block)
    }
    fn block_state_definition(&self, block: BlockKey) -> Option<Vec<StateProperty>> {
        mod_api::__private::check_access(
            "block_state_definition",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_state_definition(block)
    }
    fn block_state_matches(
        &self,
        state: BlockStateId,
        block: BlockKey,
        constraints: &[StateConstraint],
    ) -> Option<Result<bool, StateMatchError>> {
        mod_api::__private::check_access(
            "block_state_matches",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_state_matches(state, block, constraints)
    }
}
