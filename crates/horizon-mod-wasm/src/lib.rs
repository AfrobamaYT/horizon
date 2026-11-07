//! The runtime for wasm mods. Packages declaring `[wasm]` are grouped by their `[dependencies]`
//! (the same model as native mods: required and optional, semver on `horizon-mod.toml`
//! versions, one library copy shared by its dependents). Every refusal names its reason for the
//! Mods page.
pub mod link;

use horizon_mod_package::dependencies;
use horizon_mod_package::manifest::Manifest;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// One wasm mod as installed: its manifest and its checked `mod.wasm`.
pub struct WasmPackage {
    pub manifest: Manifest,
    pub module: Vec<u8>,
}

/// Why a wasm mod doesn't load, for the Mods page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// A required dependency isn't installed, or none installed matches its version.
    MissingDependency { dependency: String, version: String },
    /// The shared native/wasm resolver refused the dependency graph.
    DependencyOrder { reason: String },
    /// A dependency's installed `mod.wasm` isn't the one this mod was linked against.
    LinkedAgainst { dependency: String },
    /// Members of one group were built with different toolchains.
    Toolchain { found: String, group: String },
    /// A dependency of this mod was refused, so its group can't link.
    DependencyRefused { dependency: String },
    /// The package's `mod.wasm` failed its checks.
    Package(String),
    /// This game build has no wasm mod boundary: the one that was here is gone and its
    /// replacement is not built yet.
    NoBoundary,
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingDependency {
                dependency,
                version,
            } => {
                write!(f, "needs {dependency} {version}, which isn't installed")
            }
            Self::DependencyOrder { reason } => write!(f, "{reason}"),
            Self::LinkedAgainst { dependency } => {
                write!(f, "built against another build of {dependency}")
            }
            Self::Toolchain { found, group } => {
                write!(f, "built with {found}, its dependencies with {group}")
            }
            Self::DependencyRefused { dependency } => {
                write!(f, "its dependency {dependency} didn't load")
            }
            Self::Package(why) => write!(f, "{why}"),
            Self::NoBoundary => write!(f, "this game build has no wasm mod boundary yet"),
        }
    }
}

impl std::error::Error for Refusal {}

