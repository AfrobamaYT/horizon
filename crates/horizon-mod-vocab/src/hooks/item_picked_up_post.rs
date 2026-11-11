//! The `item_picked_up_post` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// `player` took at least part of a ground stack into their inventory
    /// (`ItemEntity.playerTouch`, right after `Inventory.add`). `original` is
    /// the stack as it lay before the pickup and `remaining` what is left on
    /// the ground, `None` when all of it was taken. It only hears: not called
    /// when `item_picked_up` refused the pickup or the inventory took nothing.
    /// Server-side.
    #[hook(
        side = "server",
        contract = "listen",
        neoforge = "ItemEntityPickupEvent.Post"
    )]
    fn item_picked_up_post();
}
