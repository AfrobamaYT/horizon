//! Data-only inputs carried from an owning load boundary to native load hooks.
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

/// The phase that owns this data load, selected before handing work to loaders.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DataLoadOrigin {
    ServerDataLoad,
    ClientPreload,
}

/// Engine inputs for one load. The mod API exposes a restricted borrowed view;
/// this type contains no game, world handles or mutable registry access.
pub struct LoadContext {
    origin: DataLoadOrigin,
    registries: BTreeMap<String, Vec<String>>,
    settings_root: PathBuf,
    mods: Vec<String>,
}

impl LoadContext {
    /// Assemble the inputs once at the load boundary, never in a data reader.
    pub fn new(
        origin: DataLoadOrigin,
        registries: BTreeMap<String, Vec<String>>,
        settings_root: PathBuf,
        mods: Vec<String>,
    ) -> Self {
        Self {
            origin,
            registries,
            settings_root,
            mods,
        }
    }

    pub fn origin(&self) -> DataLoadOrigin {
        self.origin
    }

    /// Names in registry ID order, from the load's registry snapshot.
    pub fn registry(&self, name: &str) -> Option<&[String]> {
        self.registries.get(name).map(Vec::as_slice)
    }

    /// A mod's setting, read only when the load hook requests that key.
    pub fn setting(&self, mod_id: &str, key: &str) -> Option<String> {
        read_setting(&self.settings_root.join(mod_id), key)
    }

    /// Native mod IDs in dispatch order, captured at the boundary.
    pub fn mods(&self) -> &[String] {
        &self.mods
    }
}

/// Read the same per-key storage as the runtime settings service. No folder is
/// created, and unrelated mod data files are not scanned or loaded eagerly.
pub fn read_setting(directory: &Path, key: &str) -> Option<String> {
    fs::read_to_string(directory.join(key))
        .ok()
        .map(|value| value.trim().to_owned())
}
