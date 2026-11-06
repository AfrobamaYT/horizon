//! Client option snapshots, readable at any time, including outside a world.

pub trait Helpers {
    /// Returns the current graphics preferences.
    #[notes = r#"## Option snapshots

`ClientHost::graphics_options()`, `gui_options()`, `chat_options()` and `audio_options()` each
return a copy of one group of the player's preferences, even outside a world, like
`input_options()`. Read again after the settings change; an older snapshot stays unchanged.
A zoom, brightness, HUD or chat mod reads these instead of keeping its own copy of the player's
choices. The fields are documented in the API reference."#]
    #[helper(side = "client")]
    fn graphics_options();
    /// Returns the current GUI and accessibility preferences.
    #[helper(side = "client")]
    fn gui_options();
    /// Returns the current chat preferences.
    #[helper(side = "client")]
    fn chat_options();
    /// Returns the current audio preferences.
    #[helper(side = "client")]
    fn audio_options();
}
