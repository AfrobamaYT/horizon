//! The game's side of the `item_places` area (`horizon-mod-api/src/host/item_places.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl ItemPlacesHost for Game<'_> {
    fn entity_type_in_tag(&self, entity_type: &str, tag: &str) -> bool {
        mod_api::__private::check_access(
            "entity_type_in_tag",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_entity_type_in_tag(entity_type, tag)
    }
}
impl ItemPlacesBackend for Game<'_> {
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
    fn player_selected_slot(&self, player: PlayerRef) -> Option<usize> {
        mod_api::__private::check_access(
            "player_selected_slot",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_player_selected_slot(player)
    }
}
impl ItemPlacesBackendMut for Game<'_> {
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
