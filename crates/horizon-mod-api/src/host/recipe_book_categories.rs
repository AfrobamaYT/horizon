//! The `recipe_book_categories` area: host methods the game implements (`game/host/recipe_book_categories.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `recipe_book_categories` area, implemented by the game and reached through [`Host`].
pub trait RecipeBookCategoriesHost {
    /// Side: both.
    /// Published asset/data read: available in Setup, Reloading and World.
    /// The validated category and icon of a custom recipe type, or None for
    /// an undeclared/invalid type, before resources load, or outside a world.
    /// Rebuilt when the recipe table changes. Does not unlock any recipes.
    fn recipe_book_category(&mut self, kind: &str) -> Option<RecipeBookCategory> {
        crate::phase::unprovided(
            "Host::recipe_book_category",
            "recipe_book_category",
            crate::phase::LoadAccess::PublishedResources,
        )
    }
}
