//! The side-neutral machinery of reload listeners: the error and apply types,
//! crash attribution, the shared prepare and apply loops, and the registration
//! core both sides build on. The per-side `server_reload` and `client_reload`
//! hold only their context, listener trait and vanilla anchors.
use super::{
    FileToIdConverter, Identifier,
    reload_order::{ReloadOrder, ReloadOrderError},
};
use crate::datapack::DataPack;
use std::{collections::BTreeMap, sync::Arc};

/// An error loading a resource, including the resource that caused it.
#[derive(Debug)]
pub struct ReloadResourceError {
    resource: Identifier,
    source: Box<dyn std::error::Error + Send + Sync>,
}
impl ReloadResourceError {
    /// Attach the resource ID to a parsing or validation error.
    pub fn new(
        resource: Identifier,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            resource,
            source: Box::new(source),
        }
    }
}
impl std::fmt::Display for ReloadResourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.resource, self.source)
    }
}
impl std::error::Error for ReloadResourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// Infallible publication, run only after every listener prepared successfully.
/// One closure is allocated per listener per reload, never per tick.
pub struct ReloadApply(Box<dyn FnOnce() + Send>);
impl ReloadApply {
    /// Capture newly parsed state without publishing it yet.
    pub fn new(apply: impl FnOnce() + Send + 'static) -> Self {
        Self(Box::new(apply))
    }
    pub(crate) fn apply(self) {
        (self.0)();
    }
}

/// The stateless boundary listener each side's anchors are built from; each
/// side's file implements its listener trait for it.
pub(crate) struct Boundary;

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

/// One registered listener of either side, between registration and ordering.
pub(crate) struct RegisteredListener<L: ?Sized> {
    pub(crate) id: Identifier,
    pub(crate) converter: Option<FileToIdConverter>,
    pub(crate) listener: Arc<L>,
    /// The registering mod's id, for crash attribution on its later calls; the
    /// vanilla anchors have none.
    pub(crate) owner: Option<&'static str>,
}

/// Registration both sides share: unique names and objects, dependencies, the
/// first/anchor/last boundaries, failure recording, and the ordering sort. A
/// side adds what its moment needs (the server retains listeners) around it.
pub(crate) struct ReloadCore<L: ?Sized> {
    order: ReloadOrder<RegisteredListener<L>>,
    failure: Option<ReloadOrderError>,
    first: Identifier,
    last: Identifier,
}

impl<L: ?Sized + 'static> ReloadCore<L> {
    /// The `first`, vanilla-anchor and `last` boundaries with their default
    /// edges; `boundary` builds each id's own stateless listener, as the
    /// vanilla anchors are distinct listener objects.
    pub(crate) fn new(
        first: Identifier,
        anchor: Identifier,
        last: Identifier,
        boundary: impl Fn() -> Arc<L>,
    ) -> Self {
        let mut order = ReloadOrder::default();
        for id in [first.clone(), anchor.clone(), last.clone()] {
            order
                .add_listener(
                    id.clone(),
                    RegisteredListener {
                        id,
                        converter: None,
                        listener: boundary(),
                        owner: None,
                    },
                )
                .unwrap();
        }
        let mut core = Self {
            order,
            failure: None,
            first,
            last,
        };
        // The anchor keeps its place between the boundaries even though no
        // listener ever orders against it explicitly.
        core.order
            .add_dependency(core.first.clone(), anchor.clone())
            .unwrap();
        core.order
            .add_dependency(anchor, core.last.clone())
            .unwrap();
        core
    }
    /// Register a unique listener and converter, e.g.
    /// `FileToIdConverter::json("machines")`.
    pub(crate) fn add_listener(
        &mut self,
        id: Identifier,
        converter: FileToIdConverter,
        listener: Arc<L>,
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
        listener: Arc<L>,
        owner: Option<&'static str>,
    ) -> Result<(), ReloadOrderError> {
        if let Some(registered_as) = self.find_name(&*listener) {
            return Err(ReloadOrderError::DuplicateObject {
                id,
                registered_as: registered_as.clone(),
            });
        }
        self.order.add_listener(
            id.clone(),
            RegisteredListener {
                id: id.clone(),
                converter: Some(converter),
                listener,
                owner,
            },
        )?;
        self.order
            .add_dependency(self.first.clone(), id.clone())
            .unwrap();
        self.order.add_dependency(id, self.last.clone()).unwrap();
        Ok(())
    }
    /// Require `first` before `second`; both names must already be registered.
    pub(crate) fn add_dependency(
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
    /// Registration order, boundaries included.
    pub(crate) fn registry(&self) -> impl Iterator<Item = (&Identifier, &RegisteredListener<L>)> {
        self.order.registry()
    }
    /// Read-only dependency edges as `(first, second)` names.
    pub(crate) fn graph(&self) -> impl Iterator<Item = (&Identifier, &Identifier)> {
        self.order.graph()
    }
    /// Find a listener object's registered name, or None for an unregistered
    /// object.
    pub(crate) fn find_name(&self, listener: &L) -> Option<&Identifier> {
        self.registry().find_map(|(id, entry)| {
            std::ptr::addr_eq(entry.listener.as_ref(), listener).then_some(id)
        })
    }
    /// The ordered listeners, or the first registration failure; `anchor` is
    /// the vanilla listener unrelated mods sort after.
    pub(crate) fn finish(
        self,
        anchor: &Identifier,
    ) -> Result<Vec<RegisteredListener<L>>, ReloadOrderError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        self.order.sort(Some(anchor))
    }
}

