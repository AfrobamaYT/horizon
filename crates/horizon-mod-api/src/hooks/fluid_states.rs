//! Registry-backed fluid-state facts, available on either owning side.
pub trait Helpers {
    /// A registered fluid's world default and legal properties, or TankOnly.
    /// None means the owning host has no resolved registry.
    #[helper(
        side = "both",
        backend = true,
        target = "horizon_client::game::native_block_states::Game<'_>::native_fluid_state_definition"
    )]
    fn fluid_state_definition();
    /// Validate partial property constraints in input order with indexed errors.
    /// An unavailable or tank-only definition returns NoWorldState.
    #[helper(
        side = "both",
        backend = true,
        target = "horizon_client::game::native_block_states::Game<'_>::native_fluid_state_validate"
    )]
    fn fluid_state_validate();
}
