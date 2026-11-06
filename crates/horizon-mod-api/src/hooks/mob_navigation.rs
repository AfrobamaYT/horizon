//! The `mob_navigation` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_navigation::Game<'_>::native_mob_move_to"
    )]
    fn mob_move_to();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_navigation::Game<'_>::native_mob_stop"
    )]
    fn mob_stop();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_navigation::Game<'_>::native_mob_is_done"
    )]
    fn mob_is_done();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_navigation::Game<'_>::native_mob_navigation_speed"
    )]
    fn mob_navigation_speed();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_navigation::Game<'_>::native_mob_set_navigation_speed"
    )]
    fn mob_set_navigation_speed();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_navigation::Game<'_>::native_mob_pathfinding_malus"
    )]
    fn mob_pathfinding_malus();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_navigation::Game<'_>::native_mob_set_pathfinding_malus"
    )]
    fn mob_set_pathfinding_malus();
}
