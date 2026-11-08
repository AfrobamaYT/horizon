//! Moved here from the game so mods can name it.
#[allow(unused_imports)]
use crate::*;
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::sync::Mutex;

thread_local! {
    /// The native mod hook this thread is inside, for the panic hook to name.
    static IN_NATIVE_MOD: Cell<Option<NativeModCall>> = const { Cell::new(None) };
    /// The converted package this thread is executing, for the panic hook to name.
    static IN_CONVERTED_PACKAGE: RefCell<Option<ConvertedCall>> = const { RefCell::new(None) };
}

/// The native mod hook this thread is inside, if any.
pub fn current_native_call() -> Option<NativeModCall> {
    IN_NATIVE_MOD.with(Cell::get)
}
/// What the game is doing, set as it goes: graphics adapter, settings, packs, world.
static DETAILS: Mutex<Vec<(&'static str, String)>> = Mutex::new(Vec::new());
/// A call into one hook of a native mod.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
// exhaustive: the game builds it with a literal; mods only read it
pub struct NativeModCall {
    /// The mod's `[mod] id`.
    pub id: &'static str,
    pub hook: &'static str,
    /// The part's type (`std::any::type_name`), when the call is to one of the mod's parts.
    pub part: Option<&'static str>,
}
/// Marks this thread as inside `call` until the guard drops, which also happens while a panic
/// unwinds. A hook called from a hook is named by the inner call.
pub fn enter_native_mod(call: NativeModCall) -> NativeModGuard {
    NativeModGuard {
        outer: IN_NATIVE_MOD.with(|c| c.replace(Some(call))),
        _timing: crate::__private::profiler::enter(call),
    }
}
/// The `[mod] id` of the native mod this thread is inside a hook of, if any.
pub fn current_native_mod() -> Option<&'static str> {
    IN_NATIVE_MOD.with(Cell::get).map(|call| call.id)
}

/// A call into one package a native runtime hosts, beside the runtime's own [`NativeModCall`].
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

/// The converted package this thread is executing, if any.
pub fn current_converted_call() -> Option<ConvertedCall> {
    IN_CONVERTED_PACKAGE.with(|current| current.borrow().clone())
}

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
    /// manifests or module bytes are interned once here, before any call reaches them.
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
        let outer = IN_CONVERTED_PACKAGE.with(|current| current.replace(Some(call)));
        Some(ConvertedPackageGuard { outer })
    }
}

/// Puts back the converted package scope that `ConvertedPackage::enter` found.
#[must_use]
// exhaustive: its field is private, so mods cannot build or destructure it
pub struct ConvertedPackageGuard {
    outer: Option<ConvertedCall>,
}

impl Drop for ConvertedPackageGuard {
    fn drop(&mut self) {
        IN_CONVERTED_PACKAGE.with(|current| *current.borrow_mut() = self.outer.take());
    }
}

/// Puts back the call that `enter_native_mod` found.
#[must_use]
// exhaustive: its fields are private, so mods cannot build or destructure it
pub struct NativeModGuard {
    outer: Option<NativeModCall>,
    _timing: Option<crate::__private::profiler::Guard>,
}
impl Drop for NativeModGuard {
    fn drop(&mut self) {
        IN_NATIVE_MOD.with(|c| c.set(self.outer));
    }
}
/// Sets one line of the report's details (replacing the last value under `key`).
pub fn set_detail(key: &'static str, value: impl Into<String>) {
    let mut details = DETAILS.lock().unwrap_or_else(|e| e.into_inner());
    let value = value.into();
    match details.iter_mut().find(|(k, _)| *k == key) {
        Some(entry) => entry.1 = value,
        None => details.push((key, value)),
    }
}
pub fn clear_detail(key: &'static str) {
    DETAILS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .retain(|(k, _)| *k != key);
}
pub fn details() -> Vec<(&'static str, String)> {
    DETAILS.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime(hook: &'static str) -> NativeModGuard {
        enter_native_mod(NativeModCall {
            id: "runtime",
            hook,
            part: None,
        })
    }

    #[test]
    fn a_declared_package_scopes_a_call_to_it() {
        let _runtime = runtime("dispatch");
        let packages = ConvertedPackages::new("runtime", ["example:mod", "example:data"]);
        let package = packages.package("example:mod").unwrap();
        let call = package
            .enter("tick", "com.example.Foo.tick(Foo.java:42)")
            .unwrap();
        let scoped = current_converted_call().unwrap();
        assert_eq!(scoped.runtime, "runtime");
        assert_eq!(scoped.package, "example:mod");
        assert_eq!(scoped.export, "tick");
        assert_eq!(scoped.site, "com.example.Foo.tick(Foo.java:42)");
        drop(call);
        assert_eq!(current_converted_call(), None);
    }

    #[test]
    fn an_undeclared_package_is_rejected() {
        let packages = ConvertedPackages::new("runtime", ["example:mod"]);
        assert!(packages.package("example:other").is_none());
    }

    #[test]
    fn another_mods_call_cannot_enter_this_runtimes_package() {
        let packages = ConvertedPackages::new("runtime", ["example:mod"]);
        let package = packages.package("example:mod").unwrap();
        let _other = enter_native_mod(NativeModCall {
            id: "other",
            hook: "tick",
            part: None,
        });
        assert!(package.enter("tick", "Foo.tick").is_none());
        assert_eq!(current_converted_call(), None);
        // And with no native call at all.
        drop(_other);
        assert!(package.enter("tick", "Foo.tick").is_none());
    }

    #[test]
    fn nested_scopes_restore_the_one_before() {
        let _runtime = runtime("dispatch");
        let packages = ConvertedPackages::new("runtime", ["example:outer", "example:inner"]);
        let outer = packages.package("example:outer").unwrap();
        let inner = packages.package("example:inner").unwrap();
        let _outer = outer.enter("run", "Outer.run(Outer.java:1)").unwrap();
        let _inner = inner.enter("draw", "Inner.draw(Inner.java:2)").unwrap();
        assert_eq!(current_converted_call().unwrap().package, "example:inner");
        drop(_inner);
        assert_eq!(current_converted_call().unwrap().package, "example:outer");
        drop(_outer);
        assert_eq!(current_converted_call(), None);
    }

    #[test]
    fn a_scope_unwound_by_a_panic_restores_the_one_before() {
        let _runtime = runtime("dispatch");
        let packages = ConvertedPackages::new("runtime", ["example:outer", "example:inner"]);
        let outer = packages.package("example:outer").unwrap();
        let inner = packages.package("example:inner").unwrap();
        let _outer = outer.enter("run", "Outer.run(Outer.java:1)").unwrap();
        // The panic is the guest's, not ours: nothing here catches it, so it unwinds past
        // the scope, which goes back to the outer package on the way out.
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _inner = inner.enter("draw", "Inner.draw(Inner.java:2)").unwrap();
            panic!("guest panic");
        }));
        assert!(panic.is_err(), "a fatal panic is not swallowed");
        assert_eq!(current_converted_call().unwrap().package, "example:outer");
        assert_eq!(current_native_mod(), Some("runtime"));
    }
}
