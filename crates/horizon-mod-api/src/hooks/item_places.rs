//! The `item_places` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::native_item_places::Game<'_>::native_game_stack"
    )]
    fn game_stack();
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::native_item_places::Game<'_>::native_game_insert"
    )]
    fn game_insert();
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::native_item_places::Game<'_>::native_game_extract"
    )]
    fn game_extract();
    #[helper(
        side = "both",
        backend = true,
        target = "horizon_client::game::native_item_container::Game<'_>::native_item_container_insert_slot"
    )]
    fn item_container_insert_slot();
    #[helper(
        side = "both",
        backend = true,
        target = "horizon_client::game::native_item_container::Game<'_>::native_item_container_capacity"
    )]
    fn item_container_capacity();
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::native_item_places::Game<'_>::native_entity_container_size"
    )]
    fn entity_container_size();
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::native_item_places::Game<'_>::native_entity_automation_size"
    )]
    fn entity_automation_size();
    #[helper(
        side = "both",
        receiver = "player",
        target = "horizon_client::game::native_server::Game<'_>::native_player_selected_slot"
    )]
    fn player_selected_slot();
    #[helper(
        side = "both",
        target = "horizon_client::game::native_item_places::Game<'_>::native_entity_type_in_tag"
    )]
    fn entity_type_in_tag();
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::native_item_places::Game<'_>::native_drop_at"
    )]
    fn drop_at();
}
