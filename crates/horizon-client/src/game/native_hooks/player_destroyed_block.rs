use super::*;

/// The hook points. Each does nothing in Horizon.
impl Game<'_> {
    /// A player's break removed the block `state` from `pos`:
    /// `ServerPlayerGameMode.destroyBlock`'s `Block.destroy` call, once
    /// `Level.removeBlock` succeeded. `block_entity` is the saved data the
    /// block entity there had, read before the removal. Server-side.
    pub(in crate::game) fn native_player_destroyed_block(
        &mut self,
        pos: [i32; 3],
        state: BlockStateId,
        block_entity: Option<Compound>,
    ) {
        self.fire_player_destroyed_block(pos, state, block_entity);
    }
}
