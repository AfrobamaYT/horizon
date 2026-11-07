//! The CLI and Storage page share one fixed-path inventory.
#[cfg(test)]
#[path = "inventory_regressions.rs"]
mod regressions;
use super::*;
use crate::{backend, instances, native_mods};
use serde::Serialize;

const BUILD_CACHES: [BuildCache; 13] = [
    BuildCache::Stage,
    BuildCache::PendingStage,
    BuildCache::NewStage,
    BuildCache::Patched,
    BuildCache::Source,
    BuildCache::Tools,
    BuildCache::PrebuiltOutput,
    BuildCache::InternalsSync,
    BuildCache::Prebuilt,
    BuildCache::ServerBuilds,
    BuildCache::Rust,
    BuildCache::Cargo,
    BuildCache::Scratch,
];

#[derive(Serialize)]
pub struct Row {
    pub category: Category,
    pub bytes: u64,
    pub freed_bytes: u64,
    pub remaining_bytes: u64,
}

#[derive(Serialize)]
pub struct Report {
    pub rows: Vec<Row>,
    pub total_bytes: u64,
}

impl Category {
    pub const ALL: [Self; 4] = [
        Self::BuildFiles,
        Self::PrebuiltGames,
        Self::SpareGameBuilds,
        Self::RustCompiler,
    ];
}

struct Entry {
    category: Category,
    directory: Option<Dir>,
    relative: PathBuf,
    path: PathBuf,
}

impl Entry {
    fn fixed(root: &Path, relative: &Path, category: Category) -> Result<Self, Error> {
        let directory = match Dir::open_ambient_dir(root, cap_std::ambient_authority()) {
            Ok(directory) => Some(directory),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        let mut path = root.to_owned();
        checked(root, true)?;
        if root.exists() && root.canonicalize()? != root {
            return Err(Error::Unsafe(path));
        }
        for component in relative.components() {
            let std::path::Component::Normal(component) = component else {
                return Err(Error::Unsafe(relative.to_owned()));
            };
            path.push(component);
            match fs::symlink_metadata(&path) {
                Ok(meta) if linked(&meta) => return Err(Error::Unsafe(path)),
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(Self {
            category,
            directory,
            relative: relative.to_owned(),
            path,
        })
    }

    fn clear(&self) -> Result<(), Error> {
        if !self.path.try_exists()? {
            return Ok(());
        }
        let Some(root) = &self.directory else {
            return Ok(());
        };
        let mut parent = root.try_clone()?;
        for part in self.relative.parent().unwrap_or(Path::new("")).components() {
            parent = parent.open_dir_nofollow(part.as_os_str())?;
        }
        Storage::clear_entry(root, &parent, Path::new(self.relative.file_name().unwrap()))
    }
}

fn add(
    entries: &mut Vec<Entry>,
    root: &Path,
    relative: &str,
    category: Category,
) -> Result<(), Error> {
    entries.push(Entry::fixed(root, Path::new(relative), category)?);
    Ok(())
}

impl Storage {
    fn entries(&self) -> Result<Vec<Entry>, Error> {
        let mut entries = Vec::new();
        for cache in BUILD_CACHES {
            add(&mut entries, &self.root, cache.relative(), cache.category())?;
        }
        let build_files = Category::BuildFiles;
        for name in [
            "built.json",
            "points.json",
            "internals.json",
            "build.log",
            "catalog.partial",
        ] {
            add(
                &mut entries,
                &self.root,
                &format!("native/{name}"),
                build_files,
            )?;
        }
        // The target lock files must remain in place. Their contents are inventoried here but
        // deletion goes through Cache, which also acquires Cargo's locks.
        add(&mut entries, &self.root, "native/target", build_files)?;
        let data = backend::data_dir();
        let played: HashSet<_> = instances::list()
            .into_iter()
            .filter_map(|instance| instance.resolve_version(None).ok())
            .collect();
        for version in backend::installed() {
            if native_mods::is_native(&version)
                && !played.contains(&version)
                && !backend::in_use(&version)
            {
                add(
                    &mut entries,
                    &data,
                    &format!("versions/{version}"),
                    Category::SpareGameBuilds,
                )?;
            }
        }
        Ok(entries)
    }

    pub fn report(&self, clear: Option<Category>, all: bool) -> Result<Report, Error> {
        // Also refuse an external Cargo holding a target lock before starting a cleanup.
        self.lease.bytes()?;
        self.apply(self.entries()?, clear, all)
    }

    fn apply(
        &self,
        entries: Vec<Entry>,
        clear: Option<Category>,
        all: bool,
    ) -> Result<Report, Error> {
        // Validate every descendant before any deletion and deduplicate allocated blocks per row.
        let mut rows = Vec::new();
        for category in Category::ALL {
            let bytes = allocated(&entries, category)?;
            rows.push(Row {
                category,
                bytes,
                freed_bytes: 0,
                remaining_bytes: bytes,
            });
        }
        for entry in &entries {
            if all || clear == Some(entry.category) {
                if entry.path == self.root.join("native/target") {
                    self.lease.clear()?;
                } else {
                    entry.clear()?;
                }
            }
        }
        for row in &mut rows {
            if all || clear == Some(row.category) {
                let remaining = allocated(&entries, row.category)?;
                row.freed_bytes = row.bytes.saturating_sub(remaining);
                row.remaining_bytes = remaining;
            }
        }
        Ok(Report {
            total_bytes: rows
                .iter()
                .fold(0u64, |total, row| total.saturating_add(row.bytes)),
            rows,
        })
    }
}

fn allocated(entries: &[Entry], category: Category) -> Result<u64, Error> {
    let mut seen = HashSet::new();
    entries
        .iter()
        .filter(|entry| entry.category == category)
        .try_fold(0u64, |bytes, entry| {
            Ok(bytes.saturating_add(walk(&entry.path, &mut seen)?))
        })
}
