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
pub enum PackageSide { Client, Both }

/// Read-only package metadata available during registration.
#[non_exhaustive]
pub struct PackageInfo {
    id: String,
    kind: PackageKind,
    manifest: Option<horizon_mods::Manifest>,
}

impl PackageInfo {
    /// Package identifier.
    pub fn id(&self) -> &str { &self.id }
    /// The package's version, when its manifest is available.
    pub fn version(&self) -> Option<&str> { self.manifest.as_ref().map(|m| m.version.as_str()) }
    /// Whether this package contains code.
    pub fn kind(&self) -> PackageKind { self.kind }
    /// Manifest side, if the manifest is available.
    pub fn side(&self) -> Option<PackageSide> { self.manifest.as_ref().map(|m| match m.side { horizon_mods::Side::Client => PackageSide::Client, horizon_mods::Side::Both => PackageSide::Both }) }
    /// Additional namespaces declared by this package.
    pub fn namespaces(&self) -> &[String] { self.manifest.as_ref().map_or(&[], |m| &m.namespaces) }
    /// Whether this package declares a dependency on `id`.
    pub fn depends_on(&self, id: &str) -> bool { self.manifest.as_ref().is_some_and(|m| m.dependencies.contains_key(id)) }
}

static PACKAGES: OnceLock<Vec<PackageInfo>> = OnceLock::new();

pub(crate) fn install_packages(mods: &[super::ModEntry], content: &[horizon_mods::selection::SelectedPackage]) {
    let mut packages: Vec<_> = mods.iter().map(|m| PackageInfo { id: m.id.into(), kind: PackageKind::Code, manifest: None }).collect();
    for selected in content {
        let manifest = &selected.package.manifest;
        if manifest.native.is_none() && !packages.iter().any(|p| p.id == manifest.id) {
            packages.push(PackageInfo { id: manifest.id.clone(), kind: PackageKind::Content, manifest: Some(manifest.clone()) });
        }
    }
    assert!(PACKAGES.set(packages).is_ok(), "package inventory already installed");
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
        Self {
            entries: PACKAGES.get().map_or(&[], Vec::as_slice),
        }
    }

    /// Package IDs in dispatch order.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(PackageInfo::id)
    }

    /// Packages in dispatch order.
    pub fn iter(&self) -> impl Iterator<Item = &PackageInfo> { self.entries.iter() }

    /// Find an installed package by ID.
    pub fn get(&self, id: &str) -> Option<&PackageInfo> { self.entries.iter().find(|entry| entry.id == id) }
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
