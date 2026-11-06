//! The game's side of the `item_entity_refs` area (`horizon-mod-api/src/host/item_entity_refs.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl ItemEntityRefsHost for Game<'_> {
    fn item_entity(&self, entity: EntityRef) -> Option<ItemEntityRef> {
        mod_api::__private::check_access(
            "item_entity",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_entity_ref(entity)
    }
}
impl ItemEntityRefsBackend for Game<'_> {
    fn item_entity_stack(&self, item: ItemEntityRef) -> Option<ItemStack> {
        mod_api::__private::check_access(
            "item_entity_stack",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_entity_stack(item)
    }
    fn item_entity_pickup_delay(&self, item: ItemEntityRef) -> Option<i32> {
        mod_api::__private::check_access(
            "item_entity_pickup_delay",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_entity_pickup_delay(item)
    }
    fn item_entity_age(&self, item: ItemEntityRef) -> Option<i32> {
        mod_api::__private::check_access(
            "item_entity_age",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_entity_age(item)
    }
    fn item_entity_target(&self, item: ItemEntityRef) -> Option<u128> {
        mod_api::__private::check_access(
            "item_entity_target",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_entity_target(item)
    }
    fn item_entity_owner(&self, item: ItemEntityRef) -> Option<EntityRef> {
        mod_api::__private::check_access(
            "item_entity_owner",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_entity_owner(item)
    }
}
impl ItemEntityRefsBackendMut for Game<'_> {
    fn item_entity_set_stack(&mut self, item: ItemEntityRef, stack: ItemStack) -> bool {
        mod_api::__private::check_access(
            "item_entity_set_stack",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_entity_set_stack(item, stack)
    }
    fn item_entity_set_pickup_delay(&mut self, item: ItemEntityRef, ticks: i32) -> bool {
        mod_api::__private::check_access(
            "item_entity_set_pickup_delay",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_entity_set_pickup_delay(item, ticks)
    }
    fn item_entity_set_default_pickup_delay(&mut self, item: ItemEntityRef) -> bool {
        mod_api::__private::check_access(
            "item_entity_set_default_pickup_delay",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_entity_set_default_pickup_delay(item)
    }
    fn item_entity_set_no_pickup_delay(&mut self, item: ItemEntityRef) -> bool {
        mod_api::__private::check_access(
            "item_entity_set_no_pickup_delay",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_entity_set_no_pickup_delay(item)
    }
    fn item_entity_set_never_pickup(&mut self, item: ItemEntityRef) -> bool {
        mod_api::__private::check_access(
            "item_entity_set_never_pickup",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_entity_set_never_pickup(item)
    }
    fn item_entity_set_target(
        &mut self,
        item: ItemEntityRef,
        target: Option<u128>,
    ) -> bool {
        mod_api::__private::check_access(
            "item_entity_set_target",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_entity_set_target(item, target)
    }
}
