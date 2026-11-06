//! Owning-host fluid-state definitions and constraint validation.
#![allow(unused_variables)]
use crate::{FluidDefinition, FluidKey, FluidStateError, FluidStates, StateConstraint};

/// Reaches fluid-state facts from any host.
pub trait HostFluidStates {
    /// Facts about registered world and tank-only fluids.
    fn fluid_states(&self) -> FluidStates<'_>;
}

/// Read-only fluid-state facts, implemented by the game and reached through the service.
#[doc(hidden)]
pub trait FluidStatesBackend {
    /// Side: both.
    /// A registered fluid's world default and legal properties, or TankOnly.
    /// None means the owning host has no resolved registry.
    fn fluid_state_definition(&self, fluid: FluidKey) -> Option<FluidDefinition> {
        crate::phase::unprovided(
            "Backend::fluid_state_definition",
            "fluid_state_definition",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Validate partial property constraints in input order with indexed errors.
    /// An unavailable or tank-only definition returns NoWorldState.
    fn fluid_state_validate(
        &self,
        fluid: FluidKey,
        constraints: &[StateConstraint],
    ) -> Result<(), FluidStateError> {
        crate::phase::unprovided(
            "Backend::fluid_state_validate",
            "fluid_state_validate",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
