//! Prepare server listeners against the new packs before publishing any data.

#[cfg(test)]
#[path = "server_reload_tests.rs"]
mod tests;
use super::{Game, functions::PackStack};
pub(super) use horizon_client::mod_api::reload::ReloadStageError;
use horizon_client::{
    datapack::DataStore,
    mod_api::{
        FileToIdConverter, Identifier, ServerReloadContext, ServerReloadListener,
        ServerReloadListeners, ServerReloadResources, VanillaServerListeners,
        reload::{apply_listeners, prepare_listeners, reload_files},
    },
};

pub(super) type PreparedReload = PreparedStep<Box<DataStore>>;

use horizon_client::mod_api::reload::PreparedStep;

/// Everything one successful reload publishes: its apply steps, then the
/// retained listeners' store.
pub(super) struct PreparedServerReload {
    pub(super) steps: Vec<PreparedReload>,
    pub(super) resources: ServerReloadResources,
}

/// Loads the vanilla function library from `stack`, logging per-file failures.
fn functions_store(stack: &PackStack) -> DataStore {
    let packs: Vec<_> = stack.builtin.iter().chain(stack.enabled_packs()).collect();
    let store = DataStore::load(&packs);
    for (id, e) in &store.errors {
        log::warn!("Horizon: failed to load function {id}: {e}");
    }
    store
}

impl PackStack {
    fn reload_context(
        &self,
        listener: &Identifier,
        converter: &FileToIdConverter,
    ) -> ServerReloadContext {
        let packs: Vec<_> = self.builtin.iter().chain(self.enabled_packs()).collect();
        ServerReloadContext::new(reload_files("data", &packs, listener, converter))
    }
}

impl Game<'_> {
    pub(super) fn prepare_server_reload(
        &self,
        stack: &PackStack,
    ) -> Result<PreparedServerReload, ReloadStageError> {
        // Nobody listens: no registration graph, only the vanilla function library.
        if !crate::mod_api::listened(crate::mod_api::HookId::AddServerReloadListeners) {
            return Ok(PreparedServerReload {
                steps: vec![PreparedStep::Vanilla(Box::new(functions_store(stack)))],
                resources: ServerReloadResources::default(),
            });
        }
        let mut listeners = ServerReloadListeners::new();
        self.native_add_server_reload_listeners(&mut listeners);
        let mut resources = ServerReloadResources::default();
        let ordered = listeners.finish().map_err(ReloadStageError::Order)?;
        resources.install(ordered.retained);
        let steps = prepare_listeners(
            ordered.listeners,
            "server_reload_prepare",
            |listener| {
                (listener.id == VanillaServerListeners::functions())
                    .then(|| Box::new(functions_store(stack)))
            },
            |id, converter| stack.reload_context(id, converter),
            ServerReloadListener::prepare,
        )?;
        Ok(PreparedServerReload { steps, resources })
    }
    pub(super) fn apply_server_reload(
        &mut self,
        prepared: PreparedServerReload,
        stack: &PackStack,
    ) {
        apply_listeners(prepared.steps, "server_reload_apply", |store| {
            self.fns.store = *store;
            self.sync_function_registries(stack);
        });
        self.reload_resources = prepared.resources;
    }
    /// The retained server reload listeners of the last successful reload.
    pub(super) fn native_reload_resources(&self) -> &ServerReloadResources {
        &self.reload_resources
    }
}
