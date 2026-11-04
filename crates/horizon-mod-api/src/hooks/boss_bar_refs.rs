//! The `boss_bar_refs` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar"
    )]
    fn boss_bar();
    #[helper(
        side = "server",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bars"
    )]
    fn boss_bars();
    #[helper(
        side = "server",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_create_boss_bar"
    )]
    fn create_boss_bar();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_id"
    )]
    fn boss_bar_id();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_name"
    )]
    fn boss_bar_name();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_set_name"
    )]
    fn boss_bar_set_name();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_color"
    )]
    fn boss_bar_color();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_set_color"
    )]
    fn boss_bar_set_color();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_overlay"
    )]
    fn boss_bar_overlay();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_set_overlay"
    )]
    fn boss_bar_set_overlay();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_darken_screen"
    )]
    fn boss_bar_darken_screen();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_set_darken_screen"
    )]
    fn boss_bar_set_darken_screen();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_play_boss_music"
    )]
    fn boss_bar_play_boss_music();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_set_play_boss_music"
    )]
    fn boss_bar_set_play_boss_music();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_create_world_fog"
    )]
    fn boss_bar_create_world_fog();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_set_create_world_fog"
    )]
    fn boss_bar_set_create_world_fog();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_value"
    )]
    fn boss_bar_value();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_set_value"
    )]
    fn boss_bar_set_value();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_max"
    )]
    fn boss_bar_max();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_set_max"
    )]
    fn boss_bar_set_max();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_visible"
    )]
    fn boss_bar_visible();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_set_visible"
    )]
    fn boss_bar_set_visible();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_players"
    )]
    fn boss_bar_players();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_add_player"
    )]
    fn boss_bar_add_player();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_remove_player"
    )]
    fn boss_bar_remove_player();
    #[helper(
        side = "server",
        receiver = "boss_bar",
        target = "horizon_client::game::native_boss_bar_refs::Game<'_>::native_boss_bar_remove"
    )]
    fn boss_bar_remove();
}
