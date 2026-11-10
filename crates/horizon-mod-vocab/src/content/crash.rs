//! Crash attribution vocabulary and engine-owned instrumentation entry points.
pub use crate::dispatch_scope::ModGuard;
use crate::dispatch_scope::backend;
use std::borrow::Cow;
use std::cell::Cell;
thread_local! {
 static OWNER: Cell<Option<ModCall>> = const { Cell::new(None) };
 static FRAME: Cell<Option<u64>> = const { Cell::new(None) };
 static NEXT: Cell<u64> = const { Cell::new(0) };
}
#[doc(hidden)]
pub fn native_frame() -> Option<u64> {
    FRAME.try_with(Cell::get).ok().flatten()
}
#[inline]
pub(crate) fn push(call: ModCall) -> (Option<u64>, Option<ModCall>) {
    let frame = NEXT
        .try_with(|next| {
            let frame = next.get().checked_add(1);
            if let Some(frame) = frame {
                next.set(frame)
            }
            frame
        })
        .ok()
        .flatten();
    (
        FRAME.try_with(|c| c.replace(frame)).ok().flatten(),
        OWNER.try_with(|c| c.replace(Some(call))).ok().flatten(),
    )
}
#[inline]
pub(crate) fn pop(frame: Option<u64>, owner: Option<ModCall>) {
    let _ = FRAME.try_with(|c| c.set(frame));
    let _ = OWNER.try_with(|c| c.set(owner));
}
// no wire: process-local crash attribution uses static runtime identities; not a hook payload.
/// A call into one hook of a native mod.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
// exhaustive: the game builds it with a literal; mods only read it
pub struct ModCall {
    /// The mod's `[mod] id`.
    pub id: &'static str,
    pub hook: &'static str,
    /// The part's type (`std::any::type_name`), when the call is to one of the mod's parts.
    pub part: Option<&'static str>,
}
// no wire: process-local crash attribution uses static runtime identities; not a hook payload.
/// A call into one package a native runtime hosts, beside the runtime's own [`ModCall`].
#[derive(Clone, Debug, PartialEq, Eq)]
// exhaustive: the game builds it with a literal; mods only read it
pub struct ConvertedCall {
    /// The hosting runtime's `[mod] id`, the owner the report names first.
    pub runtime: &'static str,
    /// The package the runtime declared it loaded.
    pub package: Cow<'static, str>,
    /// The guest export being run.
    pub export: Cow<'static, str>,
    /// Where in the guest code, as the runtime names it (`Foo.tick(Foo.java:42)`).
    pub site: Cow<'static, str>,
}

// no wire: process-local crash attribution uses static runtime identities; not a hook payload.
/// The packages one native runtime declared it loaded.
///
/// A runtime that hosts converted code (Portizon's wasm packages, a Java interpreter running
/// jars) builds this once from the packages it loaded and keeps it while they run. A call into
/// a package goes through the [`ConvertedPackage`] this registry hands out for an id it
/// declared, so guest code cannot name a package from another mod. The runtime's own `[mod] id`
/// stays the report's first name; a package and its call are told after it.
// exhaustive: its fields are private, so mods cannot build or destructure it
pub struct ConvertedPackages {
    runtime: &'static str,
    packages: Vec<Cow<'static, str>>,
}

impl ConvertedPackages {
    /// Declares `packages` as the ones `runtime` (its `[mod] id`) loaded. Ids read from
    /// manifests or module bytes are stored here. Entering clones an owned id on each call.
    pub fn new<I>(runtime: &'static str, packages: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Cow<'static, str>>,
    {
        Self {
            runtime,
            packages: packages.into_iter().map(Into::into).collect(),
        }
    }

    /// The handle for `id`, or None when this runtime did not declare that package.
    pub fn package(&self, id: &str) -> Option<ConvertedPackage<'_>> {
        self.packages
            .iter()
            .find(|package| package.as_ref() == id)
            .map(|package| ConvertedPackage {
                runtime: self.runtime,
                package,
            })
    }
}

// no wire: borrowed crash attribution entry tied to its package inventory.
/// One package a [`ConvertedPackages`] declared; only that registry makes one.
#[derive(Clone, Copy, Debug)]
// exhaustive: its fields are private, so only a registry creates it
pub struct ConvertedPackage<'a> {
    runtime: &'static str,
    package: &'a Cow<'static, str>,
}

impl ConvertedPackage<'_> {
    /// Marks this thread as running `export` of this package at `site` until the guard drops:
    /// a panic in that call names the runtime, the package, the export and the site. Nested
    /// scopes restore the one they found, on a normal return and while a panic unwinds.
    ///
    /// None when this thread is not inside a call the package's runtime owns: another native
    /// mod's hook never attributes its crash to this runtime's package.
    pub fn enter(
        &self,
        export: impl Into<Cow<'static, str>>,
        site: impl Into<Cow<'static, str>>,
    ) -> Option<ConvertedPackageGuard> {
        if current_native_mod() != Some(self.runtime) {
            return None;
        }
        let call = ConvertedCall {
            runtime: self.runtime,
            package: self.package.clone(),
            export: export.into(),
            site: site.into(),
        };
        crate::dispatch_scope::backend().and_then(|backend| (backend.enter_converted)(call))
    }
}
/// The call currently attributed to the executing thread.
pub fn current_native_call() -> Option<ModCall> {
    OWNER.try_with(Cell::get).ok().flatten()
}
/// The current native mod identity.
pub fn current_native_mod() -> Option<&'static str> {
    current_native_call().map(|call| call.id)
}
/// Enter an instrumented native call, or an inert scope without a backend.
#[inline]
pub fn enter_native_mod(call: ModCall) -> ModGuard {
    crate::dispatch_scope::enter(call)
}
/// The current hosted package call.
pub fn current_converted_call() -> Option<ConvertedCall> {
    backend().and_then(|b| (b.current_converted)())
}
/// Set a crash report detail when instrumentation is installed.
pub fn set_detail(key: &'static str, value: impl Into<String>) {
    if let Some(b) = backend() {
        (b.set_detail)(key, value.into());
    }
}
/// Clear a crash report detail.
pub fn clear_detail(key: &'static str) {
    if let Some(b) = backend() {
        (b.clear_detail)(key);
    }
}
/// Snapshot the current crash report details.
pub fn details() -> Vec<(&'static str, String)> {
    backend().map(|b| (b.details)()).unwrap_or_default()
}
/// An allocation-free hosted package scope, restored by its owning backend.
#[must_use]
pub struct ConvertedPackageGuard {
    leave: fn(u64, Option<ConvertedScope>),
    token: u64,
    outer: Option<ConvertedScope>,
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}
/// Per-call hosted attribution data, not process state.
#[doc(hidden)]
pub struct ConvertedScope {
    pub call: ConvertedCall,
    pub frame: u64,
    pub token: u64,
}
impl ConvertedPackageGuard {
    /// Construct a guard for a hosted attribution scope.
    #[doc(hidden)]
    pub fn new(
        leave: fn(u64, Option<ConvertedScope>),
        token: u64,
        outer: Option<ConvertedScope>,
    ) -> Self {
        Self {
            leave,
            token,
            outer,
            _thread: std::marker::PhantomData,
        }
    }
}
impl Drop for ConvertedPackageGuard {
    fn drop(&mut self) {
        (self.leave)(self.token, self.outer.take());
    }
}
