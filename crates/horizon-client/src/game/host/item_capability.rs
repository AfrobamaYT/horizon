//! The game's side of the `item_capability` area (`horizon-mod-api/src/host/item_capability.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl ItemCapabilityBackend for Game<'_> {
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
}
impl ItemCapabilityBackendMut for Game<'_> {
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
}
