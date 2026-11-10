//! The `on_destroyed_by_player` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// A player removes the block `state` at `pos` (`IBlockExtension.onDestroyedByPlayer`, where
    /// `ServerPlayerGameMode.destroyBlock` runs `playerWillDestroy` and removes the block), with
    /// `tool`, the stack in the player's hand (`None` for a bare hand), and `will_harvest`, whether
    /// the break will drop loot (never in creative, and false for a tool the block refuses). Asked
    /// after `break_block` allowed the break. `destroyed` is whether the block was actually destroyed:
    /// `Some(true)` says the part removed it itself, `Some(false)` that it stays (a block of layers
    /// losing one layer: change it with the `host` and leave it standing), and either answer replaces
    /// the game's own `playerWillDestroy` and removal. `Some(false)` ends the break there: no drops,
    /// tool wear or statistic. Left `None`, vanilla's removal runs. Server-side, for every player of
    /// the world; the first part answering wins. Asked of the block's own parts only, so a block with
    /// none costs nothing.
    #[hook(
        side = "server",
        contract = "replace",
        out = "destroyed",
        scope = "block",
        key = "state",
        neoforge = "IBlockExtension.onDestroyedByPlayer"
    )]
    fn on_destroyed_by_player();
}
