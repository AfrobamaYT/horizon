//! The `fluid_flow` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// The current of the fluid block at `pos` (`FlowingFluid.getFlow`, the vector that pushes an entity standing in it) is about to push a player, mob, item, experience orb, armor stand or arrow. `fluid` is the fluid's id (`minecraft:water`, `minecraft:flowing_lava`...); a mod may change `flow`, so it can rotate, strengthen or still a current. It runs for each fluid block the entity touches, so keep it short. The vector the renderer reads to turn a fluid's texture is not changed. Fires on both sides, on the client's replayed ticks too, with the loop `context` of the side that ticks the entity (its registries, settings and mod list): register the mod on both, and keep the listener a pure function of its inputs so prediction and the server agree. Beyond NeoForge: there is no counterpart. Diverges from upstream: new hook.
    #[hook(
        side = "both",
        contract = "listen",
        context = "loop",
        cancellable = false,
        version = 2,
        target = "horizon_client_lib::motion::native_fluid_flow",
        neoforge = "none (vanilla FlowingFluid.getFlow; NeoForge adds no event or extension for it)"
    )]
    fn fluid_flow();
}
