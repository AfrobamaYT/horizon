//! The read-only metadata resource behind block value operations.
use crate::{BlockKey, StateConstraint};

/// Transport form for a process-local state. Mods use the BlockState facade, not this record.
/// Receiving metadata and write boundaries validate the ordinal against the block definition.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct BlockStateRecord {
    /// Process-local block key, not a saved block name.
    pub block: BlockKey,
    /// State ordinal within the block's definition.
    pub state: u16,
}
impl BlockStateRecord {
    /// Construct transport data; registry-dependent bounds belong to the receiving boundary.
    pub const fn new(block: BlockKey, state: u16) -> Self {
        Self { block, state }
    }
}

/// A refused metadata read or state operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlockStateError {
    /// Metadata has not been published for this context.
    Unavailable,
    /// The context does not contain the block key.
    UnknownBlock,
    /// The ordinal is outside the block definition.
    UnknownState,
    /// Constraint index naming a missing property.
    UnknownProperty(u32),
    /// Constraint index naming an invalid value.
    InvalidValue(u32),
    /// Constraint index repeating a property.
    DuplicateProperty(u32),
    /// The requested combination is not a registered state.
    InvalidCombination,
}
impl From<horizon_mod_runtime::block_definitions::StateError> for BlockStateError {
    fn from(error: horizon_mod_runtime::block_definitions::StateError) -> Self {
        use horizon_mod_runtime::block_definitions::StateError;
        match error {
            StateError::UnknownState => Self::UnknownState,
            StateError::UnknownProperty(index) => Self::UnknownProperty(index as u32),
            StateError::InvalidValue(index) => Self::InvalidValue(index as u32),
            StateError::DuplicateProperty(index) => Self::DuplicateProperty(index as u32),
            StateError::InvalidCombination => Self::InvalidCombination,
        }
    }
}

/// Read-only metadata, lent by a host's block-states service or a tint worker.
/// This is the low-level resource contract; mods call the BlockState facade instead.
/// Native and generated WebAssembly forwards invoke the same immutable table implementation.
#[doc(hidden)]
pub trait BlockMetadata {
    /// Borrow a property's serialized value; a missing property returns None.
    fn property(
        &self,
        state: BlockStateRecord,
        name: &str,
    ) -> Result<Option<&str>, BlockStateError>;
    /// Change one property while retaining the others.
    fn with(
        &self,
        state: BlockStateRecord,
        name: &str,
        value: &str,
    ) -> Result<BlockStateRecord, BlockStateError>;
    /// Apply partial constraints to the block's default state.
    fn of(
        &self,
        block: BlockKey,
        properties: &[StateConstraint],
    ) -> Result<BlockStateRecord, BlockStateError>;
}
impl BlockMetadata for horizon_mod_runtime::block_definitions::Blocks {
    fn property(
        &self,
        state: BlockStateRecord,
        name: &str,
    ) -> Result<Option<&str>, BlockStateError> {
        self.definitions
            .get(state.block.0 as usize)
            .ok_or(BlockStateError::UnknownBlock)?
            .property(state.state, name)
            .map_err(Into::into)
    }
    fn with(
        &self,
        state: BlockStateRecord,
        name: &str,
        value: &str,
    ) -> Result<BlockStateRecord, BlockStateError> {
        let ordinal = self
            .definitions
            .get(state.block.0 as usize)
            .ok_or(BlockStateError::UnknownBlock)?
            .with(state.state, name, value)?;
        Ok(BlockStateRecord::new(state.block, ordinal))
    }
    fn of(
        &self,
        block: BlockKey,
        properties: &[StateConstraint],
    ) -> Result<BlockStateRecord, BlockStateError> {
        let constraints: Vec<_> = properties
            .iter()
            .map(|p| (p.name.as_str(), p.value.as_str()))
            .collect();
        let ordinal = self
            .definitions
            .get(block.0 as usize)
            .ok_or(BlockStateError::UnknownBlock)?
            .of(&constraints)?;
        Ok(BlockStateRecord::new(block, ordinal))
    }
}