/// Splits the packages into groups: each mod with everything it depends on, transitively;
/// two mods sharing a dependency share its group, so a library has one copy. A mod whose
/// required dependency is missing, or whose linked hashes or toolchain disagree, is refused
/// with every mod depending on it.
fn groups(packages: &[WasmPackage]) -> (Vec<Vec<usize>>, Vec<(String, Refusal)>) {
    let by_id: BTreeMap<&str, usize> = packages
        .iter()
        .enumerate()
        .map(|(i, p)| (p.manifest.id.as_str(), i))
        .collect();
    let mut refused: BTreeMap<usize, Refusal> = BTreeMap::new();
    let declared: Vec<_> = packages
        .iter()
        .map(|p| dependencies::from_manifest(&p.manifest.dependencies))
        .collect();
    // Refuse exactly the resolver's owners, then their dependents, preserving
    // unrelated libraries. There is no second semver or topological rule here.
    let order = loop {
        let active: Vec<_> = (0..packages.len())
            .filter(|i| !refused.contains_key(i))
            .collect();
        let inputs: Vec<_> = active
            .iter()
            .map(|&i| dependencies::Mod {
                id: &packages[i].manifest.id,
                version: &packages[i].manifest.version,
                has_api: true,
                dependencies: &declared[i],
            })
            .collect();
        match dependencies::resolve(&inputs) {
            Ok(resolution) => {
                break resolution
                    .order
                    .into_iter()
                    .map(|i| active[i])
                    .collect::<Vec<_>>();
            }
            Err(error) => {
                let reason = error.to_string();
                let owners: Vec<&str> = match &error {
                    dependencies::Error::Cycle { mods } => {
                        mods.iter().map(String::as_str).collect()
                    }
                    dependencies::Error::Duplicate(id) => vec![id],
                    dependencies::Error::Manifest { owner, .. }
                    | dependencies::Error::Missing { owner, .. }
                    | dependencies::Error::NoApi { owner, .. }
                    | dependencies::Error::CargoConflict { owner, .. }
                    | dependencies::Error::Version { owner, .. } => vec![owner],
                };
                for &i in &active {
                    if owners.contains(&packages[i].manifest.id.as_str()) {
                        let refusal = match &error {
                            dependencies::Error::Missing { target, .. }
                            | dependencies::Error::Version { target, .. } => {
                                Refusal::MissingDependency {
                                    dependency: target.clone(),
                                    version: declared[i]
                                        .iter()
                                        .find(|d| d.id == *target)
                                        .unwrap()
                                        .requirement
                                        .to_string(),
                                }
                            }
                            _ => Refusal::DependencyOrder {
                                reason: reason.clone(),
                            },
                        };
                        refused.insert(i, refusal);
                    }
                }
                propagate_refusals(packages, &by_id, &mut refused);
            }
        }
    };

    for (i, package) in packages.iter().enumerate() {
        let wasm = package
            .manifest
            .wasm
            .as_ref()
            .expect("a wasm package declares [wasm]");
        for id in package.manifest.dependencies.keys() {
            let Some(&d) = by_id.get(id.as_str()) else {
                continue;
            };
            let dep = packages[d]
                .manifest
                .wasm
                .as_ref()
                .expect("a wasm package declares [wasm]");
            if wasm
                .linked()
                .get(id)
                .is_some_and(|hash| hash != dep.sha512())
            {
                refused.entry(i).or_insert(Refusal::LinkedAgainst {
                    dependency: id.clone(),
                });
            }
            if dep.toolchain() != wasm.toolchain() {
                refused.entry(i).or_insert(Refusal::Toolchain {
                    found: wasm.toolchain().into(),
                    group: dep.toolchain().into(),
                });
            }
        }
    }
    propagate_refusals(packages, &by_id, &mut refused);
    // Union-find over the dependency edges of the mods that stay.
    let mut parent: Vec<usize> = (0..packages.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for (i, package) in packages
        .iter()
        .enumerate()
        .filter(|(i, _)| !refused.contains_key(i))
    {
        for id in package.manifest.dependencies.keys() {
            if let Some(&d) = by_id.get(id.as_str()).filter(|d| !refused.contains_key(d)) {
                let (a, b) = (root(&mut parent, i), root(&mut parent, d));
                parent[a] = b;
            }
        }
    }
    let mut by_root: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in (0..packages.len()).filter(|i| !refused.contains_key(i)) {
        by_root.entry(root(&mut parent, i)).or_default().push(i);
    }
    let groups = by_root
        .into_values()
        .map(|members| {
            order
                .iter()
                .copied()
                .filter(|i| members.contains(i))
                .collect()
        })
        .collect();
    let refused = refused
        .into_iter()
        .map(|(i, r)| (packages[i].manifest.id.clone(), r))
        .collect();
    (groups, refused)
}

/// A refused mod takes its dependents with it, until nothing changes.
fn propagate_refusals(
    packages: &[WasmPackage],
    by_id: &BTreeMap<&str, usize>,
    refused: &mut BTreeMap<usize, Refusal>,
) {
    loop {
        let before = refused.len();
        for (i, package) in packages.iter().enumerate() {
            if refused.contains_key(&i) {
                continue;
            }
            if let Some(id) = package.manifest.dependencies.keys().find(|id| {
                by_id
                    .get(id.as_str())
                    .is_some_and(|d| refused.contains_key(d))
            }) {
                refused.insert(
                    i,
                    Refusal::DependencyRefused {
                        dependency: id.clone(),
                    },
                );
            }
        }
        if refused.len() == before {
            break;
        }
    }
}

static REFUSED: OnceLock<Vec<(String, Refusal)>> = OnceLock::new();

/// The wasm mods refused at startup, each with its reason, for the Mods page.
pub fn refused() -> &'static [(String, Refusal)] {
    REFUSED.get().map_or(&[], Vec::as_slice)
}

