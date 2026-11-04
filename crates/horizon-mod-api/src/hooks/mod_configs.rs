//! The `mod_configs` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// The config files native mods keep (NeoForge's `registerConfig`):
    /// push yours, each of a kind, with the text written to `config/` when
    /// the file is missing. Asked by the window when the game starts (its
    /// `Client` and `Common` configs), by a dedicated server when it starts
    /// (`Common` and `Server`) and by a server before each world it opens
    /// (`Server`); a game reads only its own kinds, so push them all every
    /// time. A file outside `config/`, or one already registered, is skipped.
    #[hook(
        side = "both",
        contract = "collect",
        out = "configs",
        neoforge = "ModContainer.registerConfig"
    )]
    fn register_configs();
    /// A registered config was read (`ModConfigEvent.Loading`): `config`
    /// says whose, which file and its text, which the mod parses (TOML by
    /// convention). Every mod hears every config, so check `mod_id`. Fired
    /// where the config's kind loads (`native_register_configs`), a `Server`
    /// one before `native_server_about_to_start`.
    #[hook(
        side = "both",
        contract = "listen",
        neoforge = "ModConfigEvent.Loading",
        cancellable = false
    )]
    fn config_loading();
    /// A loaded config's file changed on disk (`ModConfigEvent.Reloading`):
    /// its new text. The game looks at the files' modification times once a
    /// second, on the window and on the server's tick; a file saved with the
    /// same text, removed or unreadable fires nothing and keeps its last
    /// text.
    #[hook(
        side = "both",
        contract = "listen",
        neoforge = "ModConfigEvent.Reloading",
        cancellable = false
    )]
    fn config_reloading();
    /// A `Server` config is dropped as its server stops
    /// (`ModConfigEvent.Unloading`), after `native_server_stopping`; the next
    /// world reads it again. `Client` and `Common` configs stay until the
    /// game quits.
    #[hook(
        side = "server",
        contract = "listen",
        neoforge = "ModConfigEvent.Unloading",
        cancellable = false
    )]
    fn config_unloading();
}
