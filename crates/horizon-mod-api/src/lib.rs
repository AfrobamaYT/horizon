//! Mod-facing API, re-exporting the engine vocabulary.
extern crate self as mod_api;

pub use mod_vocab::*;
/// Shared Wire values and the mod-side transport.
pub mod wire {
    pub use mod_vocab::wire::*;
    #[path = "wire_resources.rs"]
    mod resources;
    mod static_strings;
    pub(crate) fn initialize() -> Result<(), &'static str> {
        resources::ensure_installed()
    }
    #[doc(hidden)]
    pub use resources::{
        ResourceCodec, ResourceScope, ScopeError, ScopeLease, ScopeToken, borrow_token,
    };
    /// The mod side of the WebAssembly boundary.
    #[cfg(any(target_arch = "wasm32", feature = "wasm-guest"))]
    pub mod wire_guest;
    #[cfg(any(target_arch = "wasm32", feature = "wasm-guest"))]
    pub use wire_guest as guest;
}

/// Platform-owned crash attribution with shared vocabulary types.
#[doc(hidden)]
pub mod content {
    #[doc(hidden)]
    pub mod crash;
}
/// Platform runtime operations and engine-owned vocabulary installations.
#[doc(hidden)]
pub mod __private;

mod load_state;
#[doc(hidden)]
pub use load_state::install_guest_packages;

mod fluid_state;
mod parts;

#[cfg(any(test, feature = "testing"))]
pub mod testing;
