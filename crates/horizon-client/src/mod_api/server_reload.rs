//! Server data reload listeners and their registration context.
#[cfg(test)]
#[path = "server_reload_tests.rs"]
mod tests;

use super::{
    FileToIdConverter, Identifier, ReloadApply, ReloadResourceError,
    reload::{Boundary, ReloadFiles},
    reload_order::ReloadOrderError,
};
use std::{
    any::Any,
    collections::BTreeMap,
    marker::PhantomData,
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

/// Selected server data files for one converter, with pack overrides resolved.
/// Keys have their directory and extension removed. This cannot read assets or
/// filesystem paths. Its fields are private so the view can grow.
pub struct ServerReloadContext {
    files: ReloadFiles,
}
impl ServerReloadContext {
    pub(crate) fn new(files: BTreeMap<Identifier, Vec<u8>>) -> Self {
        Self {
            files: ReloadFiles::new(files),
        }
    }
    /// Converted IDs, across all namespaces, in deterministic resource-ID order.
    pub fn list(&self) -> impl Iterator<Item = &Identifier> {
        self.files.list()
    }
    /// The winning bytes for a converted ID, or None outside this directory.
    pub fn read(&self, id: &Identifier) -> Option<&[u8]> {
        self.files.read(id)
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
    fn prepare(&self, context: &ServerReloadContext) -> Result<ReloadApply, ReloadResourceError>;
}
impl ServerReloadListener for Boundary {
    fn prepare(&self, _: &ServerReloadContext) -> Result<ReloadApply, ReloadResourceError> {
        Ok(ReloadApply::new(|| {}))
    }
}

/// A read-only registry/graph view with registration methods, rebuilt on each
/// server reload. Listener names need not belong to the registering mod.
pub struct ServerReloadListeners {
    core: super::reload::ReloadCore<dyn ServerReloadListener>,
    retained: Vec<RetainedEntry>,
}
impl ServerReloadListeners {
    pub(crate) fn new() -> Self {
        Self {
            core: super::reload::ReloadCore::new(
                VanillaServerListeners::first(),
                VanillaServerListeners::functions(),
                VanillaServerListeners::last(),
                || -> Arc<dyn ServerReloadListener> { Arc::new(Boundary) },
            ),
            retained: Vec::new(),
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
        self.core.add_listener(id, converter, listener)
    }
    /// Register a listener whose object is also kept in the reload resources, as
    /// NeoForge's `addRetainedListener(key, listener)`: the caller builds the
    /// typed `ListenerKey` (`ListenerKey::new(id)`), and reads the object back
    /// through `ReloadResources` after the reload, from any server hook.
    pub fn add_retained_listener<T: ServerReloadListener>(
        &mut self,
        key: &ListenerKey<T>,
        converter: FileToIdConverter,
        listener: Arc<T>,
    ) -> Result<(), ReloadOrderError> {
        self.add_listener(key.id().clone(), converter, listener.clone())?;
        self.retained.push(RetainedEntry {
            id: key.id().clone(),
            any: listener,
        });
        Ok(())
    }
    /// Require `first` before `second`; both names must already be registered.
    pub fn add_dependency(
        &mut self,
        first: Identifier,
        second: Identifier,
    ) -> Result<(), ReloadOrderError> {
        self.core.add_dependency(first, second)
    }
    /// Names and listener objects in registration order, including boundaries.
    pub fn get_registry(&self) -> impl Iterator<Item = (&Identifier, &dyn ServerReloadListener)> {
        self.core
            .registry()
            .map(|(id, entry)| (id, entry.listener.as_ref()))
    }
    /// Find a listener's registered name, or None for an unregistered object.
    pub fn get_name_lookup(&self, listener: &dyn ServerReloadListener) -> Option<&Identifier> {
        self.core.find_name(listener)
    }
    /// Read-only dependency edges as `(first, second)` names.
    pub fn get_graph(&self) -> impl Iterator<Item = (&Identifier, &Identifier)> {
        self.core.graph()
    }
    /// The vanilla anchor preceding mod listeners by default.
    pub fn get_last_vanilla_listener(&self) -> &dyn ServerReloadListener {
        self.get_registry()
            .find(|(id, _)| **id == VanillaServerListeners::functions())
            .unwrap()
            .1
    }
    pub(crate) fn finish(self) -> Result<OrderedServerReload, ReloadOrderError> {
        Ok(OrderedServerReload {
            listeners: self.core.finish(&VanillaServerListeners::functions())?,
            retained: self.retained,
        })
    }
}

/// The ordered listeners and retained entries of one completed registration.
pub(crate) struct OrderedServerReload {
    pub(crate) listeners: Vec<super::reload::RegisteredListener<dyn ServerReloadListener>>,
    pub(crate) retained: Vec<RetainedEntry>,
}

/// One listener retained in the reload resources, as NeoForge's
/// `ReloadableServerResources` keeps a manager.
pub(crate) struct RetainedEntry {
    pub(crate) id: Identifier,
    pub(crate) any: Arc<dyn Any + Send + Sync>,
}

/// A typed key to a retained reload listener, built by the registering mod
/// and read back through `ReloadResources`. `ListenerKey::new` accepts any
/// id; a key whose id does not match a registration, or whose type differs
/// from the registered object, reads back as `None`.
pub struct ListenerKey<T: ?Sized> {
    id: Identifier,
    marker: PhantomData<fn() -> T>,
}
impl<T: ?Sized> ListenerKey<T> {
    pub fn new(id: Identifier) -> Self {
        Self {
            id,
            marker: PhantomData,
        }
    }
    /// The registered listener's name.
    pub fn id(&self) -> &Identifier {
        &self.id
    }
}
impl<T: ?Sized> Clone for ListenerKey<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            marker: PhantomData,
        }
    }
}
impl<T: ?Sized> std::fmt::Debug for ListenerKey<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListenerKey").field("id", &self.id).finish()
    }
}
impl<T: ?Sized> PartialEq for ListenerKey<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<T: ?Sized> Eq for ListenerKey<T> {}

/// The retained server reload listeners of the last successful reload, as
/// NeoForge's `ReloadableServerResources`, reached from
/// `ServerHost::reload_resources`. A new reload replaces the whole store.
#[derive(Default)]
pub struct ServerReloadResources {
    entries: BTreeMap<Identifier, Arc<dyn Any + Send + Sync>>,
}
impl ServerReloadResources {
    pub(crate) fn install(&mut self, retained: Vec<RetainedEntry>) {
        self.entries = retained
            .into_iter()
            .map(|entry| (entry.id, entry.any))
            .collect();
    }
    /// The retained listener registered under `key` in the last successful
    /// reload; `None` when nothing is registered under it, its type differs, or
    /// no reload has succeeded yet. The downcast itself checks the type.
    pub fn listener<T: ServerReloadListener>(&self, key: &ListenerKey<T>) -> Option<Arc<T>> {
        self.entries.get(key.id())?.clone().downcast::<T>().ok()
    }
}