/// A failed reload of one side's listeners: the ordering failed, or a
/// listener's `prepare` returned an error. Both sides share this form, so the
/// reload paths report listeners alike.
#[derive(Debug)]
pub(crate) enum ReloadStageError {
    Order(ReloadOrderError),
    Listener {
        listener: Identifier,
        source: ReloadResourceError,
    },
}
impl std::fmt::Display for ReloadStageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Order(error) => error.fmt(f),
            Self::Listener { listener, source } => {
                write!(f, "reload listener {listener}: {source}")
            }
        }
    }
}
impl std::error::Error for ReloadStageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::Order(error) => error,
            Self::Listener { source, .. } => source,
        })
    }
}

/// One mod listener's prepared step, between its `prepare` and its `apply`.
/// The server's vanilla function library rides along as the `Vanilla` step of
/// its own reload; the client's has none.
pub(crate) enum PreparedStep<V = ()> {
    Mod {
        apply: ReloadApply,
        owner: Option<&'static str>,
        id: Identifier,
    },
    Vanilla(V),
}

/// Orders the registered listeners, then prepares each with a converter
/// against the reload's own view: the shared loop of both sides' reloads.
/// `context_of` builds the side's context for one listener from its converter,
/// `prepare_of` calls the side's listener trait, and `vanilla` turns a
/// converter-less vanilla anchor into its own step (the server's function
/// library); the boundaries prepare nothing. Runs wherever the side stages its
/// reload, off the thread that owns the game.
pub(crate) fn prepare_listeners<L: ?Sized, C, V>(
    ordered: Vec<RegisteredListener<L>>,
    phase: &'static str,
    vanilla: impl Fn(&RegisteredListener<L>) -> Option<V>,
    context_of: impl Fn(&Identifier, &FileToIdConverter) -> C,
    prepare_of: impl Fn(&L, &C) -> Result<ReloadApply, ReloadResourceError>,
) -> Result<Vec<PreparedStep<V>>, ReloadStageError> {
    let mut prepared = Vec::new();
    for listener in ordered {
        if let Some(step) = vanilla(&listener) {
            prepared.push(PreparedStep::Vanilla(step));
            continue;
        }
        // The boundaries have no converter and no work.
        let Some(converter) = &listener.converter else {
            continue;
        };
        let context = context_of(&listener.id, converter);
        let _mark = ReloadMark::enter(listener.owner, &listener.id, phase);
        let apply = prepare_of(&*listener.listener, &context).map_err(|source| {
            ReloadStageError::Listener {
                listener: listener.id.clone(),
                source,
            }
        })?;
        prepared.push(PreparedStep::Mod {
            apply,
            owner: listener.owner,
            id: listener.id,
        });
    }
    Ok(prepared)
}

/// Applies the prepared steps in order on the thread that owns the game,
/// attributing each mod listener's apply to its registering mod. `vanilla`
/// publishes the side's own step.
pub(crate) fn apply_listeners<V>(
    prepared: Vec<PreparedStep<V>>,
    phase: &'static str,
    mut vanilla: impl FnMut(V),
) {
    for step in prepared {
        match step {
            PreparedStep::Mod { apply, owner, id } => {
                let _mark = ReloadMark::enter(owner, &id, phase);
                apply.apply();
            }
            PreparedStep::Vanilla(step) => vanilla(step),
        }
    }
}

/// The files one converter selected from a reload's packs, shared by both
/// sides' contexts; each side keeps its own context type so a wrong-side read
/// does not compile.
pub(crate) struct ReloadFiles(BTreeMap<Identifier, Vec<u8>>);
impl ReloadFiles {
    pub(crate) fn new(files: BTreeMap<Identifier, Vec<u8>>) -> Self {
        Self(files)
    }
    /// Converted IDs, across all namespaces, in deterministic resource-ID order.
    pub(crate) fn list(&self) -> impl Iterator<Item = &Identifier> {
        self.0.keys()
    }
    /// The winning bytes for a converted ID, or None outside this directory.
    pub(crate) fn read(&self, id: &Identifier) -> Option<&[u8]> {
        self.0.get(id).map(Vec::as_slice)
    }
}

/// Reads one converter's directory from `packs`, later packs winning, and
/// collects the converted files: the shared read-and-log loop of both sides'
/// reloads. `top` is `"data"` or `"assets"`.
pub(crate) fn reload_files(
    top: &str,
    packs: &[&DataPack],
    listener: &Identifier,
    converter: &FileToIdConverter,
) -> ReloadFiles {
    let mut selected = BTreeMap::new();
    for pack in packs {
        for file in pack.list_under(top, &[converter.directory()]) {
            if let Some(id) = file
                .strip_prefix(&format!("{top}/"))
                .and_then(|path| converter.file_to_id(path))
            {
                selected.insert(id, (*pack, file));
            }
        }
    }
    let mut files = BTreeMap::new();
    for (id, (pack, path)) in selected {
        // A file that vanishes or fails to read between listing and reading is one bad
        // entry, as vanilla's `scanDirectory` treats a parse failure: log it, skip it.
        match pack.read(&path) {
            Some(bytes) => {
                files.insert(id, bytes);
            }
            None => log::error!(
                "Couldn't read file '{id}' from '{}/{}' for listener {}",
                pack.id,
                path,
                listener
            ),
        }
    }
    ReloadFiles::new(files)
}
