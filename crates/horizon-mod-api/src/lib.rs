//! Mod-facing API, re-exporting the engine vocabulary.
extern crate self as mod_api;

pub use mod_vocab::*;
/// Shared Wire values and the mod-side transport.
pub mod wire {
    pub use mod_vocab::wire::*;
    /// The mod side of the WebAssembly boundary.
    #[cfg(any(target_arch = "wasm32", feature = "wasm-guest"))]
    pub mod wire_guest;
    #[cfg(any(target_arch = "wasm32", feature = "wasm-guest"))]
    pub use wire_guest as guest;
}

#[cfg(all(test, feature = "wasm-guest"))]
mod phase_tests {
    use super::*;
    use prelude::*;
    #[test]
    fn frame_services_do_not_read_guest_phase() {
        let _phase = __private::PhaseOwner::scope(__private::ModPhase::Packages);
        let mut guest = wire::guest::Guest;
        let _: &dyn Messages = guest.messages();
        let _: &dyn Players = guest.players();
        let _: &dyn Levels = guest.levels();
        let _: &mut dyn Commands = guest.commands();
    }
}
