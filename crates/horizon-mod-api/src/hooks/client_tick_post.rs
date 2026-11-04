//! The `client_tick_post` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// Once every client tick, paused or not and on the title screen too, after the window's tick ran (NeoForge's
    /// `ClientTickEvent.Post`): the window's `Game` only, so not on a server's
    /// own tick (`server_tick` is that moment). Client-side.
    #[hook(
        side = "client",
        contract = "listen",
        neoforge = "ClientTickEvent.Post"
    )]
    fn client_tick_post();
}
