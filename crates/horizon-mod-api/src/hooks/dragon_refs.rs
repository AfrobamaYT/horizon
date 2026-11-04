//! The `dragon_refs` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        target = "horizon_client::game::native_dragon_refs::Game<'_>::native_dragon_ref"
    )]
    fn dragon();
    #[helper(
        side = "server",
        receiver = "dragon",
        target = "horizon_client::game::native_dragon_refs::Game<'_>::native_dragon_phase"
    )]
    fn dragon_phase();
    #[helper(
        side = "server",
        receiver = "dragon",
        target = "horizon_client::game::native_dragon_refs::Game<'_>::native_dragon_set_phase"
    )]
    fn dragon_set_phase();
    #[helper(
        side = "server",
        receiver = "dragon",
        target = "horizon_client::game::native_dragon_refs::Game<'_>::native_dragon_alive_crystals"
    )]
    fn dragon_alive_crystals();
    #[helper(
        side = "server",
        receiver = "dragon",
        target = "horizon_client::game::native_dragon_refs::Game<'_>::native_dragon_previously_killed"
    )]
    fn dragon_previously_killed();
}
