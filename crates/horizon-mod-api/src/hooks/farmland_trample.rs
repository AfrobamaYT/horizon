//! The `farmland_trample` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// Called on the server before a landing converts farmland to its base block. It is given the loop `context` of the side that ticks the entity (its registries, settings and mod list); it fires inside the tick's own landing, before the block is written, so it takes no `Host`.
    #[hook(
        side = "server",
        contract = "cancel",
        out = "cancel",
        context = "loop",
        neoforge = "BlockEvent.FarmlandTrampleEvent",
        cancellable = true,
        version = 2,
        target = "horizon_client_lib::landing::native_farmland_trample"
    )]
    fn farmland_trample();
}
