//! Carrier transfer fences: the world cells and carriers locked while
//! a carrier ownership operation runs between prepared and committed
//! (issue #2413, slice 2). The set lives on the World so every refusal
//! point — the player's break, the piston hook, the container gate and
//! the mod write gate — reads one place. With no operation in flight
//! every check is one load of an empty set.

use std::collections::HashSet;

/// The locked world cells and carriers of the operations in flight.
#[derive(Default)]
pub struct Fences {
    cells: HashSet<[i32; 3]>,
    carriers: HashSet<u128>,
}
impl Fences {
    /// Whether a world cell is locked against edits, interactions,
    /// pistons and container opening.
    pub fn locks_cell(&self, pos: &[i32; 3]) -> bool {
        !self.cells.is_empty() && self.cells.contains(pos)
    }

    /// Whether a carrier is locked against motion and inventory.
    pub fn locks_carrier(&self, uuid: u128) -> bool {
        !self.carriers.is_empty() && self.carriers.contains(&uuid)
    }

    /// Locks one world cell.
    pub fn lock_cell(&mut self, pos: [i32; 3]) {
        self.cells.insert(pos);
    }

    /// Unlocks one world cell.
    pub fn unlock_cell(&mut self, pos: [i32; 3]) {
        self.cells.remove(&pos);
    }

    /// Locks one carrier.
    pub fn lock_carrier(&mut self, uuid: u128) {
        self.carriers.insert(uuid);
    }

    /// Unlocks one carrier.
    pub fn unlock_carrier(&mut self, uuid: u128) {
        self.carriers.remove(&uuid);
    }
}
