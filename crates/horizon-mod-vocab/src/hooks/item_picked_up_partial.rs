//! The `item_picked_up_partial` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// `player` picked up part of a ground stack (`ItemEntity.playerTouch`,
    /// right after `Inventory.add` took some of it and left the rest).
    /// `original` is a copy of the stack as it lay before the pickup:
    /// changing it does nothing. `remaining` is what the item entity still
    /// holds, so it starts as the part that did not fit. Set another stack
    /// there to leave the entity holding it, or `None` to empty it: the
    /// pickup then counts `original`'s count less what is left. The amount
    /// taken is `original`'s count less `remaining`'s. A mod that removes the
    /// item entity meanwhile keeps it removed: no take animation, the stat
    /// and criteria still run. Of the two notices, this one hears a partial
    /// pickup and `item_picked_up_post` a whole one; a pickup is never told
    /// to both. Not called when `item_picked_up` refused the pickup or the
    /// inventory took nothing. A creative player always takes the whole
    /// stack. Server-side; no NeoForge counterpart.
    #[hook(side = "server", contract = "listen")]
    fn item_picked_up_partial();
}
