//! Prepare client listeners against the staged resources before publishing any.

#[cfg(test)]
#[path = "client_reload_tests.rs"]
mod tests;

use super::Game;
use horizon_client::{
    datapack::DataPack,
    mod_api::{
        ClientReloadContext, ClientReloadListeners, FileToIdConverter, Identifier, ReloadApply,
        ReloadOrderError, ReloadResourceError, reload::ReloadMark,
    },
};
use std::collections::BTreeMap;

#[derive(Debug)]
pub(super) enum ClientAssetReloadError {
    Order(ReloadOrderError),
    Listener {
        listener: Identifier,
        source: ReloadResourceError,
    },
}
impl std::fmt::Display for ClientAssetReloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Order(error) => error.fmt(f),
            Self::Listener { listener, source } => {
                write!(f, "reload listener {listener}: {source}")
            }
        }
    }
}
impl std::error::Error for ClientAssetReloadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::Order(error) => error,
            Self::Listener { source, .. } => source,
        })
    }
}

pub(super) struct PreparedClientReload {
    apply: ReloadApply,
    owner: Option<&'static str>,
    id: Identifier,
}

/// One listener's asset view from the merged resource root of the coming
/// reload, where pack overrides are already resolved.
fn reload_context(
    pack: &DataPack,
    listener: &Identifier,
    converter: &FileToIdConverter,
) -> ClientReloadContext {
    let mut files = BTreeMap::new();
    for file in pack.list_under("assets", &[converter.directory()]) {
        if let Some(id) = file
            .strip_prefix("assets/")
            .and_then(|path| converter.file_to_id(path))
        {
            // A file that vanishes or fails to read between listing and reading is one bad
            // entry, as vanilla's `scanDirectory` treats a parse failure: log it, skip it.
            match pack.read(&file) {
                Some(bytes) => {
                    files.insert(id, bytes);
                }
                None => log::error!(
                    "Couldn't read asset file '{id}' from '{}/{}' for listener {}",
                    pack.id,
                    file,
                    listener
                ),
            }
        }
    }
    ClientReloadContext::new(files)
}

impl Game<'_> {
    /// The registered listeners for the coming resource reload: empty when no
    /// mod subscribes, so an ordinary reload builds no graph and reads no
    /// files. The registrations are collected here, on the thread that owns
    /// the game, so each listener keeps its registering mod for crash
    /// attribution; the reload's worker orders and prepares them.
    pub(super) fn register_client_reload_listeners(&self) -> ClientReloadListeners {
        let mut listeners = ClientReloadListeners::new();
        if crate::mod_api::listened(crate::mod_api::HookId::AddClientReloadListeners) {
            self.native_add_client_reload_listeners(&mut listeners);
        }
        listeners
    }
}

/// Orders the registered listeners, then prepares each against `root`, the
/// merged resource root the reload will publish; runs on the reload's worker,
/// off the main thread.
pub(super) fn stage_client_reload(
    root: &std::path::Path,
    listeners: ClientReloadListeners,
) -> Result<Vec<PreparedClientReload>, ClientAssetReloadError> {
    let pack = DataPack::builtin(root);
    let mut prepared = Vec::new();
    for listener in listeners.finish().map_err(ClientAssetReloadError::Order)? {
        // The vanilla anchor and boundaries have no converter and no work.
        let Some(converter) = &listener.converter else {
            continue;
        };
        let context = reload_context(&pack, &listener.id, converter);
        let _mark = ReloadMark::enter(listener.owner, &listener.id, "client_reload_prepare");
        let apply = listener.listener.prepare(&context).map_err(|source| {
            ClientAssetReloadError::Listener {
                listener: listener.id.clone(),
                source,
            }
        })?;
        prepared.push(PreparedClientReload {
            apply,
            owner: listener.owner,
            id: listener.id,
        });
    }
    Ok(prepared)
}

/// Applies the prepared listeners on the main thread, after the rest of the
/// staged pack is published and before the resources-reloaded notice.
pub(super) fn apply_client_reload(prepared: Vec<PreparedClientReload>) {
    for listener in prepared {
        let _mark = ReloadMark::enter(listener.owner, &listener.id, "client_reload_apply");
        listener.apply.apply();
    }
}
