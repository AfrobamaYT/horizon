//! `horizon-mod.toml`, a content mod's manifest, and the `.hmod` file around it.
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    io::Read,
    path::{Path, PathBuf},
};

/// The manifest's file name in a `.hmod` or a mod folder.
pub const MANIFEST: &str = "horizon-mod.toml";
/// Executable components are not content and are refused in a `.hmod`.
pub const WASM: &str = "mod.wasm";
/// The content manifest API this build loads.
pub const API: &str = "0.2";
/// Namespaces no mod may claim: vanilla's, the convention tags' (mods add
/// to those, docs/mods.md) and the game's own.
const RESERVED_NAMESPACES: [&str; 3] = ["minecraft", "c", "horizon"];
/// A mod's saved settings, shared by native mods and modpack overrides.
pub const STORAGE_DIR: &str = ".horizon-mod-data";
/// The largest manifest this build reads.
const MAX_MANIFEST: u64 = 32 << 20;

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// `[a-z0-9_]`, 1 to 64 characters: the mod's folder, log and settings key.
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub authors: Vec<String>,
    /// The content API line: `0.2`.
    pub api: String,
    #[serde(default)]
    pub side: Side,
    pub capabilities: Vec<Capability>,
    /// Namespaces besides `id` that the mod's `assets/` and `data/` use, for
    /// a content mod whose files sit under other names than its id (Towns
    /// and Towers ships `kaisyn` and `towns_and_towers`). No two content mods
    /// that are on may share one (`content::clash`).
    #[serde(default)]
    pub namespaces: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    /// Local content, not compared with the server's mods when joining.
    #[default]
    Client,
    /// Both, and the server tells clients which version it expects.
    Both,
}

/// What a content mod adds; the player sees it before turning the mod on.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    /// New blocks, items and mob effects from the mod's `data/` (API 0.2).
    Registries,
}
impl Capability {
    /// What the player reads before turning the mod on: "May <this>".
    pub fn describe(self) -> &'static str {
        match self {
            Capability::Registries => {
                "add blocks, items, mob effects and entity types to your worlds"
            }
        }
    }
}
impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Capability::Registries => f.write_str("registries"),
        }
    }
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Self, String> {
        let m: Manifest = toml::from_str(text).map_err(|e| format!("{MANIFEST}: {e}"))?;
        let id_ok = (1..=64).contains(&m.id.len())
            && m.id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        if !id_ok {
            return Err(format!(
                "{MANIFEST}: id {:?} is not [a-z0-9_]{{1,64}}",
                m.id
            ));
        }
        if m.api != API {
            return Err(format!(
                "unsupported content API {}; this Horizon reads {API}",
                m.api
            ));
        }
        if m.capabilities != [Capability::Registries] {
            return Err(format!(
                "{MANIFEST}: a content mod has capabilities = [\"registries\"]"
            ));
        }
        for (i, ns) in m.namespaces.iter().enumerate() {
            // Vanilla's namespace characters, at most as long as an id.
            let chars_ok = (1..=64).contains(&ns.len())
                && ns
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b));
            if !chars_ok {
                return Err(format!(
                    "{MANIFEST}: namespace {ns:?} is not [a-z0-9_.-]{{1,64}}"
                ));
            }
            if RESERVED_NAMESPACES.contains(&ns.as_str()) {
                return Err(format!(
                    "{MANIFEST}: namespace {ns} is not a mod's to claim (only to add to its tags)"
                ));
            }
            if *ns == m.id || m.namespaces[..i].contains(ns) {
                return Err(format!("{MANIFEST}: namespace {ns} is listed twice"));
            }
        }
        Ok(m)
    }
    /// The namespaces the mod's content owns: its id, then `namespaces`.
    pub fn owned_namespaces(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.id.as_str()).chain(self.namespaces.iter().map(String::as_str))
    }
}

