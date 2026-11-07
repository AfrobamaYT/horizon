//! Disposable launcher storage. Player-owned files are never accepted as cleanup targets.
pub use crate::native_cache::Error;
use crate::native_cache::{checked, linked, Cache};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// The five rows on the Storage page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Category {
    BuildFiles,
    PrebuiltGames,
    SpareGameBuilds,
    RustCompiler,
    OtherCaches,
}

/// Fixed build-home caches. Instance and version inventory adds its protected entries separately.
#[derive(Clone, Copy, Debug)]
pub enum BuildCache {
    Stage,
    PendingStage,
    NewStage,
    Patched,
    Source,
    Tools,
    PrebuiltOutput,
    InternalsSync,
    Prebuilt,
    ServerBuilds,
    Rust,
    Cargo,
    Scratch,
}

impl BuildCache {
    fn relative(self) -> &'static str {
        match self {
            Self::Stage => "native/stage",
            Self::PendingStage => "native/stage.pending",
            Self::NewStage => "native/stage.new",
            Self::Patched => "native/patched",
            Self::Source => "native/source",
            Self::Tools => "native/tools",
            Self::PrebuiltOutput => "native/prebuilt-build",
            Self::InternalsSync => "native/internals-sync",
            Self::Prebuilt => "native/prebuilt",
            Self::ServerBuilds => "native/server-builds",
            Self::Rust => "rust",
            Self::Cargo => "cargo",
            Self::Scratch => "tmp",
        }
    }

    pub fn category(self) -> Category {
        match self {
            Self::Prebuilt => Category::PrebuiltGames,
            Self::ServerBuilds => Category::SpareGameBuilds,
            Self::Rust | Self::Cargo | Self::Scratch => Category::RustCompiler,
            _ => Category::BuildFiles,
        }
    }
}

/// A build-home inventory and cleanup operation holds the native build lease throughout.
pub struct Storage {
    root: PathBuf,
    lease: Cache,
}

impl Storage {
    pub fn open() -> Result<Self, Error> {
        Self::at(&crate::backend::build_dir())
    }

    fn at(root: &Path) -> Result<Self, Error> {
        let lease = Cache::acquire(root)?;
        Ok(Self {
            root: root.canonicalize()?,
            lease,
        })
    }

    fn path(&self, cache: BuildCache) -> Result<PathBuf, Error> {
        let mut path = self.root.clone();
        // Validate every ancestor, not only the final directory (Windows junctions included).
        if path.canonicalize()? != path {
            return Err(Error::Unsafe(path));
        }
        for part in cache.relative().split('/') {
            path.push(part);
            checked(&path, true)?;
        }
        Ok(path)
    }

    pub fn bytes(&self, cache: BuildCache) -> Result<u64, Error> {
        allocated_bytes(&self.path(cache)?)
    }

    /// Detach on the same volume before removal. Failed removals remain for the next sweep.
    pub fn clear(&self, cache: BuildCache) -> Result<u64, Error> {
        self.trash(&self.path(cache)?)
    }

    pub fn clear_targets(&self) -> Result<u64, Error> {
        // Retain Cargo lock inodes; never rename an entire locked target directory.
        self.lease.clear()
    }

    fn trash(&self, path: &Path) -> Result<u64, Error> {
        let bytes = allocated_bytes(path)?;
        if !path.try_exists()? {
            return Ok(0);
        }
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let trash = self.root.join(format!(
            ".trash-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::rename(path, &trash)?;
        remove(&trash)?;
        Ok(bytes)
    }

    pub fn sweep(&self) -> Result<(), Error> {
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            if entry.file_name().to_string_lossy().starts_with(".trash-") {
                allocated_bytes(&entry.path())?;
                remove(&entry.path())?;
            }
        }
        Ok(())
    }
}

pub(crate) fn allocated_bytes(path: &Path) -> Result<u64, Error> {
    walk(path, &mut HashSet::new())
}

fn walk(path: &Path, seen: &mut HashSet<(u64, u64)>) -> Result<u64, Error> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.into()),
    };
    if linked(&meta) || (!meta.is_file() && !meta.is_dir()) {
        return Err(Error::Unsafe(path.to_owned()));
    }
    let (id, bytes) = allocation(path, &meta)?;
    if !seen.insert(id) {
        return Ok(0);
    }
    let mut total = bytes;
    if meta.is_dir() {
        for entry in fs::read_dir(path)? {
            total = total.saturating_add(walk(&entry?.path(), seen)?);
        }
    }
    Ok(total)
}

#[cfg(unix)]
fn allocation(_path: &Path, meta: &fs::Metadata) -> Result<((u64, u64), u64), Error> {
    use std::os::unix::fs::MetadataExt;
    Ok(((meta.dev(), meta.ino()), meta.blocks().saturating_mul(512)))
}

#[cfg(windows)]
fn allocation(path: &Path, _meta: &fs::Metadata) -> Result<((u64, u64), u64), Error> {
    use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::{
        FileStandardInfo, GetFileInformationByHandle, GetFileInformationByHandleEx,
        BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_STANDARD_INFO,
    };
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    let mut standard: FILE_STANDARD_INFO = unsafe { std::mem::zeroed() };
    // Both structures are OS-owned output buffers; file keeps the handle live.
    unsafe {
        if GetFileInformationByHandle(file.as_raw_handle(), &mut info) == 0
            || GetFileInformationByHandleEx(
                file.as_raw_handle(),
                FileStandardInfo,
                (&mut standard as *mut FILE_STANDARD_INFO).cast(),
                std::mem::size_of::<FILE_STANDARD_INFO>() as u32,
            ) == 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    if info.dwFileAttributes & 0x400 != 0 {
        return Err(Error::Unsafe(path.to_owned()));
    }
    Ok((
        (
            info.dwVolumeSerialNumber as u64,
            ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64,
        ),
        standard.AllocationSize as u64,
    ))
}

fn remove(path: &Path) -> Result<(), Error> {
    let meta = fs::symlink_metadata(path)?;
    if linked(&meta) {
        return Err(Error::Unsafe(path.to_owned()));
    }
    if meta.is_dir() {
        for entry in fs::read_dir(path)? {
            remove(&entry?.path())?;
        }
    }
    #[cfg(windows)]
    if meta.permissions().readonly() {
        let mut permissions = meta.permissions();
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions)?;
    }
    if meta.is_dir() {
        fs::remove_dir(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "storage/tests.rs"]
mod tests;
