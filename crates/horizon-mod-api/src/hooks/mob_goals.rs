//! The `mob_goals` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_goals::Game<'_>::native_mob_add_goal"
    )]
    fn mob_add_goal();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_goals::Game<'_>::native_mob_add_target_goal"
    )]
    fn mob_add_target_goal();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_goals::Game<'_>::native_mob_remove_goal"
    )]
    fn mob_remove_goal();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_goals::Game<'_>::native_mob_remove_target_goal"
    )]
    fn mob_remove_target_goal();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_goals::Game<'_>::native_mob_goals"
    )]
    fn mob_goals();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_mob_goals::Game<'_>::native_mob_target_goals"
    )]
    fn mob_target_goals();
}
