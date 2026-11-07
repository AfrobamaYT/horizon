#[horizon_mod_api_macros::wasm(via = String)]
pub use horizon_client_core::particle_type::ParticleType;
mod suggestion;
pub use suggestion::Suggestion;
