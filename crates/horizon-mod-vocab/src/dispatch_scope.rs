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
    pub unresolved_fluids: crate::__private::UnresolvedFluidAccess,
    pub enter: fn(ModGuardFactory, ModCall) -> ModGuard,
    pub leave: fn(State),
    pub profiler_enabled: &'static std::sync::atomic::AtomicBool,
    pub current_converted: fn() -> Option<crate::content::crash::ConvertedCall>,
    pub enter_converted: fn(
        crate::content::crash::ConvertedCall,
    ) -> Option<crate::content::crash::ConvertedPackageGuard>,
    pub set_detail: fn(&'static str, String),
    pub clear_detail: fn(&'static str),
    pub details: fn() -> Vec<(&'static str, String)>,
    pub current_phase: fn() -> crate::__private::ModPhase,
    pub phase_state: fn() -> crate::__private::PhaseState,
    pub set_phase: fn(crate::__private::PhaseState),
    pub packages: fn() -> std::sync::Arc<[crate::PackageInfo]>,
    pub registration_data: fn() -> Option<crate::__private::RegistrationData>,
}

/// Plain per-call instrumentation data. The platform must validate the pool
/// generation and slot before accessing its profiler storage.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct Timing {
    pub generation: usize,
    pub slot: usize,
    pub record: usize,
}

/// Platform-owned timing data bound to a scope's factory capability.
#[doc(hidden)]
pub struct State {
    pub timing: Option<Timing>,
}

/// A capability to bind exactly one guard to the installed leave callback.
/// Its fields are private and it is neither Clone nor Copy.
#[doc(hidden)]
pub struct ModGuardFactory {
    leave: fn(State),
    owner: (u64, Option<ModCall>),
}
impl ModGuardFactory {
    /// Consume the capability, transferring the backend's per-call state.
    pub fn finish(self, state: State) -> ModGuard {
        ModGuard {
            owner: Some(self.owner),
            _timing: state.timing.map(|timing| TimingGuard {
                leave: self.leave,
                timing,
            }),
            _thread: std::marker::PhantomData,
        }
    }
}

/// An allocation-free dispatch scope. With no instrumentation backend this is
/// inert; otherwise Drop restores attribution, including during unwinding.
#[must_use]
pub struct ModGuard {
    owner: Option<(u64, Option<ModCall>)>,
    _timing: Option<TimingGuard>,
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}
impl Drop for ModGuard {
    #[inline]
    fn drop(&mut self) {
        if let Some((frame, call)) = self.owner {
            crate::content::crash::pop(frame, call);
        }
    }
}

struct TimingGuard {
    leave: fn(State),
    timing: Timing,
}
impl Drop for TimingGuard {
    fn drop(&mut self) {
        (self.leave)(State {
            timing: Some(self.timing),
        });
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

#[inline]
pub(crate) fn enter(call: ModCall) -> ModGuard {
    if let Some(backend) = BACKEND.get_or_init(|| None) {
        let owner = crate::content::crash::push(call);
        if backend
            .profiler_enabled
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            return (backend.enter)(
                ModGuardFactory {
                    leave: backend.leave,
                    owner,
                },
                call,
            );
        }
        return ModGuard {
            owner: Some(owner),
            _timing: None,
            _thread: std::marker::PhantomData,
        };
    }
    ModGuard {
        owner: None,
        _timing: None,
        _thread: std::marker::PhantomData,
    }
}

pub(crate) fn backend() -> Option<&'static Backend> {
    BACKEND.get().and_then(Option::as_ref)
}
