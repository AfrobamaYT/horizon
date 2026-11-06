//! Read-only fluid-state definitions from the owning host's loaded registry.
use crate::{Backend, BackendRef, FluidKey, FluidState, Host, StateConstraint, StateProperty};

/// A registered fluid's world-state definition, or an identity used only in tanks.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FluidDefinition {
    /// A world fluid's default and legal serialized property values.
    World {
        /// Default world state.
        default: FluidState,
        /// Property names and legal values.
        properties: Vec<StateProperty>,
    },
    /// A registered fluid without world states.
    TankOnly,
}

/// Why fluid-state constraints could not be validated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FluidStateError {
    /// No such property, at this input index.
    UnknownProperty(u32),
    /// An illegal property value, at this input index.
    InvalidValue(u32),
    /// A repeated property, at this input index.
    Duplicate(u32),
    /// The host has no loaded world definition, or the fluid is tank-only.
    NoWorldState,
}
impl crate::NoBackend for FluidStateError {
    fn no_backend() -> Self {
        Self::NoWorldState
    }
}

/// The fluid-state facts service.
// exhaustive: only the host constructs this service
pub struct FluidStates<'a> {
    backend: Option<&'a dyn Backend>,
}
/// Reaches fluid-state facts from any host.
pub trait HostFluidStates {
    /// Facts about registered world and tank-only fluids.
    fn fluid_states(&self) -> FluidStates<'_>;
}
impl<H: Host + ?Sized> HostFluidStates for H {
    fn fluid_states(&self) -> FluidStates<'_> {
        FluidStates {
            backend: self.backend().map(BackendRef::open),
        }
    }
}
impl FluidStates<'_> {
    /// The definition, or none without a resolved registry. Missing prepared world
    /// definitions fail resource loading; they are never reported as tank-only.
    pub fn definition(&self, fluid: FluidKey) -> Option<FluidDefinition> {
        self.backend?.fluid_state_definition(fluid)
    }
    /// Validate partial constraints in input order, without matching a particular state.
    /// Tank-only or unavailable definitions return `NoWorldState`; use `definition`
    /// to distinguish them. Indices identify the first offending constraint.
    pub fn validate(
        &self,
        fluid: FluidKey,
        constraints: &[StateConstraint],
    ) -> Result<(), FluidStateError> {
        self.backend
            .ok_or(FluidStateError::NoWorldState)?
            .fluid_state_validate(fluid, constraints)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn carried_properties_and_unavailable_host() {
        struct Unavailable;
        impl Host for Unavailable {}
        let key = FluidKey(0);
        assert_eq!(Unavailable.fluid_states().definition(key), None);
        assert_eq!(
            Unavailable.fluid_states().validate(key, &[]),
            Err(FluidStateError::NoWorldState)
        );
        let source = FluidState {
            fluid: key,
            amount: 8,
            falling: true,
            source: true,
        };
        let flowing = FluidState {
            source: false,
            falling: false,
            ..source
        };
        assert!(source.matches(&[StateConstraint::new("falling", "true")]));
        assert!(!source.matches(&[StateConstraint::new("level", "8")]));
        assert!(flowing.matches(&[
            StateConstraint::new("level", "8"),
            StateConstraint::new("falling", "false")
        ]));
        for value in ["08", "+8", "9", "0"] {
            assert!(!flowing.matches(&[StateConstraint::new("level", value)]));
        }
        assert!(!flowing.matches(&[StateConstraint::new("unknown", "true")]));
        let empty = FluidState {
            amount: 0,
            source: false,
            falling: false,
            ..source
        };
        assert!(empty.matches(&[]));
        assert!(!empty.matches(&[StateConstraint::new("falling", "false")]));
        assert!(
            !FluidState {
                amount: u8::MAX,
                ..flowing
            }
            .matches(&[StateConstraint::new("level", "8")])
        );
    }
}
