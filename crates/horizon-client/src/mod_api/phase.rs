//! Restricted views for registration and data loading.
use super::ModEntry;
pub use horizon_world::native_load_context::DataLoadOrigin;

/// The registry whose declarations are being collected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RegistrationPhase {
    Components,
    Blocks,
    Items,
    StandaloneModels,
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
    mods: &'a [ModEntry],
}

impl RegistrationContext<'_> {
    pub(crate) fn new(phase: RegistrationPhase) -> Self {
        Self {
            phase,
            mods: super::mods(),
        }
    }

    /// The declarations this hook may add to its event.
    pub fn phase(&self) -> RegistrationPhase {
        self.phase
    }

    /// Loaded native mod IDs, in dispatch order.
    pub fn mods(&self) -> impl Iterator<Item = &str> {
        self.mods.iter().map(|entry| entry.id)
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
    source: &'a horizon_world::native_load_context::LoadContext<'a>,
}

impl DataLoadContext<'_> {
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

    /// Loaded native mod IDs, in dispatch order.
    pub fn mods(&self) -> impl Iterator<Item = &str> {
        self.source.mods().iter().map(String::as_str)
    }
}
