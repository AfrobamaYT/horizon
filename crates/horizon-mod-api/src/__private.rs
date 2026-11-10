pub use mod_vocab::__private::*;
#[path = "profiler.rs"]
pub mod profiler;

/// Install platform instrumentation before any mod registration or dispatch.
/// A first guard entry seals the socket; late initialization is refused without
/// replacing a backend or disturbing an in-flight call.
pub fn initialize() -> Result<(), &'static str> {
    crate::content::crash::ensure_installed()
}

/// Install instrumentation before publishing the process-wide mod list.
pub fn install(mods: crate::Mods) {
    let _ = initialize();
    mod_vocab::__private::install(mods);
}
