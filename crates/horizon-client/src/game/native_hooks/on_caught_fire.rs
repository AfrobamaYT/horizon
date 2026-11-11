use super::*;

impl GameLevel<'_> {
    /// On an authoritative server, fire has just burned the block `state` at
    /// `pos` from its side `face`, the one that faces the fire
    /// (`FireBlock.checkBurnOut`): the cell is already fire or air, and
    /// `state` is the block as it was. It runs after the game's own action (a
    /// TNT-class block is primed, which `tnt_primed` may still cancel), and
    /// not when `block_burn` cancelled the burn. `ticks` is the hook's.
    #[inline(always)]
    pub(in crate::game) fn native_on_caught_fire(
        &mut self,
        ticks: &mut Ticks,
        dimension: LevelDimension,
        state: BlockStateId,
        pos: BlockPos,
        face: Direction,
    ) {
        self.fire_on_caught_fire(ticks, dimension, state, pos, face);
    }
}
