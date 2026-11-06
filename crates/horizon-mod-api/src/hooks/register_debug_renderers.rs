//! The `register_debug_renderers` hooks: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// The world-space debug renderers a mod adds (NeoForge's `RegisterDebugRenderersEvent`): push a
    /// `DebugRenderer` per renderer, its id `<mod id>:<path>`. Asked once at start, on the client. An id
    /// in vanilla's namespace, not of that form, or taken by a debug entry or renderer already is logged
    /// and left out. Each renderer is listed and switched on or off with vanilla's, and
    /// `debug_renderer_draw` draws it while it is on.
    #[notes = r#"## Debug renderers

`Hooks::register_debug_renderers` is the client registration-phase counterpart of NeoForge
26.3's `RegisterDebugRenderersEvent`. It receives a `RegistrationContext`, once at client
startup, and collects a `DebugRenderer` per renderer: an id `<mod id>:<path>`. An id in
vanilla's namespace, not of that form, or taken by a debug entry or renderer already is logged
and left out.

A renderer is listed in the debug options' renderer section, sorted by its id with vanilla's,
and is switched like them (`alwaysOn`, `inOverlay`, `never`), saved in `debug-profile.json`
under its full id. It starts off, and reduced debug info hides it, as it hides vanilla's.

While it is on, `Hooks::debug_renderer_draw` is asked for each view of a world, after
`submit_custom_geometry`, with the renderer's id, the same `WorldRenderView` and the same
`WorldDrawContext`, so [its limits and rules](#custom-world-geometry) apply and are shared. Every
mod is asked about every id, so a mod ignores ids it does not own. Output clears every view and
title or connect screens ask no renderer, so nothing stays after leaving a world. A disabled
renderer is never asked, and with none registered a frame pays one test."#]
    #[hook(
        side = "client",
        contract = "collect",
        out = "renderers",
        context = "registration",
        neoforge = "RegisterDebugRenderersEvent"
    )]
    fn register_debug_renderers();
}
