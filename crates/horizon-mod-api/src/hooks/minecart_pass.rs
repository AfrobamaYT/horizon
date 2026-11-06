//! The `minecart_pass` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// A minecart passed over a rail block state this part is attached to (the moment `AbstractMinecart` calls `activateMinecart` on an activator rail, for every rail; Forge's `IBaseRailExtension.onMinecartPass`, which NeoForge 26.3 no longer has, so this hook has no counterpart). `event.entity` is the cart, `event.pos` the rail. With the old minecart physics it fires once per tick the cart is on a rail; with `minecart_improvements` once per track iteration, so a fast cart can pass several times in a tick. It fires after the cart's batch has ticked, not mid-move, so a write the part makes (a velocity change through the cart handle) lands on the stored cart, and a pass whose cart is gone or whose rail changed is dropped. Only a rail state a part listens for is recorded; with no such part nothing is kept. Diverges from upstream: new hook.
    #[hook(
        side = "server",
        contract = "listen",
        scope = "block",
        key = "state",
        neoforge = "none (Forge's IBaseRailExtension.onMinecartPass is gone in NeoForge 26.3)"
    )]
    fn minecart_pass();
}
