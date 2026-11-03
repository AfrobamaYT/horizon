//! Server data reload listeners and their registration context.
#[cfg(test)]
#[path = "server_reload_tests.rs"]
mod tests;

use super::{
    FileToIdConverter, Identifier,
    reload_order::{ReloadOrder, ReloadOrderError},
};
use std::{
    collections::BTreeMap,
    error::Error,
    sync::{Arc, LazyLock},
};

/// Horizon's vanilla server reload listeners and ordering boundaries.
pub struct VanillaServerListeners;
impl VanillaServerListeners {
    /// Boundary before every reload listener.
    pub fn first() -> Identifier {
        static ID: LazyLock<Identifier> =
            LazyLock::new(|| Identifier::parse("minecraft:first").unwrap());
        ID.clone()
    }
    /// Vanilla's function library. Use this anchor rather than its string value,
    /// which need not match NeoForge's identifier.
    pub fn functions() -> Identifier {
        static ID: LazyLock<Identifier> =
            LazyLock::new(|| Identifier::parse("minecraft:function_library").unwrap());
        ID.clone()
    }
    /// Boundary after every reload listener.
    pub fn last() -> Identifier {
        static ID: LazyLock<Identifier> =
            LazyLock::new(|| Identifier::parse("minecraft:last").unwrap());
        ID.clone()
    }
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

/// Selected server data files for one converter, with pack overrides resolved.
/// Keys have their directory and extension removed. This cannot read assets or
/// filesystem paths. Its fields are private so the view can grow.
pub struct ServerReloadContext {
    files: BTreeMap<Identifier, Vec<u8>>,
}
impl ServerReloadContext {
    pub(crate) fn new(files: BTreeMap<Identifier, Vec<u8>>) -> Self {
        Self { files }
    }
    /// Converted IDs, across all namespaces, in deterministic resource-ID order.
    pub fn list(&self) -> impl Iterator<Item = &Identifier> {
        self.files.keys()
    }
    /// The winning bytes for a converted ID, or None outside this directory.
    pub fn read(&self, id: &Identifier) -> Option<&[u8]> {
        self.files.get(id).map(Vec::as_slice)
    }
}

/// Infallible publication, run only after every listener prepared successfully.
/// One closure is allocated per listener per reload, never per tick.
pub struct ServerReloadApply(Box<dyn FnOnce() + Send>);
impl ServerReloadApply {
    /// Capture newly parsed state without publishing it yet.
    pub fn new(apply: impl FnOnce() + Send + 'static) -> Self {
        Self(Box::new(apply))
    }
    pub(crate) fn apply(self) {
        (self.0)();
    }
}

/// A server-side `SimplePreparableReloadListener`: prepare reads the new pack
/// view without changing live state; its returned apply publishes the result.
/// All prepares finish before any apply, including vanilla's, runs. Prepare
/// cannot see another listener's new published state. Dependencies order both
/// phases deterministically; they do not remove the barrier. On any error all
/// pending applies are dropped, keeping the previous data and function library.
pub trait ServerReloadListener: Send + Sync + 'static {
    /// Parse and validate new data without changing the listener's live state.
    ///
    /// As vanilla's `SimpleJsonResourceReloadListener.scanDirectory`, a
    /// listener that parses its files logs and skips one that won't parse,
    /// carrying the rest through; returning `Err` aborts the whole reload and
    /// keeps every listener's previous data. A panic in `prepare` or the
    /// returned apply is not recovered: it stops the server, and the crash
    /// report names the listener and the mod that registered it.
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
    pub id: Identifier,
    pub converter: Option<FileToIdConverter>,
    pub listener: Arc<dyn ServerReloadListener>,
    /// The registering mod's id, for crash attribution on its later calls; the
    /// vanilla anchors have none.
    pub owner: Option<&'static str>,
}

/// Crash attribution for one listener call. A returned `Err` is recovered by the
/// reload; a panic in `prepare` or `apply` is fatal, and the report names the
/// registering mod (through the native-mod mark) and the listener (through the
/// report's details).
pub(crate) struct ReloadMark {
    _guard: Option<crate::crash::NativeModGuard>,
}

impl ReloadMark {
    pub(crate) fn enter(
        owner: Option<&'static str>,
        listener: &Identifier,
        phase: &'static str,
    ) -> Self {
        let _guard = owner.map(|id| {
            crate::crash::enter_native_mod(crate::crash::NativeModCall {
                id,
                hook: phase,
                part: None,
            })
        });
        crate::crash::set_detail("Reload listener", listener.to_string());
        Self { _guard }
    }
}

