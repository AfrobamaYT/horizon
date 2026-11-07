//! Snapshot cleanup under held directory capabilities. Cargo has no launcher exclusion contract:
//! pin a `.cargo-lock` in every visited directory before enumerating it, and keep those inodes
//! and their ancestry forever rather than strand a Cargo already waiting on an unlinked lock.
//! Newly created entries are never in the deletion plan. Stamps retain files seen changed before
//! deletion, best effort; hostile same-user swaps inside the cache are not a security boundary.
pub use crate::native_cache::Error;
use crate::native_cache::{locked, Locked};
use cap_fs_ext::{DirExt, FollowSymlinks, MetadataExt, OpenOptionsFollowExt};
use cap_std::fs::{Dir, Metadata, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

const LOCKS: [&str; 3] = [".cargo-lock", ".package-cache", ".package-cache-mutate"];

#[derive(PartialEq)]
struct Stamp {
    device: u64,
    inode: u64,
    bytes: u64,
    modified: cap_std::time::SystemTime,
}
impl Stamp {
    fn read(meta: &Metadata) -> Result<Self, Error> {
        Ok(Self {
            device: meta.dev(),
            inode: meta.ino(),
            bytes: meta.len(),
            modified: meta.modified()?,
        })
    }
}

struct Output {
    parent: Arc<Dir>,
    name: PathBuf,
    stamp: Stamp,
}

/// Holds every directory lock until its last snapshotted output is removed.
pub struct Plan {
    locks: Vec<Locked>,
    outputs: Vec<Output>,
}

impl Plan {
    pub fn pin(directory: &Dir) -> Result<Self, Error> {
        let mut plan = Self {
            locks: Vec::new(),
            outputs: Vec::new(),
        };
        plan.visit(directory)?;
        Ok(plan)
    }

    pub fn pin_file(directory: &Dir, name: &Path) -> Result<Self, Error> {
        let mut plan = Self {
            locks: Vec::new(),
            outputs: Vec::new(),
        };
        plan.lock_directory(directory)?;
        let meta = directory.symlink_metadata(name)?;
        if !meta.is_file()
            || meta.file_type().is_symlink()
            || name.components().count() != 1
            || LOCKS.iter().any(|lock| name == Path::new(lock))
            || name == Path::new(".build.lock")
        {
            return Err(Error::Unsafe(name.to_owned()));
        }
        plan.outputs.push(Output {
            parent: Arc::new(directory.try_clone()?),
            name: name.to_owned(),
            stamp: Stamp::read(&meta)?,
        });
        Ok(plan)
    }

    fn lock_directory(&mut self, directory: &Dir) -> Result<(), Error> {
        let mut options = OpenOptions::new();
        options
            .read(true)
            .write(true)
            .create(true)
            .follow(FollowSymlinks::No);
        for name in LOCKS {
            self.locks
                .push(locked(directory.open_with(name, &options)?.into_std())?);
        }
        Ok(())
    }

    fn visit(&mut self, directory: &Dir) -> Result<(), Error> {
        self.lock_directory(directory)?;
        let parent = Arc::new(directory.try_clone()?);
        for entry in directory.entries()? {
            let name = PathBuf::from(entry?.file_name());
            if LOCKS.iter().any(|lock| name == Path::new(lock)) || name == Path::new(".build.lock")
            {
                continue;
            }
            let meta = directory.symlink_metadata(&name)?;
            if meta.file_type().is_symlink() {
                return Err(Error::Unsafe(name));
            }
            if meta.is_dir() {
                self.visit(&directory.open_dir_nofollow(&name)?)?;
            } else if meta.is_file() {
                self.outputs.push(Output {
                    parent: Arc::clone(&parent),
                    name,
                    stamp: Stamp::read(&meta)?,
                });
            } else {
                return Err(Error::Unsafe(name));
            }
        }
        Ok(())
    }

    /// Trash files into the caller's held root on the same volume. Never rename their ancestry.
    /// Changed or missing entries remain untouched; callers measure actual residual bytes.
    pub fn clear(self, trash: &Dir) -> Result<(), Error> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        for output in &self.outputs {
            let meta = match output.parent.symlink_metadata(&output.name) {
                Ok(meta) => meta,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            if !meta.is_file() || Stamp::read(&meta)? != output.stamp {
                continue;
            }
            let name = format!(
                ".trash-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            );
            output.parent.rename(&output.name, trash, &name)?;
            #[cfg(windows)]
            if meta.permissions().readonly() {
                let mut permissions = meta.permissions();
                permissions.set_readonly(false);
                use cap_std::fs::OpenOptionsExt;
                use windows_sys::Win32::Storage::FileSystem::FILE_WRITE_ATTRIBUTES;
                let mut options = OpenOptions::new();
                options
                    .read(true)
                    .access_mode(FILE_WRITE_ATTRIBUTES)
                    .follow(FollowSymlinks::No);
                trash
                    .open_with(&name, &options)?
                    .set_permissions(permissions)?;
            }
            trash.remove_file(&name)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "cache_cleanup/tests.rs"]
mod tests;
