//! Data-only inputs carried from an owning load boundary to native load hooks.
use std::{
    collections::{BTreeMap, HashMap},
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
    registries: BTreeMap<String, Arc<[String]>>,
    /// The fluid tags a recipe load reads for its ingredients' `#tags`:
    /// each tag id to its fluids, in the sources' order. Empty until a load
    /// that reads recipe resources fills them.
    fluid_tags: Arc<[(String, Arc<[String]>)]>,
    settings_root: PathBuf,
}

impl LoadInputs {
    pub fn new(registries: BTreeMap<String, Vec<String>>, settings_root: PathBuf) -> Self {
        Self {
            registries: registries
                .into_iter()
                .map(|(name, ids)| (name, ids.into()))
                .collect(),
            fluid_tags: Arc::from([]),
            settings_root,
        }
    }
    /// The inputs with the fluid `tags` a recipe load reads
    /// (`data/<ns>/tags/fluid`), each tag's fluids in the sources' order.
    pub fn with_fluid_tags(mut self, tags: HashMap<String, Vec<String>>) -> Self {
        self.fluid_tags = tags
            .into_iter()
            .map(|(tag, fluids)| (tag, fluids.into()))
            .collect();
        self
    }
}

/// A load's registry snapshot with the settings captured for its mods.
struct Snapshot {
    inputs: Arc<LoadInputs>,
    settings: BTreeMap<String, BTreeMap<String, String>>,
}

impl Snapshot {
    /// Captures settings for `mod_ids`; the registry inputs are shared, not scanned again.
    fn capture<'a>(inputs: Arc<LoadInputs>, mod_ids: impl IntoIterator<Item = &'a str>) -> Self {
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
        Self { inputs, settings }
    }

    fn registry(&self, name: &str) -> Option<Arc<[String]>> {
        self.inputs.registries.get(name).cloned()
    }

    /// The fluid tags the inputs carry, each tag's fluids in the sources'
    /// order; empty until a load that reads recipe resources fills them.
    fn fluid_tags(&self) -> &Arc<[(String, Arc<[String]>)]> {
        &self.inputs.fluid_tags
    }

    fn setting(&self, mod_id: &str, key: &str) -> Option<String> {
        self.settings.get(mod_id)?.get(key).cloned()
    }
}

/// Origin and borrowed engine inputs for one owning load. This contains no
/// game, world handles or mutable registry access.
pub struct LoadContext {
    origin: DataLoadOrigin,
    snapshot: Snapshot,
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
        Self {
            origin,
            snapshot: Snapshot::capture(inputs, mod_ids),
        }
    }

    /// A new owning load of `origin` over this load's registry inputs, with fresh settings.
    pub fn for_load<'a>(
        &self,
        origin: DataLoadOrigin,
        mod_ids: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        Self::new(origin, self.snapshot.inputs.clone(), mod_ids)
    }

    /// This load over the same registries and settings, with the fluid `tags`
    /// its recipe pass reads (`data/<ns>/tags/fluid`): what that pass's
    /// ingredient hooks resolve fluid `#tags` with. A recipe load builds it
    /// where it knows its overlay; loads that read no recipe resources share
    /// the inputs as-is, with no tags.
    pub fn with_fluid_tags(&self, tags: HashMap<String, Vec<String>>) -> Self {
        let inputs = Arc::new(LoadInputs {
            registries: self.snapshot.inputs.registries.clone(),
            fluid_tags: tags
                .into_iter()
                .map(|(tag, fluids)| (tag, fluids.into()))
                .collect(),
            settings_root: self.snapshot.inputs.settings_root.clone(),
        });
        Self {
            origin: self.origin,
            snapshot: Snapshot {
                inputs,
                settings: self.snapshot.settings.clone(),
            },
        }
    }

    /// The fluid tags this load's recipe pass reads: each tag id to its
    /// fluids, in the sources' order. Empty where no recipe pass filled them.
    pub fn fluid_tags(&self) -> &Arc<[(String, Arc<[String]>)]> {
        self.snapshot.fluid_tags()
    }

    /// The context of a menu, recipe or player-list loop of `side`, over this
    /// load's registry inputs, with fresh settings. Built where the side's loop
    /// owner starts or reloads its resources, never by a reader.
    pub fn loop_context<'a>(
        &self,
        side: LoopSide,
        mod_ids: impl IntoIterator<Item = &'a str>,
    ) -> LoopContext {
        LoopContext {
            side,
            snapshot: Snapshot::capture(self.snapshot.inputs.clone(), mod_ids),
        }
    }

    pub fn origin(&self) -> DataLoadOrigin {
        self.origin
    }

    /// Names in registry ID order, from the load's registry snapshot.
    pub fn registry(&self, name: &str) -> Option<Arc<[String]>> {
        self.snapshot.registry(name)
    }

    /// A loaded mod's setting captured at this load boundary, with no file I/O.
    pub fn setting(&self, mod_id: &str, key: &str) -> Option<String> {
        self.snapshot.setting(mod_id, key)
    }
}

/// The side a loop context serves, chosen where the context is built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoopSide {
    Server,
    Client,
}

/// Registries, settings and the side for hooks fired inside a menu, recipe or
/// player-list loop, where no game is at hand. It holds no game, world handle
/// or mutable registry access, and is built once per resource generation.
pub struct LoopContext {
    side: LoopSide,
    snapshot: Snapshot,
}

impl LoopContext {
    pub fn side(&self) -> LoopSide {
        self.side
    }

    /// Names in registry ID order, from the registry snapshot it was built over.
    pub fn registry(&self, name: &str) -> Option<Arc<[String]>> {
        self.snapshot.registry(name)
    }

    /// A loaded mod's setting captured when this context was built, with no file I/O.
    pub fn setting(&self, mod_id: &str, key: &str) -> Option<String> {
        self.snapshot.setting(mod_id, key)
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

    #[test]
    fn a_loop_context_shares_the_registries_and_captures_its_own_settings() {
        let root = std::env::temp_dir().join(format!(
            "horizon-loop-settings-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let mod_dir = root.join("active");
        fs::create_dir_all(&mod_dir).unwrap();
        fs::write(mod_dir.join("mode"), "one").unwrap();
        let registries = BTreeMap::from([("item".to_owned(), vec!["a:b".to_owned()])]);
        let inputs = Arc::new(LoadInputs::new(registries, root.clone()));
        let load = LoadContext::new(DataLoadOrigin::ClientPreload, inputs, ["active"]);

        let client = load.loop_context(LoopSide::Client, ["active"]);
        fs::write(mod_dir.join("mode"), "two").unwrap();
        let server = load.loop_context(LoopSide::Server, ["active"]);

        assert_eq!(client.side(), LoopSide::Client);
        assert_eq!(server.side(), LoopSide::Server);
        assert_eq!(
            client.registry("item").as_deref(),
            Some(&["a:b".to_owned()][..])
        );
        assert_eq!(server.registry("item"), client.registry("item"));
        assert_eq!(client.setting("active", "mode").as_deref(), Some("one"));
        assert_eq!(server.setting("active", "mode").as_deref(), Some("two"));
        assert_eq!(load.setting("active", "mode").as_deref(), Some("one"));
        fs::remove_dir_all(&root).unwrap();
    }
}
