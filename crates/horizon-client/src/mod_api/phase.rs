//! Restricted views for registration and data loading.
use super::ModEntry;
pub use horizon_world::native_load_context::DataLoadOrigin;

/// The bootstrap declarations being collected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RegistrationPhase {
    Components,
    Blocks,
    Items,
    StandaloneModels,
}

/// Read-only view of the currently loaded code mods.
pub struct LoadedMods<'a> {
    entries: &'a [ModEntry],
}

impl LoadedMods<'_> {
    pub(crate) fn new() -> Self {
        Self {
            entries: super::mods(),
        }
    }

    /// Mod IDs in dispatch order.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|entry| entry.id)
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

    /// Read a mod's setting without creating or writing any files.
    pub fn setting(&self, mod_id: &str, key: &str) -> Option<String> {
        self.source.setting(mod_id, key)
    }

    /// The same loaded-mod view available during registration.
    pub fn mods(&self) -> &LoadedMods<'_> {
        &self.mods
    }
}
