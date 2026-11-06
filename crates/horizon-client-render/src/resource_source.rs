//! Open resource layers, shared by asset packs and passive mod data.
//! Archive layers retain one reader and never materialize their entries.
//! Diverges from upstream: asset and passive jar-data layers share this source.
use crate::zip::Zip;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

enum Storage {
    Directory(PathBuf),
    Archive(Zip),
    Prefix {
        source: ResourceSource,
        prefix: String,
    },
    Index(BTreeMap<String, Resource>),
}

/// A winning resource keeps its source alive; merged resources keep only their
/// derived bytes. Reading an archive entry remains lazy after pack resolution.
#[derive(Clone)]
pub struct Resource(ResourceStorage);
#[derive(Clone)]
enum ResourceStorage {
    File {
        source: ResourceSource,
        name: String,
    },
    Bytes(Arc<[u8]>),
}
impl Resource {
    pub fn file(source: ResourceSource, name: String) -> Self {
        Self(ResourceStorage::File { source, name })
    }
    pub fn bytes(bytes: Vec<u8>) -> Self {
        Self(ResourceStorage::Bytes(bytes.into()))
    }
    pub fn read(&self) -> io::Result<Vec<u8>> {
        match &self.0 {
            ResourceStorage::File { source, name } => source.read(name),
            ResourceStorage::Bytes(bytes) => Ok(bytes.to_vec()),
        }
    }
}

/// A folder or an open zip/jar, with the same relative-name lookup and listing.
/// Clones share the open archive; construct a new source for a resource reload.
#[derive(Clone)]
pub struct ResourceSource(Arc<Storage>);

/// Prepared registry tables, resource reads and disposable loader caches have
/// separate locations. Archive resources never acquire a filesystem pathname.
#[derive(Clone)]
pub struct ResourceRoot(Arc<Root>);
struct Root {
    prepared: PathBuf,
    cache: PathBuf,
    source: ResourceSource,
}
impl ResourceRoot {
    pub fn directory(path: PathBuf) -> Self {
        Self::new(path.clone(), path.clone(), ResourceSource::directory(path))
    }
    pub fn new(prepared: PathBuf, cache: PathBuf, source: ResourceSource) -> Self {
        Self(Arc::new(Root {
            prepared,
            cache,
            source,
        }))
    }
    pub fn prepared(&self) -> &Path {
        &self.0.prepared
    }
    pub fn cache(&self) -> &Path {
        &self.0.cache
    }
    pub fn source(&self) -> &ResourceSource {
        &self.0.source
    }
}
impl Default for ResourceSource {
    fn default() -> Self {
        Self::indexed(BTreeMap::new())
    }
}

