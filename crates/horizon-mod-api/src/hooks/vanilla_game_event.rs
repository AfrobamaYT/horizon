//! The `vanilla_game_event` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// A vanilla game event is about to reach its listeners (NeoForge's
    /// `VanillaGameEvent`, vanilla's `Level.gameEvent`): every event, in
    /// `dimension`. `event` is its id (`minecraft:step`), `pos` where it
    /// happened and `cause` the network id of the entity that caused it
    /// (None for none). The context's affected block state is not tracked
    /// here. Canceling keeps the sculk sensors, wardens and allays from
    /// hearing it. The affected block state in NeoForge's context is not
    /// currently exposed. Server-side.
    #[hook(
        side = "server",
        contract = "cancel",
        out = "cancel",
        neoforge = "VanillaGameEvent",
        cancellable = true
    )]
    fn vanilla_game_event();
}
