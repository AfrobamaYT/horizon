//! Source-only native packages. Reading a package never executes its code.
use super::Facts;
use std::{collections::BTreeMap, io::Read, path::Path};

pub const MIXINS_REV: &str = "ddd9700b21997c95d95ac6ee5ecafe3830cf622a";

/// Whether `path` names a mod package.
pub fn is_file(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "hmod")
}

/// Whether an archive contains a root Cargo project, irrespective of install folder.
pub fn holds(path: &Path) -> bool {
    std::fs::File::open(path)
        .ok()
        .and_then(|f| zip::ZipArchive::new(f).ok())
        .is_some_and(|zip| zip.index_for_name("Cargo.toml").is_some())
}

/// A flat native mod and its checked, archive-relative files.
pub struct Bundle {
    pub manifest: crate::Manifest,
    pub files: BTreeMap<String, Vec<u8>>,
}

impl Bundle {
    pub fn read(path: &Path) -> Result<Self, String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        let package = crate::Package::read_zip(&mut zip)?;
        if package.manifest.native.is_none() {
            return Err("not a native package".into());
        }
        let mut files = BTreeMap::new();
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
            if entry.is_dir() {
                continue;
            }
            let name = entry.name().map_err(|e| e.to_string())?.into_owned();
            let size = entry.size();
            let mut bytes = Vec::new();
            (&mut entry)
                .take(size.saturating_add(1))
                .read_to_end(&mut bytes)
                .map_err(|e| format!("{name}: {e}"))?;
            if bytes.len() as u64 != size {
                return Err(format!("{name}: invalid expanded size"));
            }
            files.insert(name, bytes);
        }
        Ok(Self {
            manifest: package.manifest,
            files,
        })
    }

    /// Registry facts from the one package manifest, never Cargo metadata.
    pub fn facts(&self) -> Facts {
        let native = self.manifest.native.as_ref().expect("native bundle");
        Facts {
            id: Some(self.manifest.id.clone()),
            version: Some(self.manifest.version.clone()),
            api: format!("native-1/{}/{}", native.horizon, native.mixins),
            game: Some(native.game.clone()),
            ..Facts::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn manifest() -> String {
        format!(
            "id = 'veinminer'\nname = 'Veinminer'\nversion = '0.3.0'\napi = '0.2'\ncapabilities = ['registries']\n[native]\ngame = '26.3'\nhorizon = '{}'\nmixins = '{}'\n",
            "a".repeat(40),
            "b".repeat(40)
        )
    }

    fn read(name: &str, manifest: &str, extras: &[(&str, &[u8])]) -> Result<Bundle, String> {
        let path =
            std::env::temp_dir().join(format!("native-bundle-{name}-{}.hmod", std::process::id()));
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        for (name, bytes) in [
            (crate::manifest::MANIFEST, manifest.as_bytes()),
            (
                "Cargo.toml",
                b"[package]\nname='other'\nversion='9'\n".as_slice(),
            ),
            ("src/lib.rs", b"fn hook() {}".as_slice()),
        ]
        .into_iter()
        .chain(extras.iter().copied())
        {
            zip.start_file(name, options).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
        let result = Bundle::read(&path);
        std::fs::remove_file(path).unwrap();
        result
    }

    #[test]
    fn facts_use_manifest_identity_and_exact_build_pins() {
        let bundle = read("facts", &manifest(), &[]).unwrap();
        let facts = bundle.facts();
        assert_eq!(facts.id.as_deref(), Some("veinminer"));
        assert_eq!(facts.version.as_deref(), Some("0.3.0"));
        assert_eq!(
            facts.api,
            format!("native-1/{}/{}", "a".repeat(40), "b".repeat(40))
        );
        assert_eq!(facts.game.as_deref(), Some("26.3"));
    }

    #[test]
    fn package_carries_resources_and_code_together() {
        let bytes = [0, 255, 42];
        let bundle = read(
            "resources",
            &manifest(),
            &[
                ("data/veinminer/table.bin", &bytes),
                ("assets/veinminer/lang/en_us.json", b"{}"),
            ],
        )
        .unwrap();
        assert_eq!(bundle.files["data/veinminer/table.bin"], bytes);
        assert_eq!(bundle.files["src/lib.rs"], b"fn hook() {}");
        assert!(bundle.files.contains_key(crate::manifest::MANIFEST));
    }

    #[test]
    fn refuses_old_paths_unsafe_entries_and_unknown_fields() {
        for (i, name) in [
            "horizon-native.json",
            "horizon-mixin.toml",
            "native/rust/src/lib.rs",
            "rust/src/lib.rs",
            "../escape.rs",
            "src/../escape.rs",
            "src/blob.bin",
        ]
        .iter()
        .enumerate()
        {
            assert!(
                read(&format!("invalid-{i}"), &manifest(), &[(name, b"bad")]).is_err(),
                "{name}"
            );
        }
        assert!(read("unknown", &(manifest() + "unexpected = true\n"), &[]).is_err());
        assert!(read("pin", &manifest().replace(&"a".repeat(40), "main"), &[]).is_err());
        assert!(
            read(
                "magic",
                &manifest(),
                &[("assets/veinminer/file.bin", b"MZ bad")]
            )
            .is_err()
        );
    }

    #[test]
    fn classification_uses_root_cargo_not_install_folder() {
        let root = std::env::temp_dir().join(format!("native-kind-{}", std::process::id()));
        for (folder, native) in [("content", true), ("native", false)] {
            let dir = root.join(folder);
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join("test.hmod");
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
            zip.start_file(
                if native {
                    "Cargo.toml"
                } else {
                    crate::manifest::MANIFEST
                },
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            zip.write_all(b"contents").unwrap();
            zip.finish().unwrap();
            assert_eq!(holds(&path), native);
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
