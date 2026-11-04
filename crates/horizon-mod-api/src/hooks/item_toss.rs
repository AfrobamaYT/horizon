//! The `item_toss` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// `ItemTossEvent`: `player` tosses `count` of `item` (`Player.drop`: the drop key, a menu's throw, what no longer fits in the inventory), not what scatters when they die. Set `canceled` and no item entity is made; the items are already out of the inventory and are lost, as in NeoForge. A canceled toss awards no drop statistic, as vanilla awards one only when an item entity came of it. Server-side. Diverges from upstream: replaces the listen-only `item_dropped`, which fired for the drop key and menu throws only.
    #[hook(
        side = "server",
        contract = "cancel",
        out = "canceled",
        neoforge = "ItemTossEvent",
        cancellable = true
    )]
    fn item_toss();
}
