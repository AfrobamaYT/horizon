//! The game's side of the `recipe_book_categories` area (`horizon-mod-api/src/host/recipe_book_categories.rs`).
use crate::game::Game;
use mod_api::__private::game::*;
use mod_api::{prelude::*, *};
impl RecipeBookCategoriesHost for Game<'_> {
    fn recipe_book_category(&mut self, kind: &str) -> Option<RecipeBookCategory> {
        mod_api::__private::check_access(
            "recipe_book_category",
            mod_api::__private::LoadAccess::PublishedResources,
        );
        self.native_recipe_book_category(kind)
    }
}
