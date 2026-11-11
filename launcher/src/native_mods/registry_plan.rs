//! One dependency plan shared by source staging and immutable prebuilt SDK builds.
use super::{api::Project, crates::Wanted, registry_lock::RegistryLock};
use std::path::Path;

pub(super) struct Plan {
    pub game: RegistryLock,
    pub mods: RegistryLock,
    pub allowed: RegistryLock,
    fetch_roots: RegistryLock,
    owned: Vec<(String, String, RegistryLock)>,
    roots: std::collections::BTreeSet<String>,
}

impl Plan {
    pub fn read(game: &str, projects: &[Project<'_>], wanted: &[Wanted]) -> Result<Self, String> {
        let game = RegistryLock::parse(game)?;
        let mut mods = RegistryLock::default();
        let mut owned = Vec::new();
        let roots = projects
            .iter()
            .map(|project| project.package.clone())
            .collect();
        for project in projects {
            let bundle = project.bundle;
            let requested: Vec<_> = wanted
                .iter()
                .filter(|w| w.mod_id == bundle.manifest.id)
                .collect();
            if requested.is_empty() {
                continue;
            }
            let pins = RegistryLock::parse(&super::read_lock(&bundle.dir)?)?;
            for request in requested {
                if !pins
                    .packages
                    .keys()
                    .any(|id| id.name == request.name && request.requirement.matches(&id.version))
                {
                    return Err(format!(
                        "{}: Cargo.lock does not pin {} {}",
                        request.mod_id, request.name, request.version
                    ));
                }
            }
            mods.extend(&pins)?;
            owned.push((bundle.manifest.id.clone(), project.package.clone(), pins));
        }
        let mut fetch_roots = RegistryLock::default();
        for (identity, record) in &mods.packages {
            if wanted.iter().any(|request| {
                request.name == identity.name && request.requirement.matches(&identity.version)
            }) {
                fetch_roots
                    .packages
                    .insert(identity.clone(), record.clone());
            }
        }
        let mut allowed = game.clone();
        allowed.extend(&mods)?;
        Ok(Self {
            game,
            mods,
            allowed,
            fetch_roots,
            owned,
            roots,
        })
    }

    pub fn check_mods(&self, resolved: &RegistryLock) -> Result<(), String> {
        for (id, root, pins) in &self.owned {
            let mut skip_local: std::collections::BTreeSet<String> = self
                .roots
                .iter()
                .filter(|other| *other != root)
                .cloned()
                .collect();
            skip_local.extend(
                super::api::PACKAGES
                    .iter()
                    .map(|(package, _)| (*package).to_owned()),
            );
            skip_local.insert(super::internals::package(id));
            let mut owned_pins = self.game.clone();
            owned_pins.extend(pins)?;
            resolved.check_mod(id, root, &skip_local, &owned_pins)?;
        }
        Ok(())
    }

    pub fn explain_resolution(&self, error: String) -> String {
        let mut details = Vec::new();
        for id in self
            .mods
            .packages
            .keys()
            .filter(|id| error.contains(&id.name))
        {
            let versions = self
                .game
                .packages
                .keys()
                .filter(|game| game.name == id.name && game.version != id.version)
                .map(|game| game.version.to_string())
                .collect::<Vec<_>>();
            if !versions.is_empty() {
                details.push(format!(
                    "{}: game locks [{}], mod locks {}; the game graph must not move",
                    id.name,
                    versions.join(", "),
                    id.version
                ));
            }
        }
        if details.is_empty() {
            error
        } else {
            format!("{error}\n\n{}", details.join("\n"))
        }
    }

    pub fn fetch(
        &self,
        rust: &crate::rust_toolchain::Toolchain,
        directory: &Path,
    ) -> Result<(), String> {
        self.fetch_with_patches(rust, directory, &toml::Table::new())
    }

    pub fn fetch_with_patches(
        &self,
        rust: &crate::rust_toolchain::Toolchain,
        directory: &Path,
        patches: &toml::Table,
    ) -> Result<(), String> {
        self.mods.fetch_workspace(&self.fetch_roots, directory)?;
        apply_patches(directory, patches)?;
        let mut index = super::cargo(rust, directory, "update");
        index.arg("--workspace");
        super::run(index, "Checking the pinned registry index", |_| {})
            .map_err(|error| self.explain_resolution(error))?;
        let pins = RegistryLock::parse(&super::read_lock(directory)?)?;
        self.allowed
            .check_resolution(&pins, &RegistryLock::default())?;
        let mut fetch = super::cargo(rust, directory, "fetch");
        fetch.arg("--locked");
        super::run(fetch, "Downloading the mods' pinned crates", |_| {})
    }
}

/// Only the SDK identity supplies patches; mod manifests cannot inject overrides.
pub(super) fn apply_patches(directory: &Path, patches: &toml::Table) -> Result<(), String> {
    let path = directory.join("Cargo.toml");
    let mut manifest: toml::Table = std::fs::read_to_string(&path)
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e| format!("Cargo.toml: {e}"))?;
    manifest.insert("patch".into(), patches.clone().into());
    std::fs::write(path, manifest.to_string()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn production_plan_preserves_isolated_game_patch_identity() {
        let original = "version=4\n[[package]]\nname='allocator-sys'\nversion='0.1.49'\n";
        let plan = Plan::read(original, &[], &[]).unwrap();
        plan.allowed
            .check_resolution(
                &RegistryLock::parse(original).unwrap(),
                &RegistryLock::default(),
            )
            .unwrap();
        let moved = RegistryLock::parse(&original.replace("0.1.49", "0.1.50")).unwrap();
        // Both fetch and SDK compilation use allowed against resolved, with an empty game argument.
        let error = plan
            .allowed
            .check_resolution(&moved, &RegistryLock::default())
            .unwrap_err();
        assert!(error.contains("allocator-sys"), "{error}");
    }

    #[test]
    fn cargo_resolves_registry_parent_through_sdk_local_patch_without_compiling() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for path in [
            "vendor/allocator/src",
            "sdk/allocator-sys/src",
            "src",
            ".cargo",
        ] {
            std::fs::create_dir_all(root.join(path)).unwrap();
        }
        for (path, text) in [
            ("Cargo.toml", "[package]\nname='fixture'\nversion='1.0.0'\n[workspace]\n[dependencies]\nallocator='1'\n"),
            ("src/lib.rs", ""),
            ("vendor/allocator/Cargo.toml", "[package]\nname='allocator'\nversion='1.0.0'\n[dependencies]\nallocator-sys='0.1'\n"),
            ("vendor/allocator/src/lib.rs", ""),
            ("sdk/allocator-sys/Cargo.toml", "[package]\nname='allocator-sys'\nversion='0.1.49'\n"),
            ("sdk/allocator-sys/src/lib.rs", ""),
            (".cargo/config.toml", "[source.crates-io]\nreplace-with='fixture'\n[source.fixture]\ndirectory='vendor'\n"),
        ] { std::fs::write(root.join(path), text).unwrap(); }
        std::fs::write(
            root.join("vendor/allocator/.cargo-checksum.json"),
            format!(r#"{{"package":"{}","files":{{}}}}"#, "a".repeat(64)),
        )
        .unwrap();
        let patches: toml::Table = format!(
            "[crates-io]\nallocator-sys={{path={:?}}}\n",
            root.join("sdk/allocator-sys").to_string_lossy()
        )
        .parse()
        .unwrap();
        apply_patches(root, &patches).unwrap();
        let output = std::process::Command::new("cargo")
            .args(["generate-lockfile", "--offline"])
            .current_dir(root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = std::fs::read_to_string(root.join("Cargo.lock")).unwrap();
        let game = format!("version=4\n[[package]]\nname='allocator'\nversion='1.0.0'\nsource='registry+https://github.com/rust-lang/crates.io-index'\nchecksum='{}'\ndependencies=['allocator-sys']\n[[package]]\nname='allocator-sys'\nversion='0.1.49'\n", "a".repeat(64));
        RegistryLock::parse(&game)
            .unwrap()
            .check_resolution(
                &RegistryLock::parse(&text).unwrap(),
                &RegistryLock::default(),
            )
            .unwrap();
        assert!(!root.join("target").exists());
    }

    #[test]
    fn fetch_roots_exclude_unrelated_game_union_records() {
        let dir = tempfile::tempdir().unwrap();
        super::super::tests::write_bundle(dir.path(), "example", "");
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname='example'\nversion='1.0.0'\n[dependencies]\nhorizon-mod-api='0.1'\nused='1'\n").unwrap();
        let mut lock = "version=4\n".to_owned();
        for name in ["used", "unrelated-allocator"] {
            lock.push_str(&format!("[[package]]\nname='{name}'\nversion='1.0.0'\nsource='registry+https://github.com/rust-lang/crates.io-index'\nchecksum='{}'\n", "a".repeat(64)));
        }
        std::fs::write(dir.path().join("Cargo.lock"), &lock).unwrap();
        let bundle = super::super::bundle_in(dir.path()).unwrap().unwrap();
        let projects = [Project::read(&bundle).unwrap()];
        let wanted = [Wanted {
            mod_id: "example".into(),
            name: "used".into(),
            version: "1".into(),
            requirement: semver::VersionReq::parse("1").unwrap(),
        }];
        let plan = Plan::read(&lock, &projects, &wanted).unwrap();
        let fetch = dir.path().join("fetch");
        plan.mods
            .fetch_workspace(&plan.fetch_roots, &fetch)
            .unwrap();
        let manifest: toml::Table = std::fs::read_to_string(fetch.join("Cargo.toml"))
            .unwrap()
            .parse()
            .unwrap();
        let deps = manifest["dependencies"].as_table().unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(
            deps.values().next().unwrap()["package"].as_str(),
            Some("used")
        );
    }

    #[test]
    fn missing_malformed_and_bad_checksum_locks_refuse_before_fetch() {
        let dir = tempfile::tempdir().unwrap();
        super::super::tests::write_bundle(dir.path(), "example", "");
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname='example'\nversion='1.0.0'\n[dependencies]\nhorizon-mod-api='0.1'\nitoa='1'\n").unwrap();
        let bundle = super::super::bundle_in(dir.path()).unwrap().unwrap();
        let projects = [Project::read(&bundle).unwrap()];
        let wanted = [Wanted {
            mod_id: "example".into(),
            name: "itoa".into(),
            version: "1".into(),
            requirement: semver::VersionReq::parse("1").unwrap(),
        }];
        let game = "version=4\npackage=[]\n";
        let missing = Plan::read(game, &projects, &wanted).err().unwrap();
        assert!(missing.contains("Cargo.lock"), "{missing}");
        for (text, expected) in [("version=4", "missing packages"), ("version=4\n[[package]]\nname='itoa'\nversion='1.0.18'\nsource='registry+https://github.com/rust-lang/crates.io-index'\nchecksum='bad'\n", "checksum")] {
            std::fs::write(dir.path().join("Cargo.lock"), text).unwrap();
            let error = Plan::read(game, &projects, &wanted).err().unwrap();
            assert!(error.contains(expected), "{error}");
        }
        assert!(!dir.path().join("target").exists());
    }
}