/// A content mod as found on disk.
pub struct Package {
    pub manifest: Manifest,
}
impl Package {
    /// A `.hmod` (a zip holding `horizon-mod.toml`, `assets/` and `data/`)
    /// or a folder holding the same. Executable components are unsupported.
    pub fn read(path: &Path) -> Result<Self, String> {
        let toml = if path.is_dir() {
            if path.join(WASM).exists() {
                return Err(format!(
                    "{WASM}: unsupported executable file in a content mod"
                ));
            }
            std::fs::read_to_string(path.join(MANIFEST)).map_err(|e| format!("{MANIFEST}: {e}"))?
        } else {
            let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
            let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
            if zip.index_for_name(WASM).is_some() {
                return Err(format!(
                    "{WASM}: unsupported executable file in a content mod"
                ));
            }
            let mut toml = String::new();
            zip.by_name(MANIFEST)
                .map_err(|e| format!("{MANIFEST}: {e}"))?
                .take(MAX_MANIFEST + 1)
                .read_to_string(&mut toml)
                .map_err(|e| format!("{MANIFEST}: {e}"))?;
            if toml.len() as u64 > MAX_MANIFEST {
                return Err(format!("{MANIFEST}: over {} MiB", MAX_MANIFEST >> 20));
            }
            toml
        };
        Ok(Package {
            manifest: Manifest::parse(&toml)?,
        })
    }
    /// Whether the mod at `path` (a `.hmod` or a folder) has a [`MANIFEST`].
    /// A `.hmod` holding only native code has none.
    pub fn has_manifest(path: &Path) -> bool {
        if path.is_dir() {
            return path.join(MANIFEST).is_file();
        }
        std::fs::File::open(path)
            .ok()
            .and_then(|f| zip::ZipArchive::new(f).ok())
            .is_some_and(|mut zip| zip.by_name(MANIFEST).is_ok())
    }
    /// The `.hmod` bytes for `manifest_text` and content `files` (`assets/...`, `data/...`).
    pub fn write_files(manifest_text: &str, files: &[(&str, &[u8])]) -> Result<Vec<u8>, String> {
        use std::io::Write;
        Manifest::parse(manifest_text)?;
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        let err = |e: zip::result::ZipError| e.to_string();
        let mut put = |name: &str, bytes: &[u8]| -> Result<(), String> {
            zip.start_file(name, options).map_err(err)?;
            zip.write_all(bytes).map_err(|e| e.to_string())
        };
        put(MANIFEST, manifest_text.as_bytes())?;
        for (name, bytes) in files {
            crate::content::check_path(name)?;
            put(name, bytes)?;
        }
        Ok(zip.finish().map_err(err)?.into_inner())
    }
}

/// The `.hmod` files and mod folders in `dir`, by name.
pub fn find(dir: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = read
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "hmod") || p.join(MANIFEST).is_file())
        .collect();
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"
id = "ruby"
name = "Ruby"
version = "0.1.0"
api = "0.2"
capabilities = ["registries"]
"#;

    #[test]
    fn refuses_bad_ids_and_retired_manifests() {
        let bad_id = EXAMPLE.replace("ruby", "Ruby!");
        assert!(Manifest::parse(&bad_id).unwrap_err().contains("is not"));
        for api in ["0.1", "0.3", "0.4"] {
            let text = EXAMPLE.replace("api = \"0.2\"", &format!("api = \"{api}\""));
            assert!(
                Manifest::parse(&text)
                    .unwrap_err()
                    .contains("unsupported content API")
            );
        }
        for caps in ["[]", "[\"hud\"]", "[\"registries\", \"registries\"]"] {
            let text = EXAMPLE.replace("[\"registries\"]", caps);
            assert!(Manifest::parse(&text).is_err());
        }
        for field in ["network = true", "experimental = []", "side = \"server\""] {
            assert!(Manifest::parse(&format!("{EXAMPLE}{field}\n")).is_err());
        }
        assert_eq!(Manifest::parse(EXAMPLE).unwrap().side, Side::Client);
        assert_eq!(
            Manifest::parse(&format!("{EXAMPLE}side = \"both\"\n"))
                .unwrap()
                .side,
            Side::Both
        );
    }

    #[test]
    fn checks_extra_namespaces() {
        let with = |list: &str| Manifest::parse(&format!("{EXAMPLE}namespaces = [{list}]\n"));
        let m = with("\"kaisyn\", \"towns_and_towers\"").unwrap();
        let owned: Vec<&str> = m.owned_namespaces().collect();
        assert_eq!(owned, ["ruby", "kaisyn", "towns_and_towers"]);
        for reserved in ["minecraft", "c", "horizon"] {
            let e = with(&format!("\"{reserved}\"")).unwrap_err();
            assert!(e.contains("not a mod's to claim"), "{e}");
        }
        assert!(with("\"ruby\"").unwrap_err().contains("twice"));
        assert!(with("\"a\", \"a\"").unwrap_err().contains("twice"));
        assert!(with("\"Kaisyn\"").unwrap_err().contains("is not"));
        assert!(with("\"\"").unwrap_err().contains("is not"));
    }

    #[test]
    fn content_round_trips_and_executable_files_are_refused() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("horizon-mods-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(MANIFEST), EXAMPLE).unwrap();
        let path = dir.join("ruby.hmod");
        std::fs::write(&path, Package::write_files(EXAMPLE, &[]).unwrap()).unwrap();
        assert_eq!(Package::read(&path).unwrap().manifest.id, "ruby");
        assert_eq!(Package::read(&dir).unwrap().manifest.id, "ruby");
        for wasm in [b"".as_slice(), b"\0asm"] {
            std::fs::write(dir.join(WASM), wasm).unwrap();
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file(MANIFEST, options).unwrap();
            zip.write_all(EXAMPLE.as_bytes()).unwrap();
            zip.start_file(WASM, options).unwrap();
            zip.write_all(wasm).unwrap();
            zip.finish().unwrap();
            for source in [&dir, &path] {
                let e = Package::read(source).err().unwrap();
                assert_eq!(e, "mod.wasm: unsupported executable file in a content mod");
            }
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
