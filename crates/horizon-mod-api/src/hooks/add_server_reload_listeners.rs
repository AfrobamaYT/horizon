//! The `add_server_reload_listeners` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// Register server data reload listeners after packs are selected. The
    /// host gives registry access at registration, as
    /// `AddServerReloadListenersEvent.getRegistryAccess` does.
    #[hook(
        side = "server",
        contract = "listen",
        host = "read",
        neoforge = "AddServerReloadListenersEvent"
    )]
    fn add_server_reload_listeners();
}
