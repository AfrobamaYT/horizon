//! The game's slot table, in the one registry every point table shares
//! (`mod_api::__private::points`).
use crate::{POINT_COUNT, PointId};
use mod_api::__private::points::{Place, Table};
use std::sync::atomic::AtomicBool;

#[cfg(not(target_arch = "wasm32"))]
pub use mod_api::__private::points::register;
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
