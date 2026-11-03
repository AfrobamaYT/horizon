//! Restricted views for bootstrap registration.
use std::sync::OnceLock;

/// Whether a package contains compiled code or content alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PackageKind {
    Code,
    Content,
}

/// The package's declared side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PackageSide {
    Client,
    Both,
}

/// Read-only package metadata available during registration.
#[non_exhaustive]
pub struct PackageInfo {
    id: String,
    kind: PackageKind,
    manifest: Option<horizon_mods::Manifest>,
}

impl PackageInfo {
    /// Package identifier.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// The package's version, when its manifest is available.
    pub fn version(&self) -> Option<&str> {
        self.manifest.as_ref().map(|m| m.version.as_str())
    }
    /// Whether this package contains code.
    pub fn kind(&self) -> PackageKind {
        self.kind
    }
    /// Manifest side, if the manifest is available.
    pub fn side(&self) -> Option<PackageSide> {
        self.manifest.as_ref().map(|m| match m.side {
            horizon_mods::Side::Client => PackageSide::Client,
            horizon_mods::Side::Both => PackageSide::Both,
        })
    }
    /// The package's own namespace followed by any additional namespaces.
    pub fn namespaces(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.id()).chain(
            self.manifest
                .iter()
                .flat_map(|m| m.namespaces.iter().map(String::as_str)),
        )
    }
    /// Whether this package declares a dependency on `id`.
    pub fn depends_on(&self, id: &str) -> bool {
        self.manifest
            .as_ref()
            .is_some_and(|m| m.dependencies.contains_key(id))
    }
}

static PACKAGES: OnceLock<Vec<PackageInfo>> = OnceLock::new();

#[cfg(test)]
thread_local! {
    static TEST_PACKAGES: std::cell::Cell<Option<&'static [PackageInfo]>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub(crate) fn with_test_packages<R>(ids: &[&str], f: impl FnOnce() -> R) -> R {
    let packages: Vec<_> = ids
        .iter()
        .map(|id| PackageInfo {
            id: (*id).into(),
            kind: PackageKind::Code,
            manifest: None,
        })
        .collect();
    let packages = Box::leak(packages.into_boxed_slice());
    TEST_PACKAGES.set(Some(packages));
    let result = f();
    TEST_PACKAGES.set(None);
    result
}

fn inventory(
    mods: &[super::ModEntry],
    content: &[horizon_mods::selection::SelectedPackage],
) -> Vec<PackageInfo> {
    let mut packages: Vec<_> = mods
        .iter()
        .map(|m| PackageInfo {
            id: m.id.into(),
            kind: PackageKind::Code,
            manifest: None,
        })
        .collect();
    for selected in content {
        let manifest = &selected.package.manifest;
        if manifest.native.is_none() && !packages.iter().any(|p| p.id == manifest.id) {
            packages.push(PackageInfo {
                id: manifest.id.clone(),
                kind: PackageKind::Content,
                manifest: Some(manifest.clone()),
            });
        }
    }
    packages
}

pub(crate) fn install_packages(
    mods: &[super::ModEntry],
    content: &[horizon_mods::selection::SelectedPackage],
) {
    assert!(
        PACKAGES.set(inventory(mods, content)).is_ok(),
        "package inventory already installed"
    );
}

/// The bootstrap declarations being collected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RegistrationPhase {
    Parts,
    Components,
    Blocks,
    Items,
    StandaloneModels,
}

/// Read-only view of the startup package inventory.
pub struct LoadedMods<'a> {
    entries: &'a [PackageInfo],
}

impl LoadedMods<'_> {
    pub(crate) fn new() -> Self {
        #[cfg(test)]
        if let Some(entries) = TEST_PACKAGES.get() {
            return Self { entries };
        }
        Self {
            entries: PACKAGES.get().map_or(&[], Vec::as_slice),
        }
    }

    /// Package IDs in dispatch order.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(PackageInfo::id)
    }

    /// Packages in dispatch order.
    pub fn iter(&self) -> impl Iterator<Item = &PackageInfo> {
        self.entries.iter()
    }

    /// Find an installed package by ID.
    pub fn get(&self, id: &str) -> Option<&PackageInfo> {
        self.entries.iter().find(|entry| entry.id == id)
    }
}

/// A registration phase's view. Declarations go on the hook's event; there is
/// no world, handle constructor, transaction or game Host at this point.
///
/// ```compile_fail
/// use horizon_client::mod_api::{Host, RegistrationContext};
/// fn game(_: &dyn Host) {}
/// fn registration(context: &RegistrationContext<'_>) { game(context); }
/// ```
///
/// ```compile_fail
/// use horizon_client::mod_api::{Host, RegistrationContext};
/// fn registration(context: &RegistrationContext<'_>) { let _ = Host::entity(context, 1); }
/// ```
///
/// ```compile_fail
/// use horizon_client::mod_api::{HostTransaction, RegistrationContext};
/// fn registration(context: &mut RegistrationContext<'_>) {
///     let _: Result<(), ()> = context.transaction(|_, _| Ok(()));
/// }
/// ```
pub struct RegistrationContext<'a> {
    phase: RegistrationPhase,
    mods: LoadedMods<'a>,
}

impl RegistrationContext<'_> {
    pub(crate) fn new(phase: RegistrationPhase) -> Self {
        Self {
            phase,
            mods: LoadedMods::new(),
        }
    }

    /// The declarations this hook may add to its event.
    pub fn phase(&self) -> RegistrationPhase {
        self.phase
    }

    /// The loaded-mod view, independent of this phase's event or registrar.
    pub fn mods(&self) -> &LoadedMods<'_> {
        &self.mods
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mod_api;

    #[test]
    fn inventory_keeps_first_content_package_after_code_mods() {
        struct Code;
        #[crate::mod_api::__subscriptions]
        impl crate::mod_api::Hooks for Code {}
        let mods = vec![super::super::ModEntry::new("runtime", Box::new(Code))];
        let package = |version: &str| {
            let manifest = horizon_mods::Manifest::parse(&format!(
                "id='plain'\nname='Plain'\nversion='{version}'\napi='0.2'\ncapabilities=['registries']\n"
            )).unwrap();
            horizon_mods::selection::SelectedPackage {
                path: Default::default(),
                package: horizon_mods::Package {
                    manifest,
                    icon: None,
                },
            }
        };
        let entries = inventory(&mods, &[package("1"), package("2")]);
        let view = LoadedMods { entries: &entries };
        assert_eq!(view.ids().collect::<Vec<_>>(), ["runtime", "plain"]);
        assert_eq!(view.get("runtime").unwrap().kind(), PackageKind::Code);
        let plain = view.get("plain").unwrap();
        assert_eq!(plain.kind(), PackageKind::Content);
        assert_eq!(plain.version(), Some("1"));
        assert_eq!(plain.side(), Some(PackageSide::Client));
        assert_eq!(plain.namespaces().collect::<Vec<_>>(), ["plain"]);
        assert!(!plain.depends_on("runtime"));
        assert!(view.get("missing").is_none());
    }
}
