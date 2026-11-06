//! The game's side of the `level_craft` area (`horizon-mod-api/src/host/level_craft.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use mod_api::__private::game::*;
impl LevelCraftBackend for Game<'_> {
    fn level_craft(
        &self,
        level: LevelRef,
        width: u32,
        height: u32,
        items: &[Option<ItemStack>],
    ) -> Option<CraftResult> {
        mod_api::__private::check_access(
            "level_craft",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_level_craft(level, width, height, items)
    }
}
