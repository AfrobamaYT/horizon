//! Persistent, account-trusted Wasmtime artifacts. Only the game supplies this root.
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use wasmtime::{Engine, Module};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Source {
    Compiled,
    Cached,
}

pub(crate) struct ModuleCache {
    root: PathBuf,
    disabled: Option<String>,
    warned: AtomicBool,
}

impl ModuleCache {
    /// Hits are mapped by `Module::deserialize_open_file`, as in `deserialize_file`.
    /// Unix checks folder ownership and write permissions without following a symlink.
    /// Each opened entry must be regular, owned by this user, not group/other writable,
    /// and have exactly one link. Windows checks the folder is under LOCALAPPDATA or
    /// USERPROFILE and neither folder nor opened entry is a reparse point; entries must
    /// be regular. Windows relies on inherited profile ACLs, rather than inspecting ACLs.
    pub(crate) fn new(root: PathBuf) -> Self {
        let disabled = prepare_folder(&root)
            .err()
            .map(|why| format!("folder {}: {why}", root.display()));
        if let Some(why) = &disabled {
            log::warn!("wasm module cache disabled: {why}");
        }
        Self {
            root,
            warned: AtomicBool::new(disabled.is_some()),
            disabled,
        }
    }

    pub(crate) fn key(engine: &Engine, bytes: &[u8]) -> String {
        let mut compatibility = std::collections::hash_map::DefaultHasher::new();
        engine
            .precompile_compatibility_hash()
            .hash(&mut compatibility);
        crate::runtime::ENGINE_CONFIG.hash(&mut compatibility);
        let mut digest = Sha256::new();
        digest.update(Sha256::digest(bytes));
        digest.update(compatibility.finish().to_le_bytes());
        format!("{:x}", digest.finalize())
    }

    pub(crate) fn load_or_compile(
        &self,
        engine: &Engine,
        bytes: &[u8],
    ) -> wasmtime::Result<(Module, Source)> {
        if self.disabled.is_some() {
            return Module::new(engine, bytes).map(|module| (module, Source::Compiled));
        }
        let path = self
            .root
            .join(format!("{}.cwasm", Self::key(engine, bytes)));
        match trusted_entry(&path) {
            Ok(file) => {
                // SAFETY: the account is the trust boundary. Unix folder and entry are owned
                // by this user and not group/other writable. We verify the opened handle,
                // not the path: a directory swap can only present another user's file
                // (refused), or this account's trusted file. Artifacts are produced by
                // Module::serialize for this engine; Wasmtime checks header/compatibility.
                // A mapped entry must remain unchanged for the Module lifetime. Writers
                // only temp-and-rename (including corrupt-entry recovery), giving a new
                // inode and preserving every live mapping; they never rewrite in place.
                // Unix also rejects hard links; Windows relies on inherited profile ACLs.
                match unsafe { Module::deserialize_open_file(engine, file) } {
                    Ok(module) => return Ok((module, Source::Cached)),
                    Err(why) => self.warn_once(&path, &why),
                }
            }
            Err(why) if why.kind() == io::ErrorKind::NotFound => {}
            Err(why) => self.warn_once(&path, &why),
        }
        let module = Module::new(engine, bytes)?;
        if let Err(why) = self.write(&path, &module) {
            log::debug!("wasm module cache write {}: {why}", path.display());
        }
        Ok((module, Source::Compiled))
    }

    fn warn_once(&self, path: &Path, why: &dyn std::fmt::Display) {
        if !self.warned.swap(true, Ordering::Relaxed) {
            log::warn!(
                "wasm module cache refused {}: {why}; compiling wasm",
                path.display()
            );
        }
    }

    fn write(&self, path: &Path, module: &Module) -> wasmtime::Result<()> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let bytes = module.serialize()?;
        let (temporary, mut file) = loop {
            let temporary = self.root.join(format!(
                ".{}-{}.tmp",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&temporary) {
                Ok(file) => break (temporary, file),
                Err(why) if why.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(why) => return Err(why.into()),
            }
        };
        let result = (|| {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(fs::Permissions::from_mode(0o600))?;
            }
            file.write_all(&bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result?;
        Ok(())
    }
}

fn prepare_folder(root: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    if let Some(parent) = root
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        builder.create(parent)?;
    }
    builder.recursive(false);
    let created = match builder.create(root) {
        Ok(()) => true,
        Err(why) if why.kind() == io::ErrorKind::AlreadyExists => false,
        Err(why) => return Err(why),
    };
    #[cfg(unix)]
    if created {
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
        let folder = OpenOptions::new()
            .read(true)
            .custom_flags(
                (rustix::fs::OFlags::DIRECTORY | rustix::fs::OFlags::NOFOLLOW).bits() as i32,
            )
            .open(root)?;
        if folder.metadata()?.uid() != rustix::process::getuid().as_raw() {
            return Err(io::Error::other("new folder is not owned by this user"));
        }
        folder.set_permissions(fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(not(unix))]
    let _ = created;
    let metadata = fs::symlink_metadata(root)?;
    if !metadata.is_dir() {
        return Err(io::Error::other("not a directory"));
    }
    check_permissions(&metadata)?;
    #[cfg(windows)]
    {
        let canonical = root.canonicalize()?;
        let under_profile = ["LOCALAPPDATA", "USERPROFILE"]
            .iter()
            .filter_map(std::env::var_os)
            .filter_map(|path| PathBuf::from(path).canonicalize().ok())
            .any(|profile| canonical.starts_with(profile));
        if !under_profile {
            return Err(io::Error::other("outside the user's profile"));
        }
    }
    Ok(())
}

fn trusted_entry(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(
            (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32,
        );
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_FLAG_OPEN_REPARSE_POINT; FILE_GENERIC_READ | FILE_GENERIC_EXECUTE.
        options
            .custom_flags(0x00200000)
            .access_mode(0x00120089 | 0x001200a0);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(io::Error::other("entry is not a regular file"));
    }
    check_permissions(&metadata)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err(io::Error::other("entry has multiple hard links"));
        }
    }
    Ok(file)
}

fn check_permissions(metadata: &fs::Metadata) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.uid() != rustix::process::getuid().as_raw() || metadata.mode() & 0o022 != 0 {
            return Err(io::Error::other(
                "not owned by this user or writable by group/others",
            ));
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(io::Error::other("reparse point"));
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = metadata;
        return Err(io::Error::other(
            "ownership checks unavailable on this platform",
        ));
    }
    #[allow(unreachable_code)]
    Ok(())
}

#[cfg(test)]
#[path = "module_cache_tests.rs"]
mod tests;
