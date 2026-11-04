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
/// borrow the same registry snapshot instead of scanning it again.
pub struct LoadInputs {
    registries: BTreeMap<String, Vec<String>>,
    settings_root: PathBuf,
}

impl LoadInputs {
    pub fn new(registries: BTreeMap<String, Vec<String>>, settings_root: PathBuf) -> Self {
        Self {
            registries,
            settings_root,
        }
    }
}

/// Origin and borrowed engine inputs for one owning load. This contains no
/// game, world handles or mutable registry access.
pub struct LoadContext {
    origin: DataLoadOrigin,
    inputs: Arc<LoadInputs>,
    settings: BTreeMap<String, BTreeMap<String, String>>,
}

impl LoadContext {
    /// Called at a load boundary, never by a reader of already-loaded data.
    /// Captures settings for the owning phase's loaded package IDs; reusing the
    /// registry inputs for another load still captures fresh settings.
    pub fn new<'a>(
        origin: DataLoadOrigin,
        inputs: Arc<LoadInputs>,
        mod_ids: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        let mut settings = BTreeMap::new();
        for mod_id in mod_ids {
            let directory = inputs.settings_root.join(mod_id);
            let Ok(files) = fs::read_dir(&directory) else {
                continue;
            };
            let mut values = BTreeMap::new();
            for file in files.flatten() {
                let Ok(key) = file.file_name().into_string() else {
                    continue;
                };
                if let Some(value) = read_setting(&directory, &key) {
                    values.insert(key, value);
                }
            }
            settings.insert(mod_id.to_owned(), values);
        }
        Self {
            origin,
            inputs,
            settings,
        }
    }

    pub fn origin(&self) -> DataLoadOrigin {
        self.origin
    }

    /// Names in registry ID order, from the load's registry snapshot.
    pub fn registry(&self, name: &str) -> Option<&[String]> {
        self.inputs.registries.get(name).map(Vec::as_slice)
    }

    /// A loaded mod's setting captured at this load boundary, with no file I/O.
    pub fn setting(&self, mod_id: &str, key: &str) -> Option<String> {
        self.settings.get(mod_id)?.get(key).cloned()
    }
}

/// Read the same per-key storage as the runtime settings service. No folder is
/// created, and unrelated mod data files are not scanned or loaded eagerly.
pub fn read_setting(directory: &Path, key: &str) -> Option<String> {
    fs::read_to_string(directory.join(key))
        .ok()
        .map(|value| value.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_are_scoped_and_frozen_until_a_new_load() {
        let root = std::env::temp_dir().join(format!(
            "horizon-load-settings-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let active = root.join("active");
        fs::create_dir_all(&active).unwrap();
        fs::create_dir_all(root.join("inactive")).unwrap();
        fs::write(active.join("mode"), "  before\n").unwrap();
        fs::write(active.join("empty"), " \n").unwrap();
        fs::write(active.join("invalid"), [0xff]).unwrap();
        fs::write(root.join("inactive/mode"), "not loaded").unwrap();
        let inputs = Arc::new(LoadInputs::new(BTreeMap::new(), root.clone()));
        let before = LoadContext::new(DataLoadOrigin::ServerDataLoad, inputs.clone(), ["active"]);

        // Change the files before the first lookup, not just between reads.
        fs::write(active.join("mode"), "  after\n").unwrap();
        fs::write(active.join("late"), "new key").unwrap();
        assert_eq!(before.setting("active", "mode").as_deref(), Some("before"));
        assert_eq!(before.setting("active", "empty").as_deref(), Some(""));
        assert_eq!(before.setting("active", "invalid"), None);
        assert_eq!(before.setting("active", "late"), None);
        assert_eq!(before.setting("inactive", "mode"), None);
        assert_eq!(read_setting(&active, "mode").as_deref(), Some("after"));

        let after = LoadContext::new(DataLoadOrigin::ClientPreload, inputs.clone(), ["active"]);
        assert_eq!(after.setting("active", "mode").as_deref(), Some("after"));
        assert_eq!(after.setting("active", "late").as_deref(), Some("new key"));
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(before.setting("active", "mode").as_deref(), Some("before"));
        assert_eq!(after.setting("active", "mode").as_deref(), Some("after"));
        let missing = LoadContext::new(DataLoadOrigin::ServerDataLoad, inputs, ["active"]);
        assert_eq!(missing.setting("active", "mode"), None);
        assert!(!root.exists());
    }
}
