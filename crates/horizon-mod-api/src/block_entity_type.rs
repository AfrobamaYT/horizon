//! Typed identities and declarations of block entity types.
use std::{fmt, sync::Arc};

/// A block entity type's namespaced registry and save identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
// exhaustive: its fields are private, so mods cannot build or destructure it
pub struct BlockEntityTypeKey(pub(super) Arc<str>);

impl BlockEntityTypeKey {
    /// The namespaced registry and save id.
    pub fn id(&self) -> &str {
        &self.0
    }

    pub(crate) fn registered(id: &str) -> Option<Self> {
        let types = horizon_client_core::block_entity_types::current()?;
        types.key(id).map(Self)
    }
}

impl AsRef<str> for BlockEntityTypeKey {
    fn as_ref(&self) -> &str {
        self.id()
    }
}

/// A native Part cannot be attached to this type during registration.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlockEntityPartError {
    /// Vanilla entities do not expose a native Part's payload.
    VanillaType {
        /// The block entity type that was refused.
        kind: BlockEntityTypeKey,
    },
    /// This Part type is already attached to the entity type.
    Duplicate {
        /// The block entity type that was refused.
        kind: BlockEntityTypeKey,
        /// The Part type that is already attached.
        part: &'static str,
    },
}

impl fmt::Display for BlockEntityPartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VanillaType { kind } => write!(
                f,
                "cannot attach a native block entity Part to vanilla type {}",
                kind.id()
            ),
            Self::Duplicate { kind, part } => {
                write!(f, "Part {part} is attached to type {} twice", kind.id())
            }
        }
    }
}

impl std::error::Error for BlockEntityPartError {}

#[derive(Debug)]
// exhaustive: its fields are private, so mods cannot build or destructure it
pub struct BlockEntityTypeLookupError {
    missing: Vec<&'static super::part_table::BlockEntityTypeRequest>,
}

impl fmt::Display for BlockEntityTypeLookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, request) in self.missing.iter().enumerate() {
            if index > 0 {
                writeln!(f)?;
            }
            write!(
                f,
                "mod {} requested unknown block entity type {}",
                request.mod_id,
                request.kind.id(),
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for BlockEntityTypeLookupError {}

/// Checks that every registered block entity type key names a type the game knows.
pub fn validate_block_entity_type_keys(
    types: &horizon_client_core::block_entity_types::BlockEntityTypes,
) -> Result<(), BlockEntityTypeLookupError> {
    let missing: Vec<_> = super::part_table::requested_block_entity_types()
        .iter()
        .filter(|request| types.key(request.kind.id()).is_none())
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(BlockEntityTypeLookupError { missing })
    }
}

/// Builds one native block entity type during mod registration.
// exhaustive: its fields are private, so mods cannot build or destructure it
pub struct BlockEntityTypeBuilder<'a> {
    pub(super) registrar: &'a mut dyn super::Registrar,
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
        self.registrar
            .declare_block_entity_type(&self.id, self.blocks)
    }
}
