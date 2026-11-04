//! The `raid_refs` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_level_raid"
    )]
    fn level_raid();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_level_raid_at"
    )]
    fn level_raid_at();
    #[helper(
        side = "server",
        receiver = "level",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_level_raids"
    )]
    fn level_raids();
    #[helper(
        side = "server",
        receiver = "mob",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_mob_raid"
    )]
    fn mob_raid();
    #[helper(
        side = "server",
        receiver = "raid",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_raid_center"
    )]
    fn raid_center();
    #[helper(
        side = "server",
        receiver = "raid",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_raid_omen_level"
    )]
    fn raid_omen_level();
    #[helper(
        side = "server",
        receiver = "raid",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_raid_set_omen_level"
    )]
    fn raid_set_omen_level();
    #[helper(
        side = "server",
        receiver = "raid",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_raid_groups_spawned"
    )]
    fn raid_groups_spawned();
    #[helper(
        side = "server",
        receiver = "raid",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_raid_num_groups"
    )]
    fn raid_num_groups();
    #[helper(
        side = "server",
        receiver = "raid",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_raid_has_bonus_wave"
    )]
    fn raid_has_bonus_wave();
    #[helper(
        side = "server",
        receiver = "raid",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_raid_is_started"
    )]
    fn raid_is_started();
    #[helper(
        side = "server",
        receiver = "raid",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_raid_is_active"
    )]
    fn raid_is_active();
    #[helper(
        side = "server",
        receiver = "raid",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_raid_status"
    )]
    fn raid_status();
    #[helper(
        side = "server",
        receiver = "raid",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_raid_total_health"
    )]
    fn raid_total_health();
    #[helper(
        side = "server",
        receiver = "raid",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_raid_living_health"
    )]
    fn raid_living_health();
    #[helper(
        side = "server",
        receiver = "raid",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_raid_bar_progress"
    )]
    fn raid_bar_progress();
    #[helper(
        side = "server",
        receiver = "raid",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_raid_raiders"
    )]
    fn raid_raiders();
    #[helper(
        side = "server",
        receiver = "raid",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_raid_wave_raiders"
    )]
    fn raid_wave_raiders();
    #[helper(
        side = "server",
        receiver = "raid",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_raid_heroes"
    )]
    fn raid_heroes();
    #[helper(
        side = "server",
        receiver = "raid",
        target = "horizon_client::game::native_raid_refs::Game<'_>::native_raid_stop"
    )]
    fn raid_stop();
}
