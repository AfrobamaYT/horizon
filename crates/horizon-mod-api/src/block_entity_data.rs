//! Errors when declaring a mod block entity's live menu data.

/// Why a mod block entity's menu data slots could not be sized.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlockEntityDataError {
    /// The handle is stale, unloaded, missing, vanilla, client-side or not in the
    /// server level currently in place. A replacement at the same position is
    /// followed, as with the other block-entity handle methods.
    Unavailable,
    /// The requested vector exceeds the game's mod-menu data-slot limit.
    TooManySlots {
        /// Number of slots submitted, without clamping.
        count: u64,
        /// Maximum number of menu data slots accepted by this game.
        limit: u64,
    },
}

impl crate::NoBackend for BlockEntityDataError {
    fn no_backend() -> Self {
        Self::Unavailable
    }
}
