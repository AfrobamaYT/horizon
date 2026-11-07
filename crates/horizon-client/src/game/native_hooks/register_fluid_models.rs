use super::*;

impl Game<'_> {
    /// The models of your world fluids (NeoForge's `RegisterFluidModelsEvent`): push a
    /// `horizon_mod_package::content::FluidModel` per fluid whose look doesn't say all of it. Its
    /// sprites are block atlas textures, baked at every resource load from that pack's files; the
    /// tint stays the look's. A fluid without one draws its look's sprite on every surface, or
    /// water's textures when it has none. Asked once at start, before the resources load, and
    /// only on the client.
    pub(in crate::game) fn native_register_fluid_models(
        context: &mod_api::RegistrationContext,
        models: &mut Vec<horizon_mod_package::content::FluidModel>,
    ) {
        Self::fire_register_fluid_models(context, models);
    }
}
