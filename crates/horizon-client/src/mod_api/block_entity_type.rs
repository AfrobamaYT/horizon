//! Typed declarations of native block entity types.
use std::sync::Arc;

/// A registered block entity type and the blocks that may hold it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlockEntityTypeKey(pub(super) Arc<BlockEntityType>);

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) struct BlockEntityType {
    pub id: String,
    pub blocks: Vec<String>,
}

impl BlockEntityTypeKey {
    /// The namespaced registry and save id.
    pub fn id(&self) -> &str {
        &self.0.id
    }
}

/// Builds one native block entity type during mod registration.
pub struct BlockEntityTypeBuilder<'a> {
    pub(super) registrar: &'a mut super::Registrar,
    pub(super) id: String,
    pub(super) blocks: Vec<String>,
}

impl BlockEntityTypeBuilder<'_> {
    /// The blocks which may contain this type.
    pub fn blocks(mut self, blocks: &[&str]) -> Self {
        self.blocks = blocks.iter().map(|block| (*block).to_owned()).collect();
        self
    }

    /// Register the type and return its typed key.
    pub fn build(self) -> BlockEntityTypeKey {
        self.registrar.register_block_entity_type(self.id, self.blocks)
    }
}
