//! Refusals when accessing a mod block entity's internal item slots.

/// Why a mod block entity's internal item slot could not be read or written.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlockEntityItemError {
    /// Missing, stale, unloaded, vanilla or not a configured mod container.
    /// Writes also refuse client-side and parked-level targets.
    Unavailable,
    /// The index is outside the current container's declared slots.
    SlotOutOfBounds {
        /// Submitted index.
        slot: u32,
        /// Current declared slot count.
        slots: u32,
    },
    /// The incoming stack has an unregistered item, air or zero count.
    /// Use None to clear a slot.
    InvalidStack,
    /// The incoming count exceeds the registered or component-overridden limit.
    StackTooLarge {
        /// Submitted count, without clamping.
        count: u32,
        /// Maximum count for this stack.
        limit: u32,
    },
}

impl crate::NoBackend for BlockEntityItemError {
    fn no_backend() -> Self {
        Self::Unavailable
    }
}
