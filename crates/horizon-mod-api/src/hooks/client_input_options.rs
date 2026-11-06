//! Client input preference reads.

pub trait Helpers {
    /// Returns the current input preferences, including outside a world.
    #[notes = r#"## Input preferences

`ClientHost::input_options()` returns a copy of the current mouse and movement
input preferences, even outside a world. Read again after settings change; an
older snapshot stays unchanged. The fields are documented in the API reference."#]
    #[helper(side = "client")]
    fn input_options();
}
