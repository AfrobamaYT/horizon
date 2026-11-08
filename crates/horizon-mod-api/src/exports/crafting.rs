// hidden: a module of game-side types; mods use their re-exports
#[doc(hidden)]
pub mod crafting;
mod ingredient_parser;
mod __private {
    pub use crate::ingredient_parser::{WalkedIngredient, dedup_keep_order, resolved_items};
}
pub use crate::crafting::{
    Ingredient, IngredientKeysError, Recipe, RecipeInput, RecipeLookupError, RecipeMatch,
    RecipePropertyAddition,
};
