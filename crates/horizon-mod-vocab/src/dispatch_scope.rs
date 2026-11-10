//! Engine-owned dispatch instrumentation socket.
//!
//! The platform installs one backend before loading mods. A backend receives a
//! factory capability on entry: it cannot manufacture guards associated with a
//! different backend. No allocation or erased object is needed for a call.
use crate::content::crash::ModCall;
use std::sync::OnceLock;

static BACKEND: OnceLock<Option<Backend>> = OnceLock::new();

/// Platform callbacks, installed once before dispatch starts.
#[doc(hidden)]
pub struct Backend {
    pub enter: fn(ModGuardFactory<'_>, ModCall),
    pub leave: fn(&mut State),
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
    pub began: u64,
    pub call: ModCall,
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
pub struct ModGuardFactory<'a> {
    guard: &'a mut ModGuard,
    leave: fn(&mut State),
}
impl ModGuardFactory<'_> {
    /// Consume the capability, transferring the backend's per-call state.
    #[inline]
    pub fn finish(self, state: State) {
        self.guard.active = Some((self.leave, state));
    }
}

/// An allocation-free dispatch scope. With no instrumentation backend this is
/// inert; otherwise Drop restores attribution, including during unwinding.
#[must_use]
pub struct ModGuard {
    active: Option<(fn(&mut State), State)>,
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}
impl Drop for ModGuard {
    fn drop(&mut self) {
        if let Some((leave, state)) = self.active.as_mut() {
            leave(state);
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
    let mut guard = ModGuard {
        active: None,
        _thread: std::marker::PhantomData,
    };
    if let Some(backend) = BACKEND.get_or_init(|| None) {
        (backend.enter)(
            ModGuardFactory {
                guard: &mut guard,
                leave: backend.leave,
            },
            call,
        );
    }
    guard
}

pub(crate) fn backend() -> Option<&'static Backend> {
    BACKEND.get().and_then(Option::as_ref)
}