impl Drop for ReloadMark {
    fn drop(&mut self) {
        crate::crash::clear_detail("Reload listener");
    }
}

/// A read-only registry/graph view with registration methods, rebuilt on each
/// server reload. Listener names need not belong to the registering mod.
pub struct ServerReloadListeners {
    order: ReloadOrder<ServerListener>,
    failure: Option<ReloadOrderError>,
}
impl ServerReloadListeners {
    pub(crate) fn new() -> Self {
        let mut order = ReloadOrder::default();
        for id in [
            VanillaServerListeners::first(),
            VanillaServerListeners::functions(),
            VanillaServerListeners::last(),
        ] {
            order
                .add_listener(
                    id.clone(),
                    ServerListener {
                        id,
                        converter: None,
                        listener: Arc::new(Boundary),
                        owner: None,
                    },
                )
                .unwrap();
        }
        order
            .add_dependency(
                VanillaServerListeners::first(),
                VanillaServerListeners::functions(),
            )
            .unwrap();
        order
            .add_dependency(
                VanillaServerListeners::functions(),
                VanillaServerListeners::last(),
            )
            .unwrap();
        Self {
            order,
            failure: None,
        }
    }
    /// Register a unique listener and converter, e.g. `FileToIdConverter::json("machines")`.
    /// Retain the Arc to read the listener's published state outside reloads.
    pub fn add_listener(
        &mut self,
        id: Identifier,
        converter: FileToIdConverter,
        listener: Arc<impl ServerReloadListener>,
    ) -> Result<(), ReloadOrderError> {
        let owner = crate::crash::current_native_mod();
        let result = self.insert(id, converter, listener, owner);
        if let Err(error) = &result {
            self.failure.get_or_insert_with(|| error.clone());
        }
        result
    }
    fn insert(
        &mut self,
        id: Identifier,
        converter: FileToIdConverter,
        listener: Arc<dyn ServerReloadListener>,
        owner: Option<&'static str>,
    ) -> Result<(), ReloadOrderError> {
        if let Some(registered_as) = self.get_name_lookup(listener.as_ref()) {
            return Err(ReloadOrderError::DuplicateObject {
                id,
                registered_as: registered_as.clone(),
            });
        }
        self.order.add_listener(
            id.clone(),
            ServerListener {
                id: id.clone(),
                converter: Some(converter),
                listener,
                owner,
            },
        )?;
        self.order
            .add_dependency(VanillaServerListeners::first(), id.clone())
            .unwrap();
        self.order
            .add_dependency(id, VanillaServerListeners::last())
            .unwrap();
        Ok(())
    }
    /// Require `first` before `second`; both names must already be registered.
    pub fn add_dependency(
        &mut self,
        first: Identifier,
        second: Identifier,
    ) -> Result<(), ReloadOrderError> {
        let result = self.order.add_dependency(first, second);
        if let Err(error) = &result {
            self.failure.get_or_insert_with(|| error.clone());
        }
        result
    }
    /// Names and listener objects in registration order, including boundaries.
    pub fn get_registry(&self) -> impl Iterator<Item = (&Identifier, &dyn ServerReloadListener)> {
        self.order
            .registry()
            .map(|(id, entry)| (id, entry.listener.as_ref()))
    }
    /// Find a listener's registered name, or None for an unregistered object.
    pub fn get_name_lookup(&self, listener: &dyn ServerReloadListener) -> Option<&Identifier> {
        self.get_registry()
            .find_map(|(id, candidate)| std::ptr::addr_eq(candidate, listener).then_some(id))
    }
    /// Read-only dependency edges as `(first, second)` names.
    pub fn get_graph(&self) -> impl Iterator<Item = (&Identifier, &Identifier)> {
        self.order.graph()
    }
    /// The vanilla anchor preceding mod listeners by default.
    pub fn get_last_vanilla_listener(&self) -> &dyn ServerReloadListener {
        self.get_registry()
            .find(|(id, _)| **id == VanillaServerListeners::functions())
            .unwrap()
            .1
    }
    pub(crate) fn finish(self) -> Result<Vec<ServerListener>, ReloadOrderError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        self.order.sort(Some(&VanillaServerListeners::functions()))
    }
}
