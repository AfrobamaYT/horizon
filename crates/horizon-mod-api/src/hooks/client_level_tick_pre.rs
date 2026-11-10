//! The `client_level_tick_pre` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// Once for each tick of the client level in place, before the level's own
    /// work runs (Fabric's `ClientTickEvents.START_LEVEL_TICK`): a window
    /// showing a server's world, single player's included, and not the title
    /// screen or a connect that has no level yet. `level` is that level's
    /// handle. It only hears. Client-side.
    #[hook(
        side = "client",
        contract = "listen",
        fabric = "ClientTickEvents.START_LEVEL_TICK",
        neoforge = "none (ClientTickEvent has no per-level phase)"
    )]
    fn client_level_tick_pre();
}
