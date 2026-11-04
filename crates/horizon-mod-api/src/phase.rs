//! Restricted views for registration and data loading.
pub use horizon_world::native_load_context::{DataLoadOrigin, LoopSide};
use std::sync::{Arc, OnceLock};

/// Whether a package contains compiled code or content alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PackageKind {
    /// The package carries compiled code.
    Code,
    /// The package carries content alone: data and assets.
    Content,
}

/// Read-only package metadata available during registration.
#[non_exhaustive]
pub struct PackageInfo {
    pub(crate) id: String,
    pub(crate) version: String,
    pub(crate) kind: PackageKind,
    pub(crate) side: horizon_mods::Side,
    /// Its own namespace, then the others it declares.
    pub(crate) namespaces: Vec<String>,
    /// The ids it declares dependencies on.
    pub(crate) dependencies: Vec<String>,
}

impl PackageInfo {
    fn new(kind: PackageKind, manifest: &horizon_mods::Manifest) -> Self {
        Self {
            id: manifest.id.clone(),
            version: manifest.version.clone(),
            kind,
            side: manifest.side,
            namespaces: std::iter::once(&manifest.id)
                .chain(&manifest.namespaces)
                .cloned()
                .collect(),
            dependencies: manifest.dependencies.keys().cloned().collect(),
        }
    }
    /// Package identifier.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// The package's version.
    pub fn version(&self) -> &str {
        &self.version
    }
    /// Whether this package contains code.
    pub fn kind(&self) -> PackageKind {
        self.kind
    }
    /// The package's declared side.
    pub fn side(&self) -> horizon_mods::Side {
        self.side
    }
    /// The package's own namespace followed by any additional namespaces.
    pub fn namespaces(&self) -> impl Iterator<Item = &str> {
        self.namespaces.iter().map(String::as_str)
    }
    /// Whether this package declares a dependency on `id`.
    pub fn depends_on(&self, id: &str) -> bool {
        self.dependencies.iter().any(|d| d == id)
    }
}

static PACKAGES: OnceLock<Arc<[PackageInfo]>> = OnceLock::new();

#[cfg(any(test, feature = "testing"))]
thread_local! {
    static TEST_PACKAGES: std::cell::RefCell<Option<Arc<[PackageInfo]>>> = const { std::cell::RefCell::new(None) };
}
/// Runs `f` with only the packages `ids` installed, for tests.
#[cfg(any(test, feature = "testing"))]
pub fn with_test_packages<R>(ids: &[&str], f: impl FnOnce() -> R) -> R {
    let packages: Vec<_> = ids
        .iter()
        .map(|id| {
            let manifest = horizon_mods::Manifest::parse(&format!(
                "id='{id}'\nname='{id}'\nversion='1'\napi='0.2'\ncapabilities=['registries']\n"
            ))
            .unwrap();
            PackageInfo::new(PackageKind::Code, &manifest)
        })
        .collect();
    TEST_PACKAGES.set(Some(packages.into()));
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
        .map(|p| PackageInfo::new(PackageKind::Code, &p.manifest))
        .collect();
    for selected in content {
        let manifest = &selected.package.manifest;
        // A wasm package is code, selected with the content packages.
        let kind = match manifest.wasm {
            Some(_) => PackageKind::Code,
            None => PackageKind::Content,
        };
        if !packages.iter().any(|p| p.id() == manifest.id) {
            packages.push(PackageInfo::new(kind, manifest));
        }
    }
    packages
}

/// Records the package inventory once at startup; a second call panics.
pub fn install_packages(
    native: &[horizon_mods::packages::AdjacentManifest],
    content: &[horizon_mods::selection::SelectedPackage],
) {
    assert!(
        PACKAGES.set(inventory(native, content).into()).is_ok(),
        "package inventory already installed"
    );
}

/// Read-only view of the startup package inventory, shared: cloning it copies no package.
#[derive(Clone)]
// exhaustive: its fields are private, so mods cannot build or destructure it
pub struct LoadedMods {
    pub(crate) entries: Arc<[PackageInfo]>,
}

