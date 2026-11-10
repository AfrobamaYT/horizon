use crate::{BlockState, EntityRef};

/// Vanilla's `GameEvent.Context`: optional source and affected block state.
/// Source facts used by vibration listeners are captured when the event is emitted.
// exhaustive: mirrors vanilla's complete two-field GameEvent.Context value for literal construction.
#[derive(Clone, Copy, Debug, Default, PartialEq, horizon_mod_api_macros::Wire)]
pub struct GameEventContext {
    /// Entity responsible for the event, if any. Must be live in the level.
    pub source_entity: Option<EntityRef>,
    /// State affected by the action, which may differ from the state now in the world.
    pub affected_state: Option<BlockState>,
}
