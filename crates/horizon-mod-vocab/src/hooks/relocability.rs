//! The `relocability` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// Whether a piston may move your block `state` at `pos`
    /// (`IBlockExtension.getRelocability`): asked at the last step of
    /// `PistonBaseBlock.isPushable`, for a block that got past its push
    /// reaction, whenever a piston's structure takes the block in. Left `None`,
    /// vanilla's rule holds: a block with a block entity does not move, and any
    /// other block does. `Some(false)` keeps the block in place at that
    /// position. `Some(true)` lets a piston move it even with a mod block
    /// entity, which travels with the block: taken out of the world as the
    /// move starts, with its saved data and its attachments and without its
    /// removal side effects (nothing drops), and put where the block lands as a
    /// new block entity of that position, with a new ticker slot; a handle to
    /// the old position no longer resolves. If the moving block is replaced
    /// before it lands, the block entity is removed there as a broken block's
    /// is (a container drops its contents). A save mid-move keeps it in the
    /// moving piston's tag. A block entity of a vanilla kind (a chest, a
    /// hopper) never moves, whatever the answer. While the block moves it draws
    /// as its state, without its block entity's drawing. Asked on both sides,
    /// as a client replays the move on its own copy: answer the same on both,
    /// from the arguments, without touching the world. Only the block's own
    /// part is asked, so vanilla blocks cost nothing. Diverges from upstream:
    /// new hook.
    #[hook(
        side = "both",
        contract = "replace",
        out = "movable",
        scope = "block",
        key = "state",
        target = "horizon_client::game::native_hooks::relocability::GameLevel<'_>::native_relocability",
        neoforge = "IBlockExtension.getRelocability"
    )]
    fn relocability();
}
