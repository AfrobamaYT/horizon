//! The game's slot table, in the one registry every point table shares
//! (`mod_api::__private::points`).
use crate::{POINT_COUNT, PointId};
use mod_api::__private::points::{Place, Table};
use std::sync::atomic::AtomicBool;

/// Registers a listener, refusing game points when this build has no sites.
#[cfg(not(target_arch = "wasm32"))]
pub fn register(slot: &mod_api::__private::points::SlotListener) -> Result<(), RegisterError> {
    if !crate::STAGED
        && std::ptr::eq(slot.place().table(), &TABLE)
        && slot.place().index() < POINT_COUNT
    {
        return Err(RegisterError::Unstaged {
            id: slot.id(),
            target: crate::POINT_TARGETS[slot.place().index()],
        });
    }
    mod_api::__private::points::register(slot)
}
pub use mod_api::__private::points::{RegisterError, seal, stop};

/// Each place's slot: set once, at the seal, where a mod listens. A point that nothing listens at
/// costs one load of it.
pub static POINTS: [AtomicBool; POINT_COUNT] = [const { AtomicBool::new(false) }; POINT_COUNT];
/// The game's API point table.
pub static TABLE: Table = Table::new(&POINTS);

impl PointId {
    /// This place in the game's table, for a listener's constructor.
    pub const fn place(self) -> Place {
        Place::new(&TABLE, self as usize)
    }
}
