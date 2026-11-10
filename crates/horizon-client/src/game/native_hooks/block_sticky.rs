use super::*;

impl GameLevel<'_> {
    /// `IBlockExtension.isSlimeBlock` for the piston structure resolver: `slime` starts with
    /// vanilla's answer and the block's parts may change it.
    #[inline(always)]
    pub(in crate::game) fn native_is_slime_block(
        &self,
        dimension: LevelDimension,
        state: BlockStateId,
        slime: &mut bool,
    ) {
        self.fire_is_slime_block(dimension, state, slime);
    }

    /// `IBlockExtension.isStickyBlock` for the piston structure resolver: `sticky` starts with
    /// vanilla's answer and the block's parts may change it.
    #[inline(always)]
    pub(in crate::game) fn native_is_sticky_block(
        &self,
        dimension: LevelDimension,
        state: BlockStateId,
        sticky: &mut bool,
    ) {
        self.fire_is_sticky_block(dimension, state, sticky);
    }

    /// `IBlockExtension.canStickTo` for the piston structure resolver: `sticks` starts with
    /// vanilla's answer for `state` beside `other`, and the parts of `state` may change it.
    #[inline(always)]
    pub(in crate::game) fn native_can_stick_to(
        &self,
        dimension: LevelDimension,
        state: BlockStateId,
        other: BlockStateId,
        sticks: &mut bool,
    ) {
        self.fire_can_stick_to(dimension, state, other, sticks);
    }
}
