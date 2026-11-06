//! The game's side of the `item_components` area (`horizon-mod-api/src/host/item_components.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
impl ItemComponentsHost for Game<'_> {
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
}
impl ItemComponentsBackend for Game<'_> {
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
}