impl LoadedMods {
    pub(crate) fn new() -> Self {
        #[cfg(any(test, feature = "testing"))]
        if let Some(entries) = TEST_PACKAGES.with_borrow(Clone::clone) {
            return Self { entries };
        }
        Self {
            entries: PACKAGES.get().cloned().unwrap_or_else(|| Arc::new([])),
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
/// use mod_api::{Host, RegistrationContext};
/// fn game(_: &dyn Host) {}
/// fn registration(context: &RegistrationContext) { game(context); }
/// ```
///
/// ```compile_fail
/// use mod_api::{Host, RegistrationContext};
/// fn registration(context: &RegistrationContext) { let _ = Host::entity(context, 1); }
/// ```
///
/// ```compile_fail
/// use mod_api::{HostTransaction, RegistrationContext};
/// fn registration(context: &mut RegistrationContext) {
///     let _: Result<(), ()> = context.transaction(|_, _| Ok(()));
/// }
/// ```
// exhaustive: its fields are private, so mods cannot build or destructure it
pub struct RegistrationContext {
    pub(crate) mods: LoadedMods,
}

impl RegistrationContext {
    pub(crate) fn new() -> Self {
        Self {
            mods: LoadedMods::new(),
        }
    }

    /// The loaded-mod view, independent of this phase's event or registrar.
    pub fn mods(&self) -> &LoadedMods {
        &self.mods
    }
}

/// A data load's read-only view. The owning loader supplies its origin and
/// metadata; reading already-loaded data never creates this context.
///
/// ```compile_fail
/// use mod_api::{DataLoadContext, Host};
/// fn game(_: &dyn Host) {}
/// fn load(context: &dyn DataLoadContext) { game(context); }
/// ```
///
/// ```compile_fail
/// use mod_api::{DataLoadContext, Host};
/// fn load(context: &dyn DataLoadContext) { let _ = Host::entity(context, 1); }
/// ```
///
/// ```compile_fail
/// use mod_api::{DataLoadContext, HostTransaction};
/// fn load(context: &mut dyn DataLoadContext) {
///     let _: Result<(), ()> = context.transaction(|_, _| Ok(()));
/// }
/// ```
pub trait DataLoadContext {
    /// The phase selected at the owning load boundary.
    fn origin(&self) -> DataLoadOrigin;

    /// Registry names in ID order, from this resource generation's snapshot.
    fn registry(&self, name: &str) -> Option<Arc<[String]>>;

    /// The fluid tags this load's resources hold (`data/<ns>/tags/fluid`):
    /// each tag id to the fluids it names, in the sources' order — what a
    /// recipe's fluid ingredient resolves its `#tag`s with. `None` where the
    /// load reads no recipe resources.
    fn fluid_tags(&self) -> Option<Arc<[(String, Arc<[String]>)]>> {
        None
    }

    /// A loaded mod's setting captured at the owning load boundary, without file I/O.
    fn setting(&self, mod_id: &str, key: &str) -> Option<String>;

    /// The same loaded-mod view available during registration.
    fn mods(&self) -> LoadedMods;
}

/// The game's owning load, as a [`DataLoadContext`].
// hidden: the game's side of the context, not part of the mod surface
#[doc(hidden)]
// exhaustive: its fields are private; only the game builds it
pub struct OwningLoad<'a> {
    source: &'a horizon_world::native_load_context::LoadContext,
    mods: LoadedMods,
}

impl<'a> OwningLoad<'a> {
    pub fn new(source: &'a horizon_world::native_load_context::LoadContext) -> Self {
        Self {
            source,
            mods: LoadedMods::new(),
        }
    }
}

impl DataLoadContext for OwningLoad<'_> {
    fn origin(&self) -> DataLoadOrigin {
        self.source.origin()
    }
    fn registry(&self, name: &str) -> Option<Arc<[String]>> {
        self.source.registry(name)
    }
    fn fluid_tags(&self) -> Option<Arc<[(String, Arc<[String]>)]>> {
        Some(self.source.fluid_tags().clone())
    }
    fn setting(&self, mod_id: &str, key: &str) -> Option<String> {
        self.source.setting(mod_id, key)
    }
    fn mods(&self) -> LoadedMods {
        self.mods.clone()
    }
}

/// The view of a hook fired inside a menu, recipe or player-list loop, where no
/// game is at hand: the side, the registries, the settings and the mod list.
/// The side that owns the loop builds it once per resource generation; a hook
/// never builds one, and its answers never change while it is held.
///
/// ```compile_fail
/// use mod_api::{Host, LoopContext};
/// fn game(_: &dyn Host) {}
/// fn each(context: &dyn LoopContext) { game(context); }
/// ```
///
/// ```compile_fail
/// use mod_api::{HostTransaction, LoopContext};
/// fn each(context: &mut dyn LoopContext) {
///     let _: Result<(), ()> = context.transaction(|_, _| Ok(()));
/// }
/// ```
pub trait LoopContext {
    /// The side whose loop fires the hook.
    fn side(&self) -> LoopSide;

    /// Registry names in ID order, from the resource generation's snapshot.
    fn registry(&self, name: &str) -> Option<Arc<[String]>>;

    /// A loaded mod's setting captured when this context was built, without file I/O.
    fn setting(&self, mod_id: &str, key: &str) -> Option<String>;

    /// The same loaded-mod view available during registration.
    fn mods(&self) -> LoadedMods;
}

/// The game's loop, as a [`LoopContext`].
// hidden: the game's side of the context, not part of the mod surface
#[doc(hidden)]
// exhaustive: its fields are private; only the game builds it
pub struct OwningLoop<'a> {
    source: &'a horizon_world::native_load_context::LoopContext,
    mods: LoadedMods,
}

impl<'a> OwningLoop<'a> {
    pub fn new(source: &'a horizon_world::native_load_context::LoopContext) -> Self {
        Self {
            source,
            mods: LoadedMods::new(),
        }
    }
}

impl LoopContext for OwningLoop<'_> {
    fn side(&self) -> LoopSide {
        self.source.side()
    }
    fn registry(&self, name: &str) -> Option<Arc<[String]>> {
        self.source.registry(name)
    }
    fn setting(&self, mod_id: &str, key: &str) -> Option<String> {
        self.source.setting(mod_id, key)
    }
    fn mods(&self) -> LoadedMods {
        self.mods.clone()
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
        let view = LoadedMods {
            entries: entries.into(),
        };
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

    #[test]
    fn a_loop_context_reads_its_side_registries_and_mods() {
        use horizon_world::native_load_context::{DataLoadOrigin, LoadContext, LoadInputs};
        let registries =
            std::collections::BTreeMap::from([("item".to_owned(), vec!["a:b".into()])]);
        let load = LoadContext::new(
            DataLoadOrigin::ClientPreload,
            std::sync::Arc::new(LoadInputs::new(registries, Default::default())),
            [],
        );
        let source = load.loop_context(LoopSide::Client, []);
        with_test_packages(&["one", "two"], || {
            let context = OwningLoop::new(&source);
            assert_eq!(context.side(), LoopSide::Client);
            assert_eq!(
                context.registry("item").as_deref(),
                Some(&["a:b".to_owned()][..])
            );
            assert_eq!(context.registry("block"), None);
            assert_eq!(context.setting("one", "mode"), None);
            assert_eq!(context.mods().ids().collect::<Vec<_>>(), ["one", "two"]);
        });
    }
}
