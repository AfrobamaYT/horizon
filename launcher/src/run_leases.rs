//! Shared tree-lock primitives. Persistent files are lock inodes, never busy markers.
use crate::native_cache::{locked, Error, LockMode, Locked, Release};
use cap_fs_ext::{DirExt, FollowSymlinks, MetadataExt, OpenOptionsFollowExt};
use cap_std::fs::{Dir, OpenOptions};
use sha2::{Digest, Sha256};
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

pub(crate) struct Registry {
    directory: Dir,
    gate: Locked,
    release: Release,
}

// Field order releases the tree before the last reference can release the gate.
pub(crate) struct Claim {
    tree: Locked,
    registry: Arc<Registry>,
}

impl Claim {
    pub(crate) fn files(&self) -> [&fs::File; 2] {
        [self.tree.file(), self.registry.gate.file()]
    }
}

impl Registry {
    fn at(data: &Path) -> Result<Arc<Self>, Error> {
        Self::with_release(data, Release::Unlock)
    }

    pub(crate) fn with_release(data: &Path, release: Release) -> Result<Arc<Self>, Error> {
        crate::native_cache::checked(data, true)?;
        fs::create_dir_all(data)?;
        let data = Dir::open_ambient_dir(data.canonicalize()?, cap_std::ambient_authority())?;
        filesystem::local(&data)?;
        match data.create_dir("run-leases") {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        let directory = data.open_dir_nofollow("run-leases")?;
        let gate = open_lock(&directory, "gate", LockMode::Shared, release)?;
        Ok(Arc::new(Self {
            directory,
            gate,
            release,
        }))
    }

    pub(crate) fn open() -> Result<Arc<Self>, Error> {
        Self::at(&crate::backend::data_dir())
    }

    /// Gate first; identity comes from a held directory, never a mutable path.
    pub(crate) fn lock(self: &Arc<Self>, tree: &Dir, mode: LockMode) -> Result<Claim, Error> {
        filesystem::local(tree)?;
        let metadata = tree.dir_metadata()?;
        let identity = format!("{}:{}", metadata.dev(), metadata.ino());
        let key = format!("{:x}", Sha256::digest(identity.as_bytes()));
        let tree = open_lock(&self.directory, &key, mode, self.release)?;
        Ok(Claim {
            tree,
            registry: self.clone(),
        })
    }
}

// A creator must be compatible with shared readers even before its handle closes.
fn initialize_lock(directory: &Dir, key: &str) -> std::io::Result<cap_std::fs::File> {
    let mut initial = OpenOptions::new();
    initial
        .read(true)
        .write(true)
        .create_new(true)
        .follow(FollowSymlinks::No);
    #[cfg(windows)]
    {
        use cap_std::fs::OpenOptionsExt;
        initial
            .access_mode(windows_sys::Win32::Foundation::GENERIC_READ)
            .share_mode(1);
    }
    directory.open_with(key, &initial)
}

fn open_lock(
    directory: &Dir,
    key: &str,
    mode: LockMode,
    release: Release,
) -> Result<Locked, Error> {
    match initialize_lock(directory, key) {
        Ok(file) => drop(file),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    #[cfg(not(windows))]
    options.write(true);
    #[cfg(windows)]
    {
        use cap_std::fs::OpenOptionsExt;
        options
            .write(matches!(mode, LockMode::Exclusive))
            .share_mode(if matches!(mode, LockMode::Shared) {
                1
            } else {
                0
            });
    }
    let file = directory
        .open_with(key, &options)
        .map_err(|error| {
            #[cfg(windows)]
            if matches!(error.raw_os_error(), Some(32 | 33)) {
                return Error::Busy;
            }
            Error::Io(error)
        })?
        .into_std();
    if !file.metadata()?.is_file() {
        return Err(Error::Unsafe(key.into()));
    }
    let lock = locked(file, mode)?;
    Ok(match release {
        Release::Unlock => lock,
        Release::Close => lock.close_only(),
    })
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

mod filesystem;

#[cfg(test)]
mod tests;
