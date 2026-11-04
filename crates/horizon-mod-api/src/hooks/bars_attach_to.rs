//! The `bars_attach_to` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// `IronBarsBlock.attachsTo`, for iron bars and glass panes: set `connected`
    /// to say whether the bars or pane `state` at `pos` attach an arm toward
    /// `neighbour`, the state beside it, `direction` pointing from the bars to
    /// that neighbour and `face_solid` whether the neighbour's face toward the
    /// bars is sturdy. `connected` holds vanilla's answer (a solid face, iron bars
    /// or a pane, or a wall): change it, to false as well as to true, when the
    /// bars are your own. Vanilla's rule ignores `direction`; it is there for
    /// yours. Asked whenever the bars settle their shape or are placed, inside the
    /// level's update: answer from the arguments without touching the world. Only
    /// the bars block's own part is asked, so vanilla bars and panes cost nothing.
    /// Authoritative servers only.
    #[hook(
        side = "server",
        contract = "collect",
        out = "connected",
        scope = "block",
        key = "state",
        target = "horizon_client::game::native_hooks::GameLevel<'_>::native_bars_attach_to"
    )]
    fn bars_attach_to(
        dimension: LevelDimension,
        pos: BlockPos,
        state: BlockStateId,
        neighbour: BlockStateId,
        direction: Direction,
        face_solid: bool,
        connected: &mut bool,
    );
}
