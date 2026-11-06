//! The game's side of the `recipes` area (`horizon-mod-api/src/host/recipes.rs`).
use crate::game::Game;
use mod_api::{prelude::*, *};
use std::sync::Arc;
impl RecipesHost for Game<'_> {
    fn recipes_of_type(&self, kind: &str) -> Arc<[Recipe]> {
        mod_api::__private::check_access(
            "recipes_of_type",
            mod_api::__private::LoadAccess::PublishedResources,
        );
        self.native_recipes_of_type(kind)
    }
    fn recipe_ingredient(&self, json: &serde_json::Value) -> Option<Ingredient> {
        mod_api::__private::check_access(
            "recipe_ingredient",
            mod_api::__private::LoadAccess::PublishedResources,
        );
        self.native_recipe_ingredient(json)
    }
    fn recipe_result(&self, json: &serde_json::Value) -> Option<ItemStack> {
        mod_api::__private::check_access(
            "recipe_result",
            mod_api::__private::LoadAccess::PublishedResources,
        );
        self.native_recipe_result(json)
    }
    fn known_recipes(&mut self) -> Arc<[i32]> {
        mod_api::__private::check_access(
            "known_recipes",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_known_recipes()
    }
    fn recipe_book_category(&mut self, kind: &str) -> Option<RecipeBookCategory> {
        mod_api::__private::check_access(
            "recipe_book_category",
            mod_api::__private::LoadAccess::PublishedResources,
        );
        self.native_recipe_book_category(kind)
    }
}
impl RecipesClientHost for Game<'_> {
    fn panel_items(&mut self, mod_id: &str, items: Vec<NativeStack>) {
        mod_api::__private::check_access("panel_items", mod_api::__private::LoadAccess::WorldOnly);
        self.native_panel_items(mod_id, items)
    }
    fn place_recipe(&mut self, book_id: i32, all: bool) -> Result<(), String> {
        mod_api::__private::check_access("place_recipe", mod_api::__private::LoadAccess::WorldOnly);
        self.native_place_recipe(book_id, all)
    }
}
impl RecipesBackend for Game<'_> {
    fn recipe_ingredient_test(&self, ingredient: &Ingredient, stack: &ItemStack) -> bool {
        mod_api::__private::check_access(
            "recipe_ingredient_test",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_recipe_ingredient_test(ingredient, stack)
    }
}
