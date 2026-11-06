//! The game's side of the `block_states` area (`horizon-mod-api/src/host/block_states.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl BlockStatesHost for Game<'_> {
    fn lit_block_name(&self, pos: [i32; 3]) -> Option<&str> {
        mod_api::__private::check_access(
            "lit_block_name",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_lit_block_name(pos)
    }
    fn block_state_property(&self, state: BlockStateId, key: &str) -> Option<&str> {
        mod_api::__private::check_access(
            "block_state_property",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_state_property(state, key)
    }
    fn fluid_key(&self, id: &str) -> Option<FluidKey> {
        mod_api::__private::check_access("fluid_key", mod_api::__private::LoadAccess::WorldOnly);
        self.native_fluid_key(id)
    }
    fn block_state_id(&self, name: &str) -> Option<BlockStateId> {
        mod_api::__private::check_access(
            "block_state_id",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_state_id(name)
    }
    fn block_state_string(&self, state: BlockStateId) -> Option<String> {
        mod_api::__private::check_access(
            "block_state_string",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_state_string(state)
    }
    fn block_state_default(&self, block: BlockKey) -> Option<BlockStateId> {
        mod_api::__private::check_access(
            "block_state_default",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_state_default(block)
    }
    fn item_stack_in_tag(&self, stack: &ItemStack, tag: &str) -> Option<bool> {
        mod_api::__private::check_access(
            "item_stack_in_tag",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_stack_in_tag(stack, tag)
    }
    fn item_crafting_remainder(&self, stack: &ItemStack) -> Option<ItemStack> {
        mod_api::__private::check_access(
            "item_crafting_remainder",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_crafting_remainder(stack)
    }
    fn block_item(&self, block: BlockKey) -> Option<String> {
        mod_api::__private::check_access("block_item", mod_api::__private::LoadAccess::WorldOnly);
        self.native_block_item(block)
    }
}
impl BlockStatesServerHost for Game<'_> {
    fn set_block_property(&mut self, pos: [i32; 3], key: &str, value: &str) -> bool {
        mod_api::__private::check_access(
            "set_block_property",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_set_block_property(pos, key, value)
    }
    fn set_block_property_with(
        &mut self,
        pos: [i32; 3],
        key: &str,
        value: &str,
        flags: i32,
    ) -> bool {
        mod_api::__private::check_access(
            "set_block_property_with",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_set_block_property_with(pos, key, value, flags)
    }
}
impl BlockStatesBackend for Game<'_> {
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
