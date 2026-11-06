//! The `level_craft` area: host methods the game implements (`game/host/level_craft.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `level_craft` area, implemented by the game and reached through [`Backend`].
#[doc(hidden)]
pub trait LevelCraftBackend {
    /// Side: server.
    /// `RecipeManager.getRecipeFor(CRAFTING, CraftingInput.of(width, height, items), level)`,
    /// then the recipe's `assemble` and `getRemainingItems`: what a `width` by `height` grid
    /// (each 1 to 3, `items` row by row) makes, through the crafting table's matcher, with the
    /// world's data packs' recipes and the special recipes. None when no recipe matches or it
    /// makes nothing, for a grid of another size, and off the server.
    fn level_craft(
        &self,
        level: LevelRef,
        width: u32,
        height: u32,
        items: &[Option<ItemStack>],
    ) -> Option<CraftResult> {
        crate::phase::unprovided(
            "Backend::level_craft",
            "level_craft",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
