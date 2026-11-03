//! Server data reload listeners and their registration context.

#[cfg(test)]
#[path = "server_reload_tests.rs"]
mod tests;

use super::reload_order::{ReloadOrder, ReloadOrderError};
use horizon_world::identifier::Identifier;
use std::{collections::BTreeMap, error::Error, sync::Arc};

/// Names of Horizon's vanilla server reload listeners and ordering boundaries.
pub struct VanillaServerListeners;
impl VanillaServerListeners {
    /// Boundary before every reload listener.
    pub const FIRST: &'static str = "minecraft:first";
    /// Vanilla's server function library. Use this constant rather than its
    /// string value, which need not match NeoForge's identifier.
    pub const FUNCTIONS: &'static str = "minecraft:function_library";
    /// Boundary after every reload listener.
    pub const LAST: &'static str = "minecraft:last";
}

/// An error loading a resource, including the resource that caused it.
#[derive(Debug)]
pub struct ServerReloadError {
    resource: Identifier,
    source: Box<dyn Error + Send + Sync>,
}
impl ServerReloadError {
    /// Attach the resource ID to a parsing or validation error.
    pub fn new(resource: Identifier, source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            resource,
            source: Box::new(source),
        }
    }
}
impl std::fmt::Display for ServerReloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.resource, self.source)
    }
}
impl Error for ServerReloadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// The selected data files under one listener's prefix, with pack overrides
/// resolved. This context cannot read client assets or files outside the prefix.
pub struct ServerReloadContext {
    files: BTreeMap<Identifier, Vec<u8>>,
}
impl ServerReloadContext {
    pub(crate) fn new(files: BTreeMap<Identifier, Vec<u8>>) -> Self {
        Self { files }
    }

    /// IDs under the registered prefix, in deterministic resource-ID order.
    pub fn list(&self) -> impl Iterator<Item = &Identifier> {
        self.files.keys()
    }

    /// The highest-priority selected version of an ID in the prefix.
    pub fn read(&self, id: &Identifier) -> Option<&[u8]> {
        self.files.get(id).map(Vec::as_slice)
    }
}

/// A prepared reload's infallible publication, run only after every listener
/// prepared successfully. One closure is allocated per listener per reload.
pub struct ServerReloadApply(Box<dyn FnOnce() + Send>);
impl ServerReloadApply {
    /// Capture the newly parsed state, without publishing it yet.
    pub fn new(apply: impl FnOnce() + Send + 'static) -> Self {
        Self(Box::new(apply))
    }

    pub(crate) fn apply(self) {
        (self.0)();
    }
}

/// A server-side `SimplePreparableReloadListener`: prepare reads the new pack
/// view without changing live state, then apply publishes its prepared result.
///
/// All prepares finish before any apply runs. A prepare therefore cannot read
/// another listener's new published state. Dependencies order both phases for
/// determinism; they do not remove that barrier. On any prepare error all pending
/// applies are dropped and the previous data, including vanilla functions, stays.
pub trait ServerReloadListener: Send + Sync + 'static {
    /// Parse and validate new data without changing the listener's live state.
    fn prepare(
        &self,
        context: &ServerReloadContext,
    ) -> Result<ServerReloadApply, ServerReloadError>;
}

struct Boundary;
impl ServerReloadListener for Boundary {
    fn prepare(&self, _: &ServerReloadContext) -> Result<ServerReloadApply, ServerReloadError> {
        Ok(ServerReloadApply::new(|| {}))
    }
}

pub(crate) struct ServerListener {
    pub id: String,
    pub owner: String,
    pub prefix: Option<Identifier>,
    pub listener: Arc<dyn ServerReloadListener>,
}

