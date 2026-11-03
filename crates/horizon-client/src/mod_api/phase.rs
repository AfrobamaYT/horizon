//! Restricted contexts for phases that run before a game exists.
use super::ModEntry;

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