/// A resource name cannot escape its layer. ZIP names already have separators
/// normalized by `Zip`; reject drive/stream names and NULs on every platform.
pub(crate) fn stays_inside(name: &str) -> bool {
    !name.contains([':', '\0', '\\'])
        && name
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn prefix_inside(prefix: &str) -> bool {
    prefix.is_empty() || stays_inside(prefix.trim_end_matches('/'))
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>, active: &mut BTreeSet<PathBuf>) {
    let Ok(canonical) = fs::canonicalize(dir) else {
        return;
    };
    if !active.insert(canonical.clone()) {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        active.remove(&canonical);
        return;
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let directory = match entry.file_type() {
            Ok(kind) if !kind.is_symlink() => kind.is_dir(),
            _ => path.is_dir(),
        };
        if directory {
            walk(root, &path, out, active);
        } else if let Ok(relative) = path.strip_prefix(root) {
            let name = relative.to_string_lossy().replace('\\', "/");
            if stays_inside(&name) {
                out.push(name);
            }
        }
    }
    active.remove(&canonical);
}

impl ResourceSource {
    pub fn indexed(files: BTreeMap<String, Resource>) -> Self {
        Self(Arc::new(Storage::Index(files)))
    }
    pub fn directory(root: PathBuf) -> Self {
        Self(Arc::new(Storage::Directory(root)))
    }
    pub fn archive(path: &Path) -> io::Result<Self> {
        Ok(Self::opened_archive(Zip::open(path)?))
    }
    pub(crate) fn opened_archive(archive: Zip) -> Self {
        Self(Arc::new(Storage::Archive(archive)))
    }
    /// Select an overlay root. Folder paths are trusted repository inputs;
    /// archive prefixes must remain inside that archive.
    pub fn subdirectory(&self, directory: &str) -> io::Result<Self> {
        if directory.is_empty() {
            return Ok(self.clone());
        }
        if let Storage::Directory(root) = self.0.as_ref() {
            return Ok(Self::directory(root.join(directory)));
        }
        if !stays_inside(directory) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unsafe archive prefix",
            ));
        }
        Ok(Self(Arc::new(Storage::Prefix {
            source: self.clone(),
            prefix: format!("{directory}/"),
        })))
    }
    pub fn read(&self, name: &str) -> io::Result<Vec<u8>> {
        if !stays_inside(name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unsafe resource name",
            ));
        }
        match self.0.as_ref() {
            Storage::Directory(root) => horizon_core::prepared_files::read(&root.join(name)),
            Storage::Archive(archive) => archive.read(name),
            Storage::Prefix { source, prefix } => source.read(&format!("{prefix}{name}")),
            Storage::Index(files) => files
                .get(name)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, name.to_owned()))?
                .read(),
        }
    }
    pub fn read_to_string(&self, name: &str) -> io::Result<String> {
        String::from_utf8(self.read(name)?)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }
    /// File names from the layer root, in name order. A directory prefix ends
    /// with `/`; partial file-name prefixes are supported for archives too.
    pub fn list(&self, prefix: &str) -> Vec<String> {
        if !prefix_inside(prefix) {
            return Vec::new();
        }
        match self.0.as_ref() {
            Storage::Directory(root) => {
                let mut out = Vec::new();
                let directory = if prefix.ends_with('/') || prefix.is_empty() {
                    prefix
                } else {
                    prefix.rsplit_once('/').map_or("", |(parent, _)| parent)
                };
                walk(root, &root.join(directory), &mut out, &mut BTreeSet::new());
                out.retain(|name| name.starts_with(prefix));
                out.sort();
                out
            }
            Storage::Archive(archive) => archive
                .names()
                .filter(|name| name.starts_with(prefix) && stays_inside(name))
                .map(str::to_owned)
                .collect(),
            Storage::Prefix {
                source,
                prefix: base,
            } => source
                .list(&format!("{base}{prefix}"))
                .into_iter()
                .filter_map(|name| name.strip_prefix(base).map(str::to_owned))
                .collect(),
            Storage::Index(files) => files
                .range(prefix.to_owned()..)
                .map_while(|(name, _)| name.starts_with(prefix).then(|| name.clone()))
                .collect(),
        }
    }
    /// Namespace/kind listing. Folder layers visit only the requested kinds;
    /// archive layers filter their already indexed names without inflating files.
    pub fn list_under(&self, top: &str, kinds: &[&str]) -> Vec<String> {
        debug_assert!(kinds.is_sorted(), "kinds in walk order");
        if !stays_inside(top) || kinds.iter().any(|kind| !stays_inside(kind)) {
            return Vec::new();
        }
        match self.0.as_ref() {
            Storage::Directory(root) => {
                let Ok(entries) = fs::read_dir(root.join(top)) else {
                    return Vec::new();
                };
                let mut namespaces: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
                namespaces.sort();
                let mut out = Vec::new();
                for namespace in namespaces {
                    for kind in kinds {
                        walk(root, &namespace.join(kind), &mut out, &mut BTreeSet::new());
                    }
                }
                out.sort();
                out
            }
            _ => self
                .list(&format!("{top}/"))
                .into_iter()
                .filter(|name| {
                    name.strip_prefix(top)
                        .and_then(|rest| rest.strip_prefix('/'))
                        .and_then(|rest| rest.split_once('/'))
                        .is_some_and(|(_, rest)| {
                            kinds.iter().any(|kind| {
                                rest.strip_prefix(kind)
                                    .is_some_and(|tail| tail.starts_with('/'))
                            })
                        })
                })
                .collect(),
        }
    }

    /// The decoded-sprite cache covers textures and their companions, as it
    /// did for folder roots. Memoization is local to one fingerprint operation:
    /// a later reload observes folder edits even when a source is shared.
    pub(crate) fn texture_fingerprint(&self) -> Option<u64> {
        if let Storage::Directory(root) = self.0.as_ref()
            && !root.join("assets").is_dir()
        {
            return None;
        }
        let mut archives = HashMap::new();
        let mut hash = DefaultHasher::new();
        for name in self.list_under("assets", &["textures"]) {
            name.hash(&mut hash);
            self.file_fingerprint(&name, &mut archives)?.hash(&mut hash);
        }
        Some(hash.finish())
    }
    fn file_fingerprint(&self, name: &str, archives: &mut HashMap<usize, u64>) -> Option<u64> {
        match self.0.as_ref() {
            Storage::Directory(root) => {
                crate::zip::metadata_fingerprint(&fs::metadata(root.join(name)).ok()?).ok()
            }
            Storage::Archive(archive) => {
                let key = Arc::as_ptr(&self.0) as usize;
                let fingerprint = match archives.get(&key) {
                    Some(fingerprint) => *fingerprint,
                    None => {
                        let fingerprint = archive.fingerprint().ok()?;
                        archives.insert(key, fingerprint);
                        fingerprint
                    }
                };
                let mut hash = DefaultHasher::new();
                fingerprint.hash(&mut hash);
                name.hash(&mut hash);
                Some(hash.finish())
            }
            Storage::Prefix { source, prefix } => {
                source.file_fingerprint(&format!("{prefix}{name}"), archives)
            }
            Storage::Index(files) => match &files.get(name)?.0 {
                ResourceStorage::File { source, name } => source.file_fingerprint(name, archives),
                ResourceStorage::Bytes(bytes) => {
                    let mut hash = DefaultHasher::new();
                    bytes.hash(&mut hash);
                    Some(hash.finish())
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Write,
        sync::atomic::{AtomicU64, Ordering},
    };

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "horizon-resource-source-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn archive(&self, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
            let path = self.0.join(name);
            let mut writer = zip::ZipWriter::new(fs::File::create(&path).unwrap());
            for (name, bytes) in files {
                writer
                    .start_file(*name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                writer.write_all(bytes).unwrap();
            }
            writer.finish().unwrap();
            path
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[cfg(unix)]
    #[test]
    fn directory_aliases_are_listed_but_cycles_are_not_followed() {
        let fixture = Fixture::new();
        fs::create_dir_all(fixture.0.join("assets/demo/textures")).unwrap();
        fs::write(fixture.0.join("assets/demo/textures/one.png"), b"one").unwrap();
        std::os::unix::fs::symlink(
            fixture.0.join("assets"),
            fixture.0.join("assets/demo/textures/loop"),
        )
        .unwrap();
        std::os::unix::fs::symlink(
            fixture.0.join("assets/demo"),
            fixture.0.join("assets/alias"),
        )
        .unwrap();
        let source = ResourceSource::directory(fixture.0.clone());
        assert_eq!(
            source.list("assets/"),
            [
                "assets/alias/textures/one.png",
                "assets/demo/textures/one.png"
            ]
        );
        let first = source.texture_fingerprint().unwrap();
        fs::write(fixture.0.join("assets/demo/textures/one.png"), b"changed").unwrap();
        assert_ne!(source.texture_fingerprint(), Some(first));
    }

    #[test]
    fn folder_and_archive_have_identical_lookup_and_listing() {
        let fixture = Fixture::new();
        let files: &[(&str, &[u8])] = &[
            ("assets/demo/a/file.json", b"nested"),
            ("assets/demo/a.json", b"flat"),
            ("data/demo/tags/item/test.json", b"tag"),
            ("data/demo/recipe/test.json", b"recipe"),
        ];
        let directory = fixture.0.join("folder");
        for (name, bytes) in files {
            let path = directory.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        let folder = ResourceSource::directory(directory);
        let archive = ResourceSource::archive(&fixture.archive("pack.jar", files)).unwrap();
        for (name, bytes) in files {
            assert_eq!(folder.read(name).unwrap(), *bytes);
            assert_eq!(archive.read(name).unwrap(), *bytes);
        }
        for prefix in ["", "assets/", "assets/demo/a", "missing/"] {
            assert_eq!(folder.list(prefix), archive.list(prefix));
        }
        assert_eq!(
            folder.list_under("data", &["tags"]),
            archive.list_under("data", &["tags"])
        );
        assert_eq!(
            archive.list_under("data", &["tags"]),
            ["data/demo/tags/item/test.json"]
        );
        assert_eq!(
            archive.read("missing").unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }

    #[test]
    fn archive_safety_is_enforced_without_extracting_files() {
        let fixture = Fixture::new();
        let path = fixture.archive(
            "pack.zip",
            &[
                ("assets/demo/good.txt", b"good"),
                ("assets\\demo\\normalized.txt", b"normalized"),
                ("../escape", b"bad"),
                ("assets/demo/../escape", b"bad"),
                ("assets//bad", b"bad"),
                ("C:/escape", b"bad"),
            ],
        );
        let source = ResourceSource::archive(&path).unwrap();
        assert_eq!(
            source.list(""),
            ["assets/demo/good.txt", "assets/demo/normalized.txt"]
        );
        assert_eq!(source.read("assets/demo/good.txt").unwrap(), b"good");
        assert_eq!(
            source.read("assets/demo/normalized.txt").unwrap(),
            b"normalized"
        );
        for name in [
            "../escape",
            "assets/demo/../escape",
            "/escape",
            "C:/escape",
            "assets//bad",
        ] {
            assert_eq!(
                source.read(name).unwrap_err().kind(),
                io::ErrorKind::InvalidInput
            );
        }
        assert!(!fixture.0.join("assets").exists());
        assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
    }

    #[test]
    fn reload_opens_new_bytes_and_existing_readers_keep_their_snapshot() {
        let fixture = Fixture::new();
        let path = fixture.archive("pack.jar", &[("assets/demo/value.txt", b"before")]);
        let before = ResourceSource::archive(&path).unwrap();
        let shared = before.clone();
        let replacement = fixture.archive(
            "new.jar",
            &[
                ("assets/demo/value.txt", b"after"),
                ("data/demo/new.json", b"new"),
            ],
        );
        fs::rename(&path, fixture.0.join("old.jar")).unwrap();
        fs::rename(replacement, &path).unwrap();
        let after = ResourceSource::archive(&path).unwrap();
        assert_eq!(before.read("assets/demo/value.txt").unwrap(), b"before");
        assert_eq!(shared.read("assets/demo/value.txt").unwrap(), b"before");
        assert_eq!(after.read("assets/demo/value.txt").unwrap(), b"after");
        assert!(before.list("data/").is_empty());
        assert_eq!(after.list("data/"), ["data/demo/new.json"]);
        assert!(!fixture.0.join("assets").exists());
    }
}
