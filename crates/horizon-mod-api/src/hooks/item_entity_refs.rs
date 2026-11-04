//! The `item_entity_refs` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "both",
        target = "horizon_client::game::native_item_entity_refs::Game<'_>::native_item_entity_ref"
    )]
    fn item_entity();
    #[helper(
        side = "server",
        receiver = "item_entity",
        target = "horizon_client::game::native_item_entity_refs::Game<'_>::native_item_entity_stack"
    )]
    fn item_entity_stack();
    #[helper(
        side = "server",
        receiver = "item_entity",
        target = "horizon_client::game::native_item_entity_refs::Game<'_>::native_item_entity_set_stack"
    )]
    fn item_entity_set_stack();
    #[helper(
        side = "server",
        receiver = "item_entity",
        target = "horizon_client::game::native_item_entity_refs::Game<'_>::native_item_entity_pickup_delay"
    )]
    fn item_entity_pickup_delay();
    #[helper(
        side = "server",
        receiver = "item_entity",
        target = "horizon_client::game::native_item_entity_refs::Game<'_>::native_item_entity_set_pickup_delay"
    )]
    fn item_entity_set_pickup_delay();
    #[helper(
        side = "server",
        receiver = "item_entity",
        target = "horizon_client::game::native_item_entity_refs::Game<'_>::native_item_entity_set_default_pickup_delay"
    )]
    fn item_entity_set_default_pickup_delay();
    #[helper(
        side = "server",
        receiver = "item_entity",
        target = "horizon_client::game::native_item_entity_refs::Game<'_>::native_item_entity_set_no_pickup_delay"
    )]
    fn item_entity_set_no_pickup_delay();
    #[helper(
        side = "server",
        receiver = "item_entity",
        target = "horizon_client::game::native_item_entity_refs::Game<'_>::native_item_entity_set_never_pickup"
    )]
    fn item_entity_set_never_pickup();
    #[helper(
        side = "server",
        receiver = "item_entity",
        target = "horizon_client::game::native_item_entity_refs::Game<'_>::native_item_entity_age"
    )]
    fn item_entity_age();
    #[helper(
        side = "server",
        receiver = "item_entity",
        target = "horizon_client::game::native_item_entity_refs::Game<'_>::native_item_entity_target"
    )]
    fn item_entity_target();
    #[helper(
        side = "server",
        receiver = "item_entity",
        target = "horizon_client::game::native_item_entity_refs::Game<'_>::native_item_entity_set_target"
    )]
    fn item_entity_set_target();
    #[helper(
        side = "server",
        receiver = "item_entity",
        target = "horizon_client::game::native_item_entity_refs::Game<'_>::native_item_entity_owner"
    )]
    fn item_entity_owner();
}
