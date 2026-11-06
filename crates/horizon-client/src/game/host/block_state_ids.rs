//! The game's side of the `block_state_ids` area (`horizon-mod-api/src/host/block_state_ids.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl BlockStateIdsHost for Game<'_> {
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
impl BlockStateIdsBackend for Game<'_> {
    fn block_key_id(&self, key: BlockKey) -> String {
        mod_api::__private::check_access("block_key_id", mod_api::__private::LoadAccess::WorldOnly);
        self.native_block_key_id(key)
    }
    fn item_key_id(&self, key: ItemKey) -> String {
        mod_api::__private::check_access("item_key_id", mod_api::__private::LoadAccess::WorldOnly);
        self.native_item_key_id(key)
    }
    fn effect_key_id(&self, key: EffectKey) -> String {
        mod_api::__private::check_access(
            "effect_key_id",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_effect_key_id(key)
    }
    fn block_entity_type_key_id(&self, key: BlockEntityTypeKey) -> String {
        mod_api::__private::check_access(
            "block_entity_type_key_id",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_block_entity_type_key_id(key)
    }
}
