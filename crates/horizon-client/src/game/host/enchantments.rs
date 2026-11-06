//! The game's side of the `enchantments` area (`horizon-mod-api/src/host/enchantments.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl EnchantmentsClientHost for Game<'_> {
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
