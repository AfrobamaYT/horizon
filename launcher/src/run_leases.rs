//! Shared tree-lock primitives. Persistent files are lock inodes, never busy markers.
use crate::native_cache::{Error, LockMode, Release};
use cap_std::fs::Dir;
use std::{fs, path::Path, process::Command, sync::Arc};

pub(crate) const SOURCE: &str = "HORIZON_RUNTIME_SOURCE";

pub(crate) fn mark(command: &mut Command, source: &Path) {
    command.env(SOURCE, source);
}

// Outer None inherits the environment; inner None explicitly removes the variable.
fn override_value<'a>(command: &'a Command, key: &str) -> Option<Option<&'a std::ffi::OsStr>> {
    command
        .get_envs()
        .find(|(name, _)| *name == key)
        .map(|(_, value)| value)
}

// This adapter owns localized launcher errors; the kernel protocol lives below game/launcher.
pub(crate) struct Registry(Arc<horizon_mod_package::runtime_leases::Registry>);
pub(crate) use horizon_mod_package::runtime_leases::Claim;
impl Registry {
    fn at(data: &Path) -> Result<Arc<Self>, Error> {
        Self::with_release(data, Release::Unlock)
    }
    pub(crate) fn with_release(data: &Path, release: Release) -> Result<Arc<Self>, Error> {
        horizon_mod_package::runtime_leases::Registry::with_release(data, release)
            .map(|registry| Arc::new(Self(registry)))
            .map_err(Into::into)
    }
    pub(crate) fn open() -> Result<Arc<Self>, Error> {
        Self::at(&crate::backend::data_dir())
    }
    pub(crate) fn lock(self: &Arc<Self>, tree: &Dir, mode: LockMode) -> Result<Claim, Error> {
        self.0.lock(tree, mode).map_err(Into::into)
    }
}

/// Launcher-owned primitives-in-use; this is not a standalone cleanup safety guarantee.
pub(crate) fn shared(command: &Command) -> Result<Vec<Claim>, Error> {
    let logs = override_value(command, "HORIZON_HOME").map_or_else(
        || std::env::var_os("HORIZON_HOME").map(std::path::PathBuf::from),
        |value| value.map(std::path::PathBuf::from),
    );
    let source = override_value(command, SOURCE)
        .flatten()
        .map(Path::new)
        .or_else(|| {
            let program = Path::new(command.get_program());
            program.is_absolute().then(|| program.parent()).flatten()
        });
    let mut paths = Vec::new();
    if let Some(home) = command.get_current_dir() {
        let home = home.canonicalize()?;
        for relative in [".horizon-cache", "wasm-cache", "cache"] {
            crate::storage::mark_used(&home.join(relative))?;
        }
        use horizon_mod_package::{mod_list, Package};
        for folder in [mod_list::DIR, mod_list::NATIVE_DIR] {
            for path in horizon_mod_package::find(&home.join(folder)) {
                if let Ok(package) = Package::read(&path) {
                    if crate::browser::java::is_enabled(&home, &package.manifest.id) {
                        for jar in package.manifest.jar {
                            crate::storage::mark_used(&jar.cache_path(&home.join("jars")))?;
                        }
                    }
                }
            }
        }
        paths.push(home);
    }
    if let Some(logs) = logs.filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(&logs)?;
        paths.push(logs.canonicalize()?);
    }
    if let Some(source) = source {
        let source = source.canonicalize()?;
        if let Some(parent) = source.parent() {
            paths.push(parent.to_owned());
        }
        paths.push(source);
    }
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let registry = Registry::open()?;
    for path in &paths {
        crate::storage::mark_used(path)?;
    }
    paths
        .into_iter()
        .map(|path| {
            let directory = Dir::open_ambient_dir(path, cap_std::ambient_authority())?;
            registry.lock(&directory, LockMode::Shared)
        })
        .collect()
}

#[cfg(test)]
mod tests;
