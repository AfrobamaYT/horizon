//! The game's side of the `item_components` area (`horizon-mod-api/src/host/item_components.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
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
}
impl ItemComponentsBackend for Game<'_> {
    fn item_has_enchantment_effect(&self, stack: &ItemStack, effect: &str) -> bool {
        mod_api::__private::check_access(
            "item_has_enchantment_effect",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_item_has_enchantment_effect(stack, effect)
    }
}
