//! The `item_capability` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::machines::native_items::Game<'_>::native_container_size"
    )]
    fn container_size();
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::machines::native_items::Game<'_>::native_container_stack"
    )]
    fn container_stack();
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::machines::native_items::Game<'_>::native_container_insert"
    )]
    fn container_insert();
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::machines::native_items::Game<'_>::native_container_extract"
    )]
    fn container_extract();
    #[helper(
        side = "server",
        backend = true,
        target = "horizon_client::game::machines::native_items::Game<'_>::native_container_capacity"
    )]
    fn container_capacity();
}
