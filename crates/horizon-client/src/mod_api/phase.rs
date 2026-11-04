//! Restricted views for registration and data loading.
pub use horizon_world::native_load_context::DataLoadOrigin;
use std::sync::OnceLock;

/// Whether a package contains compiled code or content alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PackageKind {
    Code,
    Content,
}

/// Read-only package metadata available during registration.
#[non_exhaustive]
pub struct PackageInfo {
    kind: PackageKind,
    manifest: horizon_mods::Manifest,
}

impl PackageInfo {
    /// Package identifier.
    pub fn id(&self) -> &str {
        &self.manifest.id
    }
    /// The package's version.
    pub fn version(&self) -> &str {
        &self.manifest.version
    }
    /// Whether this package contains code.
    pub fn kind(&self) -> PackageKind {
        self.kind
    }
    /// The package's declared side.
    pub fn side(&self) -> horizon_mods::Side {
        self.manifest.side
    }
    /// The package's own namespace followed by any additional namespaces.
    pub fn namespaces(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.id()).chain(self.manifest.namespaces.iter().map(String::as_str))
    }
    /// Whether this package declares a dependency on `id`.
    pub fn depends_on(&self, id: &str) -> bool {
        self.manifest.dependencies.contains_key(id)
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
            kind: PackageKind::Code,
            manifest: horizon_mods::Manifest::parse(&format!(
                "id='{id}'\nname='{id}'\nversion='1'\napi='0.2'\ncapabilities=['registries']\n"
            ))
            .unwrap(),
        })
        .collect();
    let packages = Box::leak(packages.into_boxed_slice());
    TEST_PACKAGES.set(Some(packages));
    let result = f();
    TEST_PACKAGES.set(None);
    result
}

fn inventory(
    native: &[horizon_mods::packages::AdjacentManifest],
    content: &[horizon_mods::selection::SelectedPackage],
) -> Vec<PackageInfo> {
    let mut packages: Vec<_> = native
        .iter()
        .map(|p| PackageInfo {
            kind: PackageKind::Code,
            manifest: p.manifest.clone(),
        })
        .collect();
    for selected in content {
        let manifest = &selected.package.manifest;
        if !packages.iter().any(|p| p.id() == manifest.id) {
            packages.push(PackageInfo {
                kind: PackageKind::Content,
                manifest: manifest.clone(),
            });
        }
    }
    packages
}

pub(crate) fn install_packages(
    native: &[horizon_mods::packages::AdjacentManifest],
    content: &[horizon_mods::selection::SelectedPackage],
) {
    assert!(
        PACKAGES.set(inventory(native, content)).is_ok(),
        "package inventory already installed"
    );
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

    /// Find a package by ID. This linear lookup is intended for registration, not hot paths.
    pub fn get(&self, id: &str) -> Option<&PackageInfo> {
        self.entries.iter().find(|entry| entry.id() == id)
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
    mods: LoadedMods<'a>,
}

impl RegistrationContext<'_> {
    pub(crate) fn new() -> Self {
        Self {
            mods: LoadedMods::new(),
        }
    }

    /// The loaded-mod view, independent of this phase's event or registrar.
    pub fn mods(&self) -> &LoadedMods<'_> {
        &self.mods
    }
}

/// A data load's read-only view. The owning loader supplies its origin and
/// metadata; reading already-loaded data never creates this context.
///
/// ```compile_fail
/// use horizon_client::mod_api::{DataLoadContext, Host};
/// fn game(_: &dyn Host) {}
/// fn load(context: &DataLoadContext<'_>) { game(context); }
/// ```
///
/// ```compile_fail
/// use horizon_client::mod_api::{DataLoadContext, Host};
/// fn load(context: &DataLoadContext<'_>) { let _ = Host::entity(context, 1); }
/// ```
///
/// ```compile_fail
/// use horizon_client::mod_api::{DataLoadContext, HostTransaction};
/// fn load(context: &mut DataLoadContext<'_>) {
///     let _: Result<(), ()> = context.transaction(|_, _| Ok(()));
/// }
/// ```
pub struct DataLoadContext<'a> {
    source: &'a horizon_world::native_load_context::LoadContext,
    mods: LoadedMods<'a>,
}

impl<'a> DataLoadContext<'a> {
    pub(crate) fn new(source: &'a horizon_world::native_load_context::LoadContext) -> Self {
        Self {
            source,
            mods: LoadedMods::new(),
        }
    }

    /// The phase selected at the owning load boundary.
    pub fn origin(&self) -> DataLoadOrigin {
        self.source.origin()
    }

    /// Registry names in ID order, from this resource generation's snapshot.
    pub fn registry(&self, name: &str) -> Option<&[String]> {
        self.source.registry(name)
    }

    /// A loaded mod's setting captured at the owning load boundary, without file I/O.
    pub fn setting(&self, mod_id: &str, key: &str) -> Option<String> {
        self.source.setting(mod_id, key)
    }

    /// The same loaded-mod view available during registration.
    pub fn mods(&self) -> &LoadedMods<'_> {
        &self.mods
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_keeps_first_content_package_after_code_mods() {
        let native = [horizon_mods::packages::AdjacentManifest {
            path: Default::default(),
            manifest: horizon_mods::Manifest::parse(
                "id='runtime'\nname='Runtime'\nversion='3'\napi='0.2'\ncapabilities=['registries']\n",
            )
            .unwrap(),
        }];
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
        let entries = inventory(&native, &[package("1"), package("2")]);
        let view = LoadedMods { entries: &entries };
        assert_eq!(view.ids().collect::<Vec<_>>(), ["runtime", "plain"]);
        let runtime = view.get("runtime").unwrap();
        assert_eq!(runtime.kind(), PackageKind::Code);
        assert_eq!(runtime.version(), "3");
        assert_eq!(runtime.side(), horizon_mods::Side::Client);
        let plain = view.get("plain").unwrap();
        assert_eq!(plain.kind(), PackageKind::Content);
        assert_eq!(plain.version(), "1");
        assert_eq!(plain.side(), horizon_mods::Side::Client);
        assert_eq!(plain.namespaces().collect::<Vec<_>>(), ["plain"]);
        assert!(!plain.depends_on("runtime"));
        assert!(view.get("missing").is_none());
    }
}
