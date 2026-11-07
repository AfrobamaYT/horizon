// hidden: the wasm boundary's encoding, used by generated code, not a mod's API
#[doc(hidden)]
pub mod wire;
#[doc(hidden)]
pub use crate::wire_slots::{Id, Slots, Stale};