/// A listener's graph and registry, built on each server-data reload.
pub struct ServerReloadListeners {
    order: ReloadOrder<ServerListener>,
    owner: &'static str,
    failure: Option<ReloadOrderError>,
}
impl ServerReloadListeners {
    pub(crate) fn new() -> Self {
        let mut order = ReloadOrder::default();
        for id in [
            VanillaServerListeners::FIRST,
            VanillaServerListeners::FUNCTIONS,
            VanillaServerListeners::LAST,
        ] {
            order
                .add_listener(
                    id.into(),
                    "minecraft".into(),
                    ServerListener {
                        id: id.into(),
                        owner: "minecraft".into(),
                        prefix: None,
                        listener: Arc::new(Boundary),
                    },
                )
                .unwrap();
        }
        order
            .add_dependency(
                "minecraft",
                VanillaServerListeners::FIRST,
                VanillaServerListeners::FUNCTIONS,
            )
            .unwrap();
        order
            .add_dependency(
                "minecraft",
                VanillaServerListeners::FUNCTIONS,
                VanillaServerListeners::LAST,
            )
            .unwrap();
        Self {
            order,
            owner: "minecraft",
            failure: None,
        }
    }

    pub(crate) fn set_owner(&mut self, owner: &'static str) {
        self.owner = owner;
    }

    /// Register a named listener for data under `prefix`, such as `my_mod:machines/`.
    /// The prefix must name a directory. Any namespace is allowed; no filesystem
    /// traversal is allowed. Retain the Arc to access the listener's published data.
    pub fn add_listener(
        &mut self,
        id: Identifier,
        prefix: Identifier,
        listener: Arc<impl ServerReloadListener>,
    ) -> Result<(), ReloadOrderError> {
        let result = self.insert(id, prefix, listener);
        if let Err(error) = &result {
            self.failure.get_or_insert_with(|| error.clone());
        }
        result
    }

    fn insert(
        &mut self,
        id: Identifier,
        prefix: Identifier,
        listener: Arc<dyn ServerReloadListener>,
    ) -> Result<(), ReloadOrderError> {
        let safe_segment = |s: &str| !s.is_empty() && s != "." && s != "..";
        if !safe_segment(prefix.namespace())
            || !prefix.path().ends_with('/')
            || !prefix
                .path()
                .strip_suffix('/')
                .unwrap_or(prefix.path())
                .split('/')
                .all(safe_segment)
        {
            return Err(ReloadOrderError::InvalidPrefix {
                prefix: prefix.to_string(),
                owner: self.owner.into(),
            });
        }
        let id = id.to_string();
        let owner = self.owner.to_owned();
        self.order.add_listener(
            id.clone(),
            owner.clone(),
            ServerListener {
                id: id.clone(),
                owner,
                prefix: Some(prefix),
                listener,
            },
        )?;
        self.order
            .add_dependency(self.owner, VanillaServerListeners::FIRST, &id)
            .unwrap();
        self.order
            .add_dependency(self.owner, &id, VanillaServerListeners::LAST)
            .unwrap();
        Ok(())
    }

    /// Require `first` to finish before `second`. Both must already exist.
    pub fn add_dependency(&mut self, first: &str, second: &str) -> Result<(), ReloadOrderError> {
        let result = self.order.add_dependency(self.owner, first, second);
        if let Err(error) = &result {
            self.failure.get_or_insert_with(|| error.clone());
        }
        result
    }

    /// Names and listener objects in registration order, including vanilla boundaries.
    pub fn get_registry(&self) -> impl Iterator<Item = (&str, &dyn ServerReloadListener)> {
        self.order
            .registry()
            .map(|(id, entry)| (id, entry.listener.as_ref()))
    }

    /// Find the registered name of a listener object, or None if it is unregistered.
    pub fn get_name_lookup(&self, listener: &dyn ServerReloadListener) -> Option<&str> {
        self.get_registry()
            .find_map(|(id, candidate)| std::ptr::eq(candidate, listener).then_some(id))
    }

    /// Read-only dependency edges, as `(first, second)` names.
    pub fn get_graph(&self) -> impl Iterator<Item = (&str, &str)> {
        self.order.graph()
    }

    /// The final vanilla listener, whose default position precedes mod listeners.
    pub fn get_last_vanilla_listener(&self) -> &dyn ServerReloadListener {
        self.get_registry()
            .find(|(id, _)| *id == VanillaServerListeners::FUNCTIONS)
            .unwrap()
            .1
    }

    pub(crate) fn finish(self) -> Result<Vec<ServerListener>, ReloadOrderError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        self.order.sort(Some(VanillaServerListeners::FUNCTIONS))
    }
}
