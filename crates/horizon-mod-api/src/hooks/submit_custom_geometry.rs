pub trait DrawContexts {
    /// Per-view untextured geometry, cleared even without listeners; `usage` reports fixed setup-time limits shared across listeners and depth modes.
    #[draw_context(
        surface = "crate::world_geometry::WorldGeometry",
        name = "WorldDrawContext",
        prefix = "world_draw_"
    )]
    fn world_draw_context();
}
pub trait Hooks {
    /// Submit unlit geometry after particle submission, before opaque rendering
    /// (NeoForge 26.3 `SubmitCustomGeometryEvent`, vanilla `LevelRenderer.submitFeatures`).
    /// Client-only, noncancellable. `view` holds only camera, partial tick and game time,
    /// not full extracted level state or visible sections. Submit from extracted state.
    #[notes = r#"## Custom world geometry

`Hooks::submit_custom_geometry` runs on the client after particles, before rendering. Its
`WorldRenderView` holds camera, partial tick and game time; `WorldDrawContext::lines` and `quads`
accept unlit world-space ARGB geometry with depth testing or through walls. Batches fail atomically
for invalid inputs or overflow. `usage()` reports shared limits (4,096 of each by default);
output clears every view. Submit extracted state, without world writes or resource loading."#]
    #[hook(
        side = "client",
        contract = "listen",
        neoforge = "SubmitCustomGeometryEvent",
        cancellable = false
    )]
    fn submit_custom_geometry();
}
pub trait Helpers {
    #[helper(
        side = "client",
        target = "horizon_client::game::native_custom_geometry::Game<'_>::native_world_draw_lines"
    )]
    fn world_draw_lines();
    #[helper(
        side = "client",
        target = "horizon_client::game::native_custom_geometry::Game<'_>::native_world_draw_quads"
    )]
    fn world_draw_quads();
    #[helper(
        side = "client",
        target = "horizon_client::game::native_custom_geometry::Game<'_>::native_world_draw_usage"
    )]
    fn world_draw_usage();
}
