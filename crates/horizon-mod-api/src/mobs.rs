//! Moved here from the game so mods can name it.
#[allow(unused_imports)]
use crate::*;
/// `PathComputationType`: what a path is computed for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathComputationType {
    Land,
    Water,
    Air,
}
impl PathComputationType {
    /// Every kind, in vanilla's order.
    pub const ALL: [Self; 3] = [Self::Land, Self::Water, Self::Air];
}
