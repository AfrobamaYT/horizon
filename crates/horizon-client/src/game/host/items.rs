//! The game's side of the `items` area (`horizon-mod-api/src/host/items.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl ItemsHost for Game<'_> {
    fn max_stack_size(&self, item: &str) -> Option<u32> {
        mod_api::__private::check_access(
            "max_stack_size",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_max_stack_size(item)
    }
    fn item_default_component(&self, item: &str, component: &str) -> Option<Tag> {
        mod_api::__private::check_access(
            "item_default_component",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_default_component(item, component)
    }
    fn item_burn_time(&self, item: &str) -> Option<i32> {
        mod_api::__private::check_access(
            "item_burn_time",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_burn_time(item)
    }
    fn item_compost_chance(&self, item: &str) -> Option<f32> {
        mod_api::__private::check_access(
            "item_compost_chance",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_compost_chance(item)
    }
    fn enchantments_compatible(&self, a: &str, b: &str) -> bool {
        mod_api::__private::check_access(
            "enchantments_compatible",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_enchantments_compatible(a, b)
    }
    fn entity_type_in_tag(&self, entity_type: &str, tag: &str) -> bool {
        mod_api::__private::check_access(
            "entity_type_in_tag",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_type_in_tag(entity_type, tag)
    }
}
impl ItemsServerHost for Game<'_> {
    fn dispense_default(
        &mut self,
        dimension: &str,
        pos: [i32; 3],
        facing: Direction,
        stack: ItemStack,
    ) -> Option<ItemStack> {
        mod_api::__private::check_access(
            "dispense_default",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_dispense_default(dimension, pos, facing, stack)
    }
}
impl ItemsClientHost for Game<'_> {
    fn give(&mut self, item: &str, count: u32) -> Result<(), String> {
        mod_api::__private::check_access("give", mod_api::__private::LoadAccess::WorldOnly);
        self.native_give(item, count)
    }
    fn enchantments(&self) -> Vec<EnchantmentInfo> {
        mod_api::__private::check_access(
            "enchantments",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_enchantments()
    }
    fn enchantment_min_cost(&self, id: &str, level: u32) -> Option<i32> {
        mod_api::__private::check_access(
            "enchantment_min_cost",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_enchantment_min_cost(id, level)
    }
    fn enchantment_supports(&self, id: &str, item: &str) -> bool {
        mod_api::__private::check_access(
            "enchantment_supports",
            mod_api::__private::LoadAccess::ClientSetup,
        );
        self.native_enchantment_supports(id, item)
    }
}
impl ItemsBackend for Game<'_> {
    fn container_size(&self, at: BlockSide) -> Option<usize> {
        mod_api::__private::check_access(
            "container_size",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_container_size(at)
    }
    fn container_stack(&self, at: BlockSide, slot: usize) -> Option<ItemStack> {
        mod_api::__private::check_access(
            "container_stack",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_container_stack(at, slot)
    }
    fn container_capacity(&self, at: BlockSide, slot: usize, stack: &ItemStack) -> u32 {
        mod_api::__private::check_access(
            "container_capacity",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_container_capacity(at, slot, stack)
    }
    fn item_has_enchantment_effect(&self, stack: &ItemStack, effect: &str) -> bool {
        mod_api::__private::check_access(
            "item_has_enchantment_effect",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_has_enchantment_effect(stack, effect)
    }
    fn item_equippable_slot(&self, stack: &ItemStack) -> Option<String> {
        mod_api::__private::check_access(
            "item_equippable_slot",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_equippable_slot(stack)
    }
    fn item_container_slots(&self, stack: &ItemStack) -> Option<Vec<Option<ItemStack>>> {
        mod_api::__private::check_access(
            "item_container_slots",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_container_slots(stack)
    }
    fn item_container_insert(
        &self,
        stack: &ItemStack,
        item: &ItemStack,
    ) -> Option<(u32, ItemStack)> {
        mod_api::__private::check_access(
            "item_container_insert",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_container_insert(stack, item)
    }
    fn item_container_extract(
        &self,
        stack: &ItemStack,
        slot: usize,
        max: u32,
    ) -> Option<(Option<ItemStack>, ItemStack)> {
        mod_api::__private::check_access(
            "item_container_extract",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_container_extract(stack, slot, max)
    }
    fn item_container_insert_slot(
        &self,
        stack: &ItemStack,
        slot: usize,
        item: &ItemStack,
    ) -> Option<(u32, ItemStack)> {
        mod_api::__private::check_access(
            "item_container_insert_slot",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_container_insert_slot(stack, slot, item)
    }
    fn item_container_capacity(&self, stack: &ItemStack, slot: usize, item: &ItemStack) -> u32 {
        mod_api::__private::check_access(
            "item_container_capacity",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_container_capacity(stack, slot, item)
    }
    fn entity_container_size(&self, entity: u32) -> Option<usize> {
        mod_api::__private::check_access(
            "entity_container_size",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_container_size(entity)
    }
    fn entity_automation_size(&self, entity: u32) -> Option<usize> {
        mod_api::__private::check_access(
            "entity_automation_size",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_automation_size(entity)
    }
}
impl ItemsBackendMut for Game<'_> {
    fn container_insert(
        &mut self,
        at: BlockSide,
        slot: usize,
        stack: ItemStack,
        tx: &Transaction,
    ) -> Option<ItemStack> {
        mod_api::__private::check_access(
            "container_insert",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_container_insert(at, slot, stack, tx)
    }
    fn container_extract(
        &mut self,
        at: BlockSide,
        slot: usize,
        max: u32,
        tx: &Transaction,
    ) -> Option<ItemStack> {
        mod_api::__private::check_access(
            "container_extract",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_container_extract(at, slot, max, tx)
    }
    fn game_stack(&mut self, place: &ItemPlace) -> Option<ItemStack> {
        mod_api::__private::check_access("game_stack", mod_api::__private::LoadAccess::WorldOnly);
        self.native_game_stack(place)
    }
    fn game_insert(&mut self, place: &ItemPlace, stack: ItemStack, tx: &Transaction) -> u32 {
        mod_api::__private::check_access("game_insert", mod_api::__private::LoadAccess::WorldOnly);
        self.native_game_insert(place, stack, tx)
    }
    fn game_extract(&mut self, place: &ItemPlace, stack: ItemStack, tx: &Transaction) -> u32 {
        mod_api::__private::check_access("game_extract", mod_api::__private::LoadAccess::WorldOnly);
        self.native_game_extract(place, stack, tx)
    }
    fn drop_at(&mut self, pos: [i32; 3], stack: ItemStack) {
        mod_api::__private::check_access("drop_at", mod_api::__private::LoadAccess::WorldOnly);
        self.native_drop_at(pos, stack)
    }
}
