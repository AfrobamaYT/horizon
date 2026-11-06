//! The `player_stats` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_stats::Game<'_>::native_player_stat"
    )]
    fn player_stat();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_stats::Game<'_>::native_player_award_stat"
    )]
    fn player_award_stat();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_stats::Game<'_>::native_player_reset_stat"
    )]
    fn player_reset_stat();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_stats::Game<'_>::native_player_has_discovered_recipe"
    )]
    fn player_has_discovered_recipe();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_stats::Game<'_>::native_player_award_recipes"
    )]
    fn player_award_recipes();
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_stats::Game<'_>::native_player_reset_recipes"
    )]
    fn player_reset_recipes();
    /// The recipes in the player's recipe book (`ServerRecipeBook.recipes`), as qualified ids in
    /// order (`minecraft:oak_planks`). None for a player it can't reach. Server-side.
    #[notes = r#"## Recipe book view

`PlayerRef::discovered_recipes(host)` lists every recipe the player's book knows, the whole set
`has_discovered_recipe` asks about one at a time. It reads a guest's book from outside their turn
as it reads the host's, and changes only through `award_recipes` and `reset_recipes`."#]
    #[helper(
        side = "server",
        receiver = "player",
        target = "horizon_client::game::native_player_stats::Game<'_>::native_player_discovered_recipes"
    )]
    fn player_discovered_recipes();
}
