// hidden: a module of game-side types; mods use their re-exports
#[doc(hidden)]
pub mod health;
#[horizon_mod_api_macros::wasm(via = DamageValues)]
pub use crate::health::DamageContainer;
pub use crate::health::{DamageValues, HurtSource, Reduction, ShieldBlock, Victim};