/// Loads the wasm mods the player turned on in `game_dir` (and every `dev` package), once at
/// startup: the same selection as content mods (`horizon_mod_package::selection`), so a wasm mod's
/// `assets/` and `data/` load as content beside its code. Refusals are kept for [`refused`].
/// Until the shared-memory boundary exists, a mod whose dependencies resolve is refused with
/// [`Refusal::NoBoundary`].
pub fn load_installed(game_dir: &Path, dev: &[PathBuf]) {
    let enabled = horizon_mod_package::mod_list::ModList::load(game_dir).enabled;
    let mut refused = Vec::new();
    let packages: Vec<WasmPackage> =
        horizon_mod_package::selection::content_packages(game_dir, &enabled, dev)
            .filter(|selected| selected.package.manifest.wasm.is_some())
            .filter_map(
                |selected| match horizon_mod_package::Package::read_wasm(&selected.path) {
                    Ok(module) => Some(WasmPackage {
                        manifest: selected.package.manifest,
                        module,
                    }),
                    Err(why) => {
                        refused.push((selected.package.manifest.id, Refusal::Package(why)));
                        None
                    }
                },
            )
            .collect();
    let (groups, grouped) = groups(&packages);
    refused.extend(grouped);
    refused.extend(
        groups
            .into_iter()
            .flatten()
            .map(|i| (packages[i].manifest.id.clone(), Refusal::NoBoundary)),
    );
    for (id, why) in &refused {
        log::warn!("wasm mod {id} refused: {why}");
    }
    let _ = REFUSED.set(refused);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(
        id: &str,
        version: &str,
        deps: &str,
        sha: char,
        toolchain: &str,
        linked: &str,
    ) -> WasmPackage {
        let text = format!(
            "id = \"{id}\"\nname = \"{id}\"\nversion = \"{version}\"\napi = \"0.2\"\ncapabilities = [\"registries\"]\n\n[dependencies]\n{deps}\n[wasm]\nsha512 = \"{}\"\ntoolchain = \"{toolchain}\"\nbuilt_with = \"test\"\n{linked}",
            sha.to_string().repeat(128)
        );
        WasmPackage {
            manifest: Manifest::parse(&text).unwrap(),
            module: Vec::new(),
        }
    }

    fn ids(packages: &[WasmPackage], groups: &[Vec<usize>]) -> Vec<Vec<String>> {
        groups
            .iter()
            .map(|g| g.iter().map(|&i| packages[i].manifest.id.clone()).collect())
            .collect()
    }

    #[test]
    fn mods_sharing_a_library_share_one_group_in_dependency_order() {
        let packages = vec![
            package("b", "1.0.0", "lib = \"^1\"", 'b', "t", ""),
            package("lib", "1.2.0", "", 'a', "t", ""),
            package("c", "1.0.0", "lib = \"^1\"", 'c', "t", ""),
            package("alone", "1.0.0", "", 'd', "t", ""),
        ];
        let (groups, refused) = groups(&packages);
        assert!(refused.is_empty());
        assert_eq!(
            ids(&packages, &groups),
            [vec!["lib", "b", "c"], vec!["alone"]]
        );
    }

    #[test]
    fn bundle_ties_and_cycle_refusals_match_the_native_resolver() {
        let packages = vec![
            package("z", "1", "lib = '^1'", 'b', "t", ""),
            package("lib", "1.2", "", 'a', "t", ""),
            package("a", "1", "lib = '^1'", 'c', "t", ""),
        ];
        let (group, refused) = groups(&packages);
        assert!(refused.is_empty());
        assert_eq!(ids(&packages, &group), [vec!["lib", "z", "a"]]);
        let declared: Vec<_> = packages
            .iter()
            .map(|p| dependencies::from_manifest(&p.manifest.dependencies))
            .collect();
        let native: Vec<_> = packages
            .iter()
            .zip(&declared)
            .map(|(p, dependencies)| dependencies::Mod {
                id: &p.manifest.id,
                version: &p.manifest.version,
                has_api: true,
                dependencies,
            })
            .collect();
        assert_eq!(group[0], dependencies::resolve(&native).unwrap().order);
        let cyclic = vec![
            package("a", "1", "b = '^1'", 'a', "t", ""),
            package("b", "1", "a = '^1'", 'b', "t", ""),
            package("user", "1", "a = '^1'", 'c', "t", ""),
            package("free", "1", "", 'd', "t", ""),
        ];
        let (group, refused) = groups(&cyclic);
        assert_eq!(ids(&cyclic, &group), [vec!["free"]]);
        assert!(
            matches!(&refused[0].1, Refusal::DependencyOrder { reason } if reason.contains("a -> b -> a"))
        );
        assert!(matches!(&refused[1].1, Refusal::DependencyOrder { .. }));
        assert!(matches!(&refused[2].1, Refusal::DependencyRefused { .. }));
    }

    #[test]
    fn a_missing_required_dependency_refuses_the_mod_and_its_dependents() {
        let packages = vec![
            package("lib", "2.0.0", "", 'a', "t", ""),
            package("b", "1.0.0", "lib = \"^1\"", 'b', "t", ""),
            package(
                "c",
                "1.0.0",
                "b = \"^1\"\nextra = { version = \"^1\", optional = true }",
                'c',
                "t",
                "",
            ),
        ];
        let (groups, refused) = groups(&packages);
        assert_eq!(ids(&packages, &groups), [vec!["lib"]]);
        assert_eq!(
            refused,
            [
                (
                    "b".to_string(),
                    Refusal::MissingDependency {
                        dependency: "lib".into(),
                        version: "^1".into()
                    }
                ),
                (
                    "c".to_string(),
                    Refusal::DependencyRefused {
                        dependency: "b".into()
                    }
                ),
            ]
        );
    }

    #[test]
    fn another_build_of_a_dependency_or_toolchain_is_refused() {
        let linked = format!("[wasm.linked]\nlib = \"{}\"\n", "f".repeat(128));
        let packages = vec![
            package("lib", "1.0.0", "", 'a', "t", ""),
            package("b", "1.0.0", "lib = \"^1\"", 'b', "t", &linked),
            package("c", "1.0.0", "lib = \"^1\"", 'c', "other", ""),
        ];
        let (_, refused) = groups(&packages);
        assert_eq!(
            refused,
            [
                (
                    "b".to_string(),
                    Refusal::LinkedAgainst {
                        dependency: "lib".into()
                    }
                ),
                (
                    "c".to_string(),
                    Refusal::Toolchain {
                        found: "other".into(),
                        group: "t".into()
                    }
                ),
            ]
        );
    }
}
