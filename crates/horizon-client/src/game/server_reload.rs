//! Prepare server listeners against the new packs before publishing any data.

#[cfg(test)]
#[path = "server_reload_tests.rs"]
mod tests;
use super::{Game, functions::PackStack};
use horizon_client::{
    datapack::DataStore,
    mod_api::{
        FileToIdConverter, Identifier, ReloadOrderError, ServerReloadApply, ServerReloadContext,
        ServerReloadError, ServerReloadListeners, VanillaServerListeners,
    },
};

#[derive(Debug)]
pub(super) enum ServerDataReloadError {
    Order(ReloadOrderError),
    Listener {
        listener: Identifier,
        source: ServerReloadError,
    },
}
impl std::fmt::Display for ServerDataReloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Order(error) => error.fmt(f),
            Self::Listener { listener, source } => {
                write!(f, "reload listener {listener}: {source}")
            }
        }
    }
}
impl std::error::Error for ServerDataReloadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::Order(error) => error,
            Self::Listener { source, .. } => source,
        })
    }
}

pub(super) enum PreparedReload {
    Functions(Box<DataStore>),
    Mod(ServerReloadApply),
}

impl PackStack {
    fn reload_context(
        &self,
        listener: &Identifier,
        converter: &FileToIdConverter,
    ) -> ServerReloadContext {
        let mut selected = std::collections::BTreeMap::new();
        for pack in self.builtin.iter().chain(self.enabled_packs()) {
            for file in pack.list_kinds(&[converter.directory()]) {
                if let Some(id) = file
                    .strip_prefix("data/")
                    .and_then(|path| converter.file_to_id(path))
                {
                    selected.insert(id, (pack, file));
                }
            }
        }
        let mut files = std::collections::BTreeMap::new();
        for (id, (pack, path)) in selected {
            // A file that vanishes or fails to read between listing and reading is one bad
            // entry, as vanilla's `scanDirectory` treats a parse failure: log it, skip it.
            match pack.read(&path) {
                Some(bytes) => {
                    files.insert(id, bytes);
                }
                None => log::error!(
                    "Couldn't read data file '{id}' from '{}/{}' for listener {}",
                    pack.id,
                    path,
                    listener
                ),
            }
        }
        ServerReloadContext::new(files)
    }
}

impl Game<'_> {
    pub(super) fn prepare_server_reload(
        &self,
        stack: &PackStack,
    ) -> Result<Vec<PreparedReload>, ServerDataReloadError> {
        let mut listeners = ServerReloadListeners::new();
        self.native_add_server_reload_listeners(&mut listeners);
        let listeners = listeners.finish().map_err(ServerDataReloadError::Order)?;
        let mut prepared = Vec::new();
        for listener in listeners {
            if let Some(converter) = &listener.converter {
                let context = stack.reload_context(&listener.id, converter);
                let apply = listener.listener.prepare(&context).map_err(|source| {
                    ServerDataReloadError::Listener {
                        listener: listener.id,
                        source,
                    }
                })?;
                prepared.push(PreparedReload::Mod(apply));
            } else if listener.id == VanillaServerListeners::functions() {
                let packs: Vec<_> = stack.builtin.iter().chain(stack.enabled_packs()).collect();
                let store = DataStore::load(&packs);
                for (id, e) in &store.errors {
                    log::warn!("Horizon: failed to load function {id}: {e}");
                }
                prepared.push(PreparedReload::Functions(Box::new(store)));
            }
        }
        Ok(prepared)
    }
    pub(super) fn apply_server_reload(&mut self, prepared: Vec<PreparedReload>, stack: &PackStack) {
        for listener in prepared {
            match listener {
                PreparedReload::Functions(store) => {
                    self.fns.store = *store;
                    self.sync_function_registries(stack);
                }
                PreparedReload::Mod(apply) => apply.apply(),
            }
        }
    }
}
