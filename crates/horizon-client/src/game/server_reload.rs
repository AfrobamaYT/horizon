//! Prepare server listeners against the new packs before publishing any data.

#[cfg(test)]
#[path = "server_reload_tests.rs"]
mod tests;
use super::{Game, functions::PackStack};
use horizon_client::{
    datapack::DataStore,
    mod_api::{
        Identifier, ReloadOrderError, ServerReloadApply, ServerReloadContext, ServerReloadError,
        ServerReloadListeners, VanillaServerListeners,
    },
};

#[derive(Debug)]
pub(super) enum ServerDataReloadError {
    Order(ReloadOrderError),
    Listener {
        owner: String,
        listener: String,
        source: ServerReloadError,
    },
}
impl std::fmt::Display for ServerDataReloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Order(error) => error.fmt(f),
            Self::Listener {
                owner,
                listener,
                source,
            } => write!(f, "mod {owner}: reload listener {listener}: {source}"),
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
        prefix: &Identifier,
    ) -> Result<ServerReloadContext, ServerReloadError> {
        let namespace_path = format!("data/{}/", prefix.namespace());
        let path = format!("{namespace_path}{}", prefix.path());
        let mut selected = std::collections::BTreeMap::new();
        for pack in self.builtin.iter().chain(self.enabled_packs()) {
            for file in pack.list(&path) {
                let resource_path = &file[namespace_path.len()..];
                // Zip entry names are untrusted, unlike the registered prefix.
                if resource_path
                    .split('/')
                    .any(|p| p.is_empty() || p == "." || p == "..")
                {
                    continue;
                }
                if let Ok(id) = Identifier::from_parts(prefix.namespace(), resource_path) {
                    selected.insert(id, (pack, file));
                }
            }
        }
        let mut files = std::collections::BTreeMap::new();
        for (id, (pack, path)) in selected {
            let bytes = pack.read(&path).ok_or_else(|| {
                ServerReloadError::new(
                    id.clone(),
                    std::io::Error::other("could not read selected data resource"),
                )
            })?;
            files.insert(id, bytes);
        }
        Ok(ServerReloadContext::new(files))
    }
}

impl Game<'_> {
    pub(super) fn prepare_server_reload(
        &self,
        stack: &PackStack,
    ) -> Result<Vec<PreparedReload>, ServerDataReloadError> {
        let mut listeners = ServerReloadListeners::new();
        Self::native_add_server_reload_listeners(&mut listeners);
        let listeners = listeners.finish().map_err(ServerDataReloadError::Order)?;
        let mut prepared = Vec::new();
        for listener in listeners {
            if let Some(prefix) = &listener.prefix {
                let apply = stack
                    .reload_context(prefix)
                    .and_then(|context| listener.listener.prepare(&context))
                    .map_err(|source| ServerDataReloadError::Listener {
                        owner: listener.owner,
                        listener: listener.id,
                        source,
                    })?;
                prepared.push(PreparedReload::Mod(apply));
            } else if listener.id == VanillaServerListeners::FUNCTIONS {
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
