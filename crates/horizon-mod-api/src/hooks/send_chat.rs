//! The `send_chat` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Helpers {
    #[helper(
        side = "client",
        target = "horizon_client::game::native_net::Game<'_>::native_send_chat"
    )]
    fn send_chat();
}
