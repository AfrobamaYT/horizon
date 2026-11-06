//! The game's side of the `item_container` area (`horizon-mod-api/src/host/item_container.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl ItemContainerBackend for Game<'_> {
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
}
