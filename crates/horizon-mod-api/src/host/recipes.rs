//! The `recipes` area: host methods the game implements (`game/host/recipes.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `recipes` area, implemented by the game and reached through [`Host`].
pub trait RecipesHost {
    /// Side: both.
    /// Published asset/data read: available in Setup, Reloading and World.
    /// The recipes of type `kind` (`minecraft:smelting`, `mymod:crusher`), vanilla's
    /// types and mods' own alike, from `data/<namespace>/recipe/*.json` of vanilla,
    /// the content mods (`.hmod`) and the world's data packs, in file order: each
    /// one typed (`Recipe`: its ingredients, results, group, cooking time and
    /// experience, or its stonecutting, smithing or special crafting rule),
    /// with its file as JSON for a mod's own type. Dynamic rules are not fixed
    /// outputs; use `LevelRef::craft` to assemble a crafting grid. These are the
    /// recipes in force (`RecipeManager.getAllRecipesFor`): data packs merged and
    /// load conditions held, so a pack's override or removal shows. A data pack
    /// changes them on `/reload`, so look them up again when you need them. Empty
    /// for a type nobody uses and outside a world. These are the server's table on
    /// the server thread, a dedicated server or a LAN host; a client, the
    /// single-player window included, sees vanilla's and the content mods' recipes,
    /// not the world's data packs', unless a mod had the server send it the type
    /// (`native_datapack_sync`).
    fn recipes_of_type(&self, kind: &str) -> Arc<[Recipe]> {
        crate::phase::unprovided(
            "Host::recipes_of_type",
            "recipes_of_type",
            crate::phase::LoadAccess::PublishedResources,
        )
    }
    /// Side: both.
    /// Published asset/data read: available in Setup, Reloading and World.
    /// An ingredient as a recipe's JSON names it: an item, a `#tag`, or a list
    /// of them, a bare id as `minecraft:` (as vanilla reads it); `test` says
    /// whether a stack fits. Items the game doesn't know are left out; None
    /// when nothing is left (an unknown or empty tag, for one) and outside a
    /// world.
    fn recipe_ingredient(&self, json: &serde_json::Value) -> Option<Ingredient> {
        crate::phase::unprovided(
            "Host::recipe_ingredient",
            "recipe_ingredient",
            crate::phase::LoadAccess::PublishedResources,
        )
    }
    /// Side: both.
    /// Published asset/data read: available in Setup, Reloading and World.
    /// A stack as a recipe's `result` names it: `id` (a bare one as
    /// `minecraft:`), `count` (1 by default, up to 99) and `components`. None
    /// without an `id`, for an item the game doesn't know or a count outside
    /// that, and outside a world.
    fn recipe_result(&self, json: &serde_json::Value) -> Option<ItemStack> {
        crate::phase::unprovided(
            "Host::recipe_result",
            "recipe_result",
            crate::phase::LoadAccess::PublishedResources,
        )
    }
    /// Side: both.
    /// The `book_id`s of the recipes the player's recipe book holds now
    /// (the server's book on a server); empty outside a world.
    fn known_recipes(&mut self) -> Arc<[i32]> {
        crate::phase::unprovided(
            "Host::known_recipes",
            "known_recipes",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
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
/// The client host's methods of the `recipes` area, implemented by the game and reached through [`crate::ClientHost`].
pub trait RecipesClientHost {
    /// Side: client.
    /// Shows the panel beside container screens with these items (at most
    /// 8192), as the game's own slots in pages, with a search box; the
    /// player's clicks, searches and keys over items come back as
    /// `native_panel_event`. The panel is the last mod's to show one; empty
    /// items and no recipe page take yours away.
    fn panel_items(&mut self, mod_id: &str, items: Vec<NativeStack>) {
        crate::phase::unprovided(
            "ClientHost::panel_items",
            "panel_items",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// Places a recipe in the open screen's grid as a click in the recipe
    /// book does (`ServerboundPlaceRecipePacket`): the inventory's grid or a
    /// crafting table's. `all` places as many as the items allow, as with
    /// Shift. Refused while no such screen is open and for a recipe the
    /// player's book doesn't hold; one missing items shows as a ghost.
    fn place_recipe(&mut self, book_id: i32, all: bool) -> Result<(), String> {
        crate::phase::unprovided(
            "ClientHost::place_recipe",
            "place_recipe",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' reads of the `recipes` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait RecipesBackend {
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
