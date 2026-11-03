//! Data-only inputs carried from an owning load boundary to native load hooks.
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

/// The phase that owns this data load, selected before handing work to loaders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DataLoadOrigin {
    ServerDataLoad,
    ClientPreload,
}

/// Immutable metadata captured with the loaded item data. Later data loads
/// borrow the same registry and mod views instead of scanning them again.
pub struct LoadInputs {
    registries: BTreeMap<String, Vec<String>>,
    settings_root: PathBuf,
    mods: Vec<String>,
}

impl LoadInputs {
    pub fn new(
        registries: BTreeMap<String, Vec<String>>,
        settings_root: PathBuf,
        mods: Vec<String>,
    ) -> Self {
        Self {
            registries,
            settings_root,
            mods,
        }
    }
}

/// Origin and borrowed engine inputs for one owning load. This contains no
/// game, world handles or mutable registry access.
pub struct LoadContext {
    origin: DataLoadOrigin,
    inputs: Arc<LoadInputs>,
}

impl LoadContext {
    /// Called at a load boundary, never by a reader of already-loaded data.
    pub fn new(origin: DataLoadOrigin, inputs: Arc<LoadInputs>) -> Self {
        Self { origin, inputs }
    }

    /// Start another owning load over this resource generation's metadata.
    /// Only the boundary chooses the new origin; readers borrow its context.
    pub fn for_load(&self, origin: DataLoadOrigin) -> Self {
        Self::new(origin, self.inputs.clone())
    }

    pub fn origin(&self) -> DataLoadOrigin {
        self.origin
    }

    /// Names in registry ID order, from the load's registry snapshot.
    pub fn registry(&self, name: &str) -> Option<&[String]> {
        self.inputs.registries.get(name).map(Vec::as_slice)
    }

    /// A mod's setting, read only when the load hook requests that key.
    pub fn setting(&self, mod_id: &str, key: &str) -> Option<String> {
        read_setting(&self.inputs.settings_root.join(mod_id), key)
    }

    /// Native mod IDs in dispatch order, captured with the loaded inputs.
    pub fn mods(&self) -> &[String] {
        &self.inputs.mods
    }
}

/// Read the same per-key storage as the runtime settings service. No folder is
/// created, and unrelated mod data files are not scanned or loaded eagerly.
pub fn read_setting(directory: &Path, key: &str) -> Option<String> {
    fs::read_to_string(directory.join(key))
        .ok()
        .map(|value| value.trim().to_owned())
}
