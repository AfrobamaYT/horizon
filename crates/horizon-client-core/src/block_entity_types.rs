//! Accepted native block entity membership, frozen before resources or worlds load.
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, OnceLock},
};

#[derive(Debug, PartialEq, Eq)]
pub struct BlockEntityTypes {
    blocks: BTreeMap<String, String>,
    /// Names in protocol order: a reload may not change numeric wire IDs either.
    wire: Vec<Arc<str>>,
    keys: BTreeSet<Arc<str>>,
}

impl BlockEntityTypes {
    /// Membership has already been checked by the content plan.
    pub fn new(blocks: BTreeMap<String, String>, wire: Vec<String>) -> Self {
        let wire: Vec<Arc<str>> = wire.into_iter().map(Into::into).collect();
        let keys = wire.iter().filter(|id| !id.is_empty()).cloned().collect();
        Self { blocks, wire, keys }
    }

    pub fn type_for(&self, block: &str) -> Option<&str> {
        self.blocks.get(block).map(String::as_str)
    }

    /// Registered names in protocol order, excluding empty slots.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.wire
            .iter()
            .filter(|id| !id.is_empty())
            .map(AsRef::as_ref)
    }

    /// A canonical registered identity, shared without allocating at lookup.
    pub fn key(&self, id: &str) -> Option<Arc<str>> {
        self.keys.get(id).cloned()
    }

    /// Whether a mod declared the block entity type `id`.
    /// Wire keys include vanilla types; `mod_content::Plan::new` refuses native
    /// declarations in `minecraft:`.
    pub fn declares(&self, id: &str) -> bool {
        !id.starts_with("minecraft:") && self.keys.contains(id)
    }
}

static TYPES: OnceLock<BlockEntityTypes> = OnceLock::new();

/// Reading before startup installation never freezes an empty table.
pub fn type_for(block: &str) -> Option<&'static str> {
    current()?.type_for(block)
}

pub fn current() -> Option<&'static BlockEntityTypes> {
    TYPES.get()
}

/// Whether a mod declared the block entity type `id` (none before startup
/// installs the table).
pub fn declared(id: &str) -> bool {
    current().is_some_and(|types| types.declares(id))
}

/// An identical setup is harmless; changing membership or wire IDs needs a restart.
pub fn install(types: BlockEntityTypes) -> Result<(), &'static str> {
    freeze(&TYPES, types)
}

fn freeze(slot: &OnceLock<BlockEntityTypes>, types: BlockEntityTypes) -> Result<(), &'static str> {
    match slot.set(types) {
        Ok(()) => Ok(()),
        Err(types) if slot.get() == Some(&types) => Ok(()),
        Err(_) => Err("block entity types changed; restart the game to use them"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declaration_lookup_keeps_vanilla_types_and_block_ids_out() {
        let types = BlockEntityTypes::new(
            [
                ("test:press".into(), "test:machine".into()),
                ("test:mill".into(), "test:machine".into()),
                ("test:casing".into(), "test:single".into()),
            ]
            .into(),
            vec![
                "minecraft:chest".into(),
                "test:machine".into(),
                "test:single".into(),
                "test:empty".into(),
            ],
        );
        for (id, declared) in [
            ("test:machine", true),
            ("test:single", true),
            ("test:empty", true),
            ("minecraft:chest", false),
            ("test:press", false),
            ("test:mill", false),
            ("test:casing", false),
            ("other:machine", false),
            ("", false),
        ] {
            assert_eq!(types.declares(id), declared, "{id}");
        }
    }

    #[test]
    fn shared_membership_and_wire_ids_are_frozen_together() {
        let slot = OnceLock::new();
        let types = || {
            BlockEntityTypes::new(
                [
                    ("test:press".into(), "test:machine".into()),
                    ("test:mill".into(), "test:machine".into()),
                ]
                .into(),
                vec!["minecraft:sign".into(), "test:machine".into()],
            )
        };
        freeze(&slot, types()).unwrap();
        freeze(&slot, types()).unwrap();
        let accepted = slot.get().unwrap();
        for block in ["test:press", "test:mill"] {
            assert_eq!(accepted.type_for(block), Some("test:machine"));
        }
        for block in ["test:casing", "other:press", "minecraft:stone"] {
            assert_eq!(accepted.type_for(block), None);
        }
        let mut changed = types();
        changed.blocks.remove("test:mill");
        assert!(freeze(&slot, changed).is_err());
        let mut changed = types();
        changed.wire.swap(0, 1);
        assert!(freeze(&slot, changed).is_err());
        assert_eq!(slot.get(), Some(&types()));
    }
}
