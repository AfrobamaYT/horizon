//! One block entity's or entity's saved data upgraded on its own, for callers that hold it outside a chunk (a mod
//! reading a schematic or an old structure): the fixes the chunk loaders run on it, behind a check of the input.
use super::{CURRENT, Ty, key, schema, update};
use horizon_core::nbt::{Compound, JavaString, Tag};

/// Why a value was not upgraded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpgradeError {
    /// The data version is below zero.
    NegativeVersion(i32),
    /// The data version is newer than the game's ([`CURRENT`]): the fixes only go forward.
    FutureVersion(i32),
}

impl std::fmt::Display for UpgradeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NegativeVersion(v) => write!(f, "data version {v} is negative"),
            Self::FutureVersion(v) => {
                write!(f, "data version {v} is newer than the game's ({CURRENT})")
            }
        }
    }
}

impl std::error::Error for UpgradeError {}

/// One block entity's NBT saved at `data_version`, upgraded as the chunk loader upgrades the block entities of a
/// chunk of that version: each block entity type's fixes, up to [`CURRENT`]. Data of the current version comes back
/// as it is, and so does a block entity whose `id` that version doesn't know, as a chunk keeps it. Fixes of the
/// whole chunk (the blocks around it, the removal of bed block entities) have no chunk to act on and don't run. A
/// tag the fixes can't read comes back as it was saved. Refused before any fix runs: a negative or future version.
pub fn upgrade_block_entity(data_version: i32, tag: Compound) -> Result<Compound, UpgradeError> {
    let tag = checked(data_version, tag)?;
    let id = Tag::compound([(
        "id",
        tag.get(&JavaString::from("id"))
            .cloned()
            .unwrap_or(Tag::End),
    )]);
    // Before 1.13 the known types come from the fixers' tables too: a failure there keeps the data, as a fix's would.
    let known =
        std::panic::catch_unwind(|| schema::block_entity_known(&id, key(data_version as u32, 0)));
    if known.unwrap_or(false) {
        Ok(upgraded(Ty::BlockEntity, tag, data_version))
    } else {
        Ok(tag)
    }
}

/// One entity's NBT saved at `data_version`, upgraded as the entity chunk loader upgrades the entities of a chunk of
/// that version: each entity type's fixes, its passengers' included, up to [`CURRENT`]. Data of the current version
/// comes back as it is; a tag the fixes can't read comes back as it was saved. Refused as
/// [`upgrade_block_entity`] refuses.
pub fn upgrade_entity(data_version: i32, tag: Compound) -> Result<Compound, UpgradeError> {
    let tag = checked(data_version, tag)?;
    Ok(upgraded(Ty::EntityTree, tag, data_version))
}

/// `tag` when its version can be upgraded from.
fn checked(data_version: i32, tag: Compound) -> Result<Compound, UpgradeError> {
    if data_version < 0 {
        Err(UpgradeError::NegativeVersion(data_version))
    } else if data_version > CURRENT {
        Err(UpgradeError::FutureVersion(data_version))
    } else {
        Ok(tag)
    }
}

/// [`update`] of one value of type `root` up to [`CURRENT`]. A value the fixes fail on (a legacy fixer's panic on
/// data it can't read) comes back as it was saved.
fn upgraded(root: Ty, tag: Compound, data_version: i32) -> Compound {
    if data_version == CURRENT {
        return tag;
    }
    let mut value = Tag::Compound(tag.clone());
    let done = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        update(root, &mut value, data_version, CURRENT);
    }))
    .is_ok();
    match value {
        Tag::Compound(upgraded) if done => upgraded,
        _ => tag,
    }
}
