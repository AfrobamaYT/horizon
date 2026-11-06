//! The `ingredient_test` area: host methods the game implements (`game/host/ingredient_test.rs`).
#![allow(unused_variables)]
use crate::*;
/// The handles' reads of the `ingredient_test` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait IngredientTestBackend {
    /// Side: both.
    /// Whether `stack` fits `ingredient` (`Ingredient.test`), matched with this
    /// game's loop context.
    fn recipe_ingredient_test(&self, ingredient: &Ingredient, stack: &ItemStack) -> bool {
        crate::phase::unprovided(
            "Backend::recipe_ingredient_test",
            "recipe_ingredient_test",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
