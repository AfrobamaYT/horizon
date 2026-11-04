//! The `reload_resources` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "server",
        target = "horizon_client::game::server_reload::Game<'_>::native_reload_resources"
    )]
    fn reload_resources();
}
