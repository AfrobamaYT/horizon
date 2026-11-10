//! The `client_level_tick_post` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// Once for each tick of the client level in place, after the level's own
    /// work ran and it returned normally (Fabric's `ClientTickEvents.END_LEVEL_TICK`):
    /// the same level `client_level_tick_pre` named. A tick that failed
    /// returns nothing, as Fabric's injection at `ClientLevel.tick`'s RETURN
    /// does not run for an exception. It only hears. Client-side.
    #[hook(
        side = "client",
        contract = "listen",
        fabric = "ClientTickEvents.END_LEVEL_TICK",
        neoforge = "none (ClientTickEvent has no per-level phase)"
    )]
    fn client_level_tick_post();
}
