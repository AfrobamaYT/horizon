//! Client input preference reads.

pub trait Helpers {
    /// Returns the current input preferences, including outside a world.
    #[helper(side = "client")]
    fn input_options();
}
