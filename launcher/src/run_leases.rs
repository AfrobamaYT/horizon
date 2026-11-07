//! Kernel-owned game lifetime locks. Persistent files are lock inodes, never busy markers.
use crate::native_cache::{locked, Error, LockMode, Locked};
use cap_fs_ext::{DirExt, FollowSymlinks, MetadataExt, OpenOptionsFollowExt};
use cap_std::fs::{Dir, OpenOptions};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

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

pub(crate) struct Registry {
    directory: Dir,
}

impl Registry {
    fn at(data: &Path) -> Result<Self, Error> {
        crate::native_cache::checked(data, true)?;
        fs::create_dir_all(data)?;
        let data = Dir::open_ambient_dir(data.canonicalize()?, cap_std::ambient_authority())?;
        match data.create_dir("run-leases") {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        Ok(Self {
            directory: data.open_dir_nofollow("run-leases")?,
        })
    }

    pub(crate) fn open() -> Result<Self, Error> {
        Self::at(&crate::backend::data_dir())
    }

    /// Identity comes from the held directory, not a path that could now name a replaced root.
    pub(crate) fn lock(&self, tree: &Dir, mode: LockMode) -> Result<Locked, Error> {
        let metadata = tree.dir_metadata()?;
        let identity = format!("{}:{}", metadata.dev(), metadata.ino());
        let key = format!("{:x}", Sha256::digest(identity.as_bytes()));
        let mut options = OpenOptions::new();
        options
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .follow(FollowSymlinks::No);
        let file = self.directory.open_with(&key, &options)?.into_std();
        if !file.metadata()?.is_file() {
            return Err(Error::Unsafe(key.into()));
        }
        locked(file, mode)
    }
}

/// Acquire before spawn. The caller retains these guards through the child's exit.
pub(crate) fn shared(command: &Command) -> Result<Vec<Locked>, Error> {
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
        paths.push(home.canonicalize()?);
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
