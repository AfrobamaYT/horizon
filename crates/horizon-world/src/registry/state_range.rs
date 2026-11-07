//! A range of engine registry state IDs.
use super::BlockStateId;

/// The states of one block (`StateDefinition.getPossibleStates`): consecutive ids, as the
/// registry numbers a block's states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
// exhaustive: the game builds it with `new`; mods read it
pub struct BlockStateRange {
    first: BlockStateId,
    count: u32,
}
impl BlockStateRange {
    /// `count` states from `first`.
    pub const fn new(first: BlockStateId, count: u32) -> Self {
        Self { first, count }
    }
    /// The first state's id.
    pub const fn first(self) -> BlockStateId {
        self.first
    }
    /// How many states the block has.
    pub const fn len(self) -> u32 {
        self.count
    }
    /// Never true: a block has at least one state.
    pub const fn is_empty(self) -> bool {
        self.count == 0
    }
    /// Whether `state` is one of them.
    pub const fn contains(self, state: BlockStateId) -> bool {
        state.0 >= self.first.0 && state.0 - self.first.0 < self.count
    }
    /// Each state, in id order.
    pub fn iter(self) -> impl Iterator<Item = BlockStateId> {
        (self.first.0..self.first.0 + self.count).map(BlockStateId)
    }
}
