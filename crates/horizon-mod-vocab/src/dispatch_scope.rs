//! Engine-owned dispatch instrumentation socket.
//!
//! The platform installs one backend before loading mods. A backend receives a
//! factory capability on entry: it cannot manufacture guards associated with a
//! different backend. No allocation or erased object is needed for a call.
use crate::content::crash::ModCall;
use std::sync::OnceLock;

static PROFILING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Set the dispatch activity flag from the platform profiler control path.
#[doc(hidden)]
pub fn set_profiling(on: bool) {
    PROFILING.store(on, std::sync::atomic::Ordering::Relaxed);
}

static BACKEND: OnceLock<Option<Backend>> = OnceLock::new();

/// Platform callbacks, installed once before dispatch starts.
#[doc(hidden)]
pub struct Backend {
    pub enter: fn(ModGuardFactory, ModCall) -> ModGuard,
    pub leave: fn(State),
    pub current_native: fn() -> Option<ModCall>,
    pub current_converted: fn() -> Option<crate::content::crash::ConvertedCall>,
    pub enter_converted: fn(
        crate::content::crash::ConvertedCall,
    ) -> Option<crate::content::crash::ConvertedPackageGuard>,
    pub set_detail: fn(&'static str, String),
    pub clear_detail: fn(&'static str),
    pub details: fn() -> Vec<(&'static str, String)>,
}

/// Plain per-call instrumentation data. The platform must validate the pool
/// generation and slot before accessing its profiler storage.
#[doc(hidden)]
pub struct Timing {
    pub generation: usize,
    pub slot: usize,
    pub record: usize,
}

/// The outer attribution and optional timing record restored at scope exit.
#[doc(hidden)]
pub struct State {
    pub outer_frame: Option<u64>,
    pub outer_call: Option<ModCall>,
    pub timing: Option<Timing>,
}

/// A capability to bind exactly one guard to the installed leave callback.
/// Its fields are private and it is neither Clone nor Copy.
#[doc(hidden)]
pub struct ModGuardFactory {
    leave: fn(State),
    owner: (Option<u64>, Option<ModCall>),
}
impl ModGuardFactory {
    /// Consume the capability, transferring the backend's per-call state.
    pub fn finish(self, state: State) -> ModGuard {
        ModGuard {
            owner: self.owner,
            timing: state.timing.map(|timing| (self.leave, timing)),
            _thread: std::marker::PhantomData,
        }
    }
}

/// An allocation-free dispatch scope. With no instrumentation backend this is
/// inert; otherwise Drop restores attribution, including during unwinding.
#[must_use]
pub struct ModGuard {
    owner: (Option<u64>, Option<ModCall>),
    timing: Option<(fn(State), Timing)>,
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}
impl Drop for ModGuard {
    fn drop(&mut self) {
        crate::content::crash::pop(self.owner.0, self.owner.1);
        if let Some((leave, timing)) = self.timing.take() {
            leave(State {
                outer_frame: None,
                outer_call: None,
                timing: Some(timing),
            });
        }
    }
}

/// Install the instrumentation backend. An already installed backend is never
/// replaced, including after an unsuccessful second registration.
#[doc(hidden)]
pub fn install(backend: Backend) -> Result<(), &'static str> {
    BACKEND
        .set(Some(backend))
        .map_err(|_| "dispatch instrumentation is already sealed")
}

pub(crate) fn enter(call: ModCall) -> ModGuard {
    let owner = crate::content::crash::push(call);
    if PROFILING.load(std::sync::atomic::Ordering::Relaxed) {
        if let Some(backend) = BACKEND.get_or_init(|| None) {
            return (backend.enter)(
                ModGuardFactory {
                    leave: backend.leave,
                    owner,
                },
                call,
            );
        }
    }
    ModGuard {
        owner,
        timing: None,
        _thread: std::marker::PhantomData,
    }
}

pub(crate) fn backend() -> Option<&'static Backend> {
    BACKEND.get().and_then(Option::as_ref)
}
