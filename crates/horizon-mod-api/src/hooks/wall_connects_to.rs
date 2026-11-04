//! The `wall_connects_to` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// `WallBlock.connectsTo`: set `connected` to say whether the wall
    /// `state` at `pos` connects a side toward `neighbour`, the state beside it,
    /// `direction` pointing from the wall to that neighbour and `face_solid`
    /// whether the neighbour's face toward the wall is sturdy. `connected` holds
    /// vanilla's answer (a wall, a solid face, iron bars or a pane, or a gate
    /// across the side): change it, to false as well as to true, when the wall is
    /// your own. A connected side is low or tall from the block above, as vanilla's.
    /// Asked whenever the wall settles its shape or is placed, inside the level's
    /// update: answer from the arguments without touching the world. Only the wall
    /// block's own part is asked, so vanilla walls cost nothing. Authoritative
    /// servers only.
    #[hook(
        side = "server",
        contract = "collect",
        out = "connected",
        scope = "block",
        key = "state",
        target = "horizon_client::game::native_hooks::GameLevel<'_>::native_wall_connects_to"
    )]
    fn wall_connects_to(
        dimension: LevelDimension,
        pos: BlockPos,
        state: BlockStateId,
        neighbour: BlockStateId,
        direction: Direction,
        face_solid: bool,
        connected: &mut bool,
    );
}
