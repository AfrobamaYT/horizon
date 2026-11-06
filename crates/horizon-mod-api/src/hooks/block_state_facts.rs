//! The `block_state_facts` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "both",
        backend = true,
        target = "horizon_client::game::native_block_states::Game<'_>::native_block_state_replaceable"
    )]
    fn block_state_replaceable();
    #[helper(
        side = "both",
        backend = true,
        target = "horizon_client::game::native_block_states::Game<'_>::native_block_state_destroy_speed"
    )]
    fn block_state_destroy_speed();
    #[helper(
        side = "both",
        backend = true,
        target = "horizon_client::game::native_block_states::Game<'_>::native_block_state_sound"
    )]
    fn block_state_sound();
    #[helper(
        side = "both",
        backend = true,
        target = "horizon_client::game::native_block_states::Game<'_>::native_block_state_has_block_entity"
    )]
    fn block_state_has_block_entity();
    #[helper(
        side = "both",
        backend = true,
        target = "horizon_client::game::native_block_states::Game<'_>::native_block_states_of"
    )]
    fn block_states_of();
    #[helper(
        side = "both",
        backend = true,
        target = "horizon_client::game::native_block_states::Game<'_>::native_block_state_definition"
    )]
    fn block_state_definition();
    #[helper(
        side = "both",
        backend = true,
        target = "horizon_client::game::native_block_states::Game<'_>::native_block_state_matches"
    )]
    fn block_state_matches();
}
