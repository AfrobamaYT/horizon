use super::*;
use crate::native_cache::{locked, Locked};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::{Dir, OpenOptions};

// Inspect and lease every Cargo profile before detaching or deleting any of the tree.
fn guard(dir: &Dir, locks: &mut Vec<Locked>) -> Result<(), Error> {
    for entry in dir.entries()? {
        let entry = entry?;
        let name = entry.file_name();
        let meta = dir.symlink_metadata(&name)?;
        if meta.file_type().is_symlink() {
            return Err(Error::Unsafe(name.into()));
        }
        if meta.is_dir() {
            guard(&dir.open_dir_nofollow(&name)?, locks)?;
        } else if name == ".cargo-lock" {
            let mut options = OpenOptions::new();
            options.read(true).write(true).follow(FollowSymlinks::No);
            locks.push(locked(dir.open_with(&name, &options)?.into_std())?);
        } else if !meta.is_file() {
            return Err(Error::Unsafe(name.into()));
        }
    }
    Ok(())
}

// Every resolution is relative to a held directory. A swapped symlink is never traversed.
fn remove(dir: &Dir, name: &Path) -> Result<bool, Error> {
    let meta = dir.symlink_metadata(name)?;
    if meta.file_type().is_symlink() {
        return Err(Error::Unsafe(name.to_owned()));
    }
    if name == Path::new(".cargo-lock") {
        return Ok(false);
    }
    let mut empty = true;
    if meta.is_dir() {
        let child = dir.open_dir_nofollow(name)?;
        for entry in child.entries()? {
            empty &= remove(&child, Path::new(&entry?.file_name()))?;
        }
    }
    if !empty {
        return Ok(false);
    }
    #[cfg(windows)]
    if meta.permissions().readonly() {
        let mut permissions = meta.permissions();
        permissions.set_readonly(false);
        dir.set_permissions(name, permissions)?;
    }
    if meta.is_dir() {
        dir.remove_dir(name)?;
    } else {
        dir.remove_file(name)?;
    }
    Ok(true)
}

pub(super) fn clear(dir: &Dir, name: &Path) -> Result<(), Error> {
    let mut locks = Vec::new();
    let meta = dir.symlink_metadata(name)?;
    if meta.file_type().is_symlink() {
        return Err(Error::Unsafe(name.to_owned()));
    }
    if meta.is_dir() {
        guard(&dir.open_dir_nofollow(name)?, &mut locks)?;
    }
    if locks.is_empty() {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let trash = PathBuf::from(format!(
            ".trash-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        dir.rename(name, dir, &trash)?;
        remove(dir, &trash)?;
    } else {
        // Never unlink Cargo's lock inode or its ancestors, even when the lock is idle.
        remove(dir, name)?;
    }
    Ok(())
}

pub(super) fn clear_path(directory: &Dir, path: &Path) -> Result<(), Error> {
    let mut parent = directory.try_clone()?;
    let mut parts = path.components().peekable();
    while let Some(part) = parts.next() {
        if parts.peek().is_none() {
            match parent.symlink_metadata(part.as_os_str()) {
                Ok(_) => clear(&parent, Path::new(part.as_os_str()))?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(error.into()),
            }
        } else {
            match parent.open_dir_nofollow(part.as_os_str()) {
                Ok(directory) => parent = directory,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(error.into()),
            }
        }
    }
    Ok(())
}
