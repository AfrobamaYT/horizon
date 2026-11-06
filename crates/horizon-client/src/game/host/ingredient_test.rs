//! The game's side of the `ingredient_test` area (`horizon-mod-api/src/host/ingredient_test.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl IngredientTestBackend for Game<'_> {
    fn recipe_ingredient_test(&self, ingredient: &Ingredient, stack: &ItemStack) -> bool {
        mod_api::__private::check_access(
            "recipe_ingredient_test",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_recipe_ingredient_test(ingredient, stack)
    }
}
