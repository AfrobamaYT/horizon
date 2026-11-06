//! The `projectile_refs` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        target = "horizon_client::game::native_projectile_refs::Game<'_>::native_projectile_ref"
    )]
    fn projectile();
    #[helper(
        side = "server",
        receiver = "projectile",
        target = "horizon_client::game::native_projectile_refs::Game<'_>::native_projectile_owner"
    )]
    fn projectile_owner();
    #[helper(
        side = "server",
        receiver = "projectile",
        target = "horizon_client::game::native_projectile_refs::Game<'_>::native_projectile_set_owner"
    )]
    fn projectile_set_owner();
    #[helper(
        side = "server",
        receiver = "projectile",
        target = "horizon_client::game::native_projectile_refs::Game<'_>::native_projectile_deflect"
    )]
    fn projectile_deflect();
    #[helper(
        side = "server",
        receiver = "projectile",
        target = "horizon_client::game::native_projectile_refs::Game<'_>::native_projectile_pickup"
    )]
    fn projectile_pickup();
    #[helper(
        side = "server",
        receiver = "projectile",
        target = "horizon_client::game::native_projectile_refs::Game<'_>::native_projectile_set_pickup"
    )]
    fn projectile_set_pickup();
    #[helper(
        side = "server",
        receiver = "projectile",
        target = "horizon_client::game::native_projectile_refs::Game<'_>::native_projectile_pierce_level"
    )]
    fn projectile_pierce_level();
    #[helper(
        side = "server",
        receiver = "projectile",
        target = "horizon_client::game::native_projectile_refs::Game<'_>::native_projectile_set_pierce_level"
    )]
    fn projectile_set_pierce_level();
    #[helper(
        side = "server",
        receiver = "projectile",
        target = "horizon_client::game::native_projectile_refs::Game<'_>::native_projectile_base_damage"
    )]
    fn projectile_base_damage();
    #[helper(
        side = "server",
        receiver = "projectile",
        target = "horizon_client::game::native_projectile_refs::Game<'_>::native_projectile_set_base_damage"
    )]
    fn projectile_set_base_damage();
    #[helper(
        side = "server",
        receiver = "projectile",
        target = "horizon_client::game::native_projectile_refs::Game<'_>::native_projectile_critical"
    )]
    fn projectile_critical();
    #[helper(
        side = "server",
        receiver = "projectile",
        target = "horizon_client::game::native_projectile_refs::Game<'_>::native_projectile_set_critical"
    )]
    fn projectile_set_critical();
}
