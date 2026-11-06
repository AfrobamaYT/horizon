//! The game's fluid-state facts service implementation.
use crate::game::Game;
use mod_api::{
    FluidDefinition, FluidKey, FluidStateError, StateConstraint, prelude::FluidStatesBackend,
};

impl FluidStatesBackend for Game<'_> {
    fn fluid_state_definition(&self, fluid: FluidKey) -> Option<FluidDefinition> {
        mod_api::__private::check_access(
            "fluid_state_definition",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_fluid_state_definition(fluid)
    }
    fn fluid_state_validate(
        &self,
        fluid: FluidKey,
        constraints: &[StateConstraint],
    ) -> Result<(), FluidStateError> {
        mod_api::__private::check_access(
            "fluid_state_validate",
            mod_api::__private::LoadAccess::WorldOnly,
        );
        self.native_fluid_state_validate(fluid, constraints)
    }
}
