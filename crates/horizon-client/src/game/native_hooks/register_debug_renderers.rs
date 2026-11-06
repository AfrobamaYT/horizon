use super::*;

/// The hook points. Each does nothing in Horizon.
impl Game<'_> {
    /// The world-space debug renderers a mod adds (NeoForge's
    /// `RegisterDebugRenderersEvent`): push a `DebugRenderer` per renderer, its id
    /// `<mod id>:<path>`. Asked once at start, on the client.
    pub(in crate::game) fn native_register_debug_renderers(
        context: &mod_api::RegistrationContext,
        renderers: &mut Vec<DebugRenderer>,
    ) {
        Self::fire_register_debug_renderers(context, renderers);
    }
}
