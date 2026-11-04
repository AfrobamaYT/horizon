//! The game's side of the WebAssembly interface (feature `wasm-host`): the generated host
//! forwarding answers a mod's imports from the hosts of the call in progress, and the
//! generated proxies make a wasm mod a [`Hooks`] and its handlers parts. Loading, linking and
//! checking packages is `horizon-mod-wasm`'s; this module runs what it instantiated.
use crate::{
    BackendMut, BlockEntityDrawContext, ClientHost, DrawContext, HookId, Host, LevelHost,
    LevelRandomHost, LevelTicksHost, LevelWriteHost, ServerHost,
};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::time::{Duration, Instant};
use wasmtime::Store;
use wasmtime::component::{Component, Instance, Linker};

/// A value a mod sent that the mod API can't hold: the call traps.
#[derive(Debug)]
pub(crate) struct Refused;

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the mod sent a value the mod API can't hold")
    }
}

impl std::error::Error for Refused {}

/// Why a linked group can't run, named for the Mods page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// The module doesn't export an interface or function this game calls.
    MissingExport(String),
    /// An export this game calls has another type than the game's.
    ChangedExport(String, String),
    /// The game can't provide an import the module needs (the first one is named).
    Import(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingExport(name) => write!(f, "mod.wasm doesn't export `{name}`"),
            Self::ChangedExport(name, why) => {
                write!(f, "mod.wasm's `{name}` has another type: {why}")
            }
            Self::Import(why) => write!(f, "mod.wasm needs what this game doesn't have: {why}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// Run budgets, in wasmtime fuel units (about one per instruction).
#[derive(Clone, Copy, Debug)]
pub struct Budgets {
    /// What one hook or part call may spend.
    pub call: u64,
    /// What a group's start-up (its constructors and `register`) may spend.
    pub init: u64,
    /// The most memory one group may grow to, in bytes.
    pub memory: usize,
}

impl Default for Budgets {
    fn default() -> Self {
        Self {
            call: 50_000_000,
            init: 4_000_000_000,
            memory: 256 << 20,
        }
    }
}

/// A group's store data: its limits; the hosts of the call in progress are on the thread.
pub(crate) struct State {
    limits: wasmtime::StoreLimits,
}

/// The hosts one call into a mod hands over, valid only while that call runs.
#[derive(Default)]
pub(crate) struct Frame {
    host: Option<*const dyn Host>,
    server_host: Option<*mut dyn ServerHost>,
    client_host: Option<*const dyn ClientHost>,
    client_host_mut: Option<*mut dyn ClientHost>,
    level_host: Option<*const dyn LevelHost>,
    level_write_host: Option<*mut dyn LevelWriteHost>,
    level_ticks_host: Option<*mut dyn LevelTicksHost>,
    level_random_host: Option<*mut dyn LevelRandomHost>,
    draw_context: Option<*mut dyn DrawContext>,
    block_entity_draw_context: Option<*mut dyn BlockEntityDrawContext>,
}

/// Erases a borrow's lifetime: the frame holding it lives only for the call it was made for.
fn erase<T: ?Sized>(value: &T) -> *const T {
    value
}

fn erase_mut<T: ?Sized>(value: &mut T) -> *mut T {
    value
}

#[allow(clippy::transmute_ptr_to_ptr)]
impl Frame {
    pub(crate) fn host(&mut self, host: &dyn Host) {
        // SAFETY (all setters): only the lifetime is erased; `frame::*` hands the pointer out
        // only while the call that set it runs.
        self.host =
            Some(unsafe { std::mem::transmute::<*const dyn Host, *const dyn Host>(erase(host)) });
    }
    pub(crate) fn server_host_mut(&mut self, host: &mut dyn ServerHost) {
        self.server_host = Some(unsafe {
            std::mem::transmute::<*mut dyn ServerHost, *mut dyn ServerHost>(erase_mut(host))
        });
    }
    pub(crate) fn client_host(&mut self, host: &dyn ClientHost) {
        self.client_host = Some(unsafe {
            std::mem::transmute::<*const dyn ClientHost, *const dyn ClientHost>(erase(host))
        });
    }
    pub(crate) fn client_host_mut(&mut self, host: &mut dyn ClientHost) {
        self.client_host_mut = Some(unsafe {
            std::mem::transmute::<*mut dyn ClientHost, *mut dyn ClientHost>(erase_mut(host))
        });
    }
    pub(crate) fn level_host(&mut self, host: &dyn LevelHost) {
        self.level_host = Some(unsafe {
            std::mem::transmute::<*const dyn LevelHost, *const dyn LevelHost>(erase(host))
        });
    }
    pub(crate) fn level_write_host_mut(&mut self, host: &mut dyn LevelWriteHost) {
        self.level_write_host = Some(unsafe {
            std::mem::transmute::<*mut dyn LevelWriteHost, *mut dyn LevelWriteHost>(erase_mut(host))
        });
    }
    pub(crate) fn level_ticks_host_mut(&mut self, host: &mut dyn LevelTicksHost) {
        self.level_ticks_host = Some(unsafe {
            std::mem::transmute::<*mut dyn LevelTicksHost, *mut dyn LevelTicksHost>(erase_mut(host))
        });
    }
    pub(crate) fn level_random_host_mut(&mut self, host: &mut dyn LevelRandomHost) {
        self.level_random_host = Some(unsafe {
            std::mem::transmute::<*mut dyn LevelRandomHost, *mut dyn LevelRandomHost>(erase_mut(
                host,
            ))
        });
    }
    pub(crate) fn draw_context_mut(&mut self, draw: &mut dyn DrawContext) {
        self.draw_context = Some(unsafe {
            std::mem::transmute::<*mut dyn DrawContext, *mut dyn DrawContext>(erase_mut(draw))
        });
    }
    pub(crate) fn block_entity_draw_context_mut(&mut self, draw: &mut dyn BlockEntityDrawContext) {
        self.block_entity_draw_context = Some(unsafe {
            std::mem::transmute::<*mut dyn BlockEntityDrawContext, *mut dyn BlockEntityDrawContext>(
                erase_mut(draw),
            )
        });
    }
}

thread_local! {
    /// The frame of the innermost call into a mod on this thread.
    static CURRENT: Cell<*mut Frame> = const { Cell::new(std::ptr::null_mut()) };
}

/// The hosts of the call in progress, as the generated host forwarding asks for them. A
/// host the call didn't hand over is a typed trap: the mod asked for what its hook doesn't
/// give (a client call from a server hook).
pub(crate) mod frame {
    use super::*;

    fn current() -> wasmtime::Result<&'static mut Frame> {
        let frame = CURRENT.get();
        // SAFETY: `call` sets the pointer to a frame on its stack and restores the previous
        // one before that frame drops; imports run only inside a call.
        unsafe { frame.as_mut() }
            .ok_or_else(|| wasmtime::Error::msg("a mod import ran outside a call"))
    }

    fn missing(what: &str) -> wasmtime::Error {
        wasmtime::Error::msg(format!("this hook gives no {what}"))
    }

    pub(crate) fn server_host_mut() -> wasmtime::Result<&'static mut dyn ServerHost> {
        // SAFETY (all getters): the frame's pointers are live for the call (see `Frame`).
        current()?
            .server_host
            .map(|h| unsafe { &mut *h })
            .ok_or_else(|| missing("server host"))
    }
    pub(crate) fn server_host() -> wasmtime::Result<&'static dyn ServerHost> {
        server_host_mut().map(|h| &*h)
    }
    pub(crate) fn client_host_mut() -> wasmtime::Result<&'static mut dyn ClientHost> {
        current()?
            .client_host_mut
            .map(|h| unsafe { &mut *h })
            .ok_or_else(|| missing("mutable client host"))
    }
    pub(crate) fn client_host() -> wasmtime::Result<&'static dyn ClientHost> {
        let frame = current()?;
        match (frame.client_host, frame.client_host_mut) {
            (Some(h), _) => Ok(unsafe { &*h }),
            (None, Some(h)) => Ok(unsafe { &*h }),
            (None, None) => Err(missing("client host")),
        }
    }
    pub(crate) fn host() -> wasmtime::Result<&'static dyn Host> {
        let frame = current()?;
        if let Some(h) = frame.host {
            return Ok(unsafe { &*h });
        }
        if let Some(h) = frame.server_host {
            return Ok(unsafe { &*h });
        }
        client_host()
            .map(|h| h as &dyn Host)
            .map_err(|_| missing("host"))
    }
    pub(crate) fn host_mut() -> wasmtime::Result<&'static mut dyn Host> {
        let frame = current()?;
        if let Some(h) = frame.server_host {
            return Ok(unsafe { &mut *h });
        }
        client_host_mut()
            .map(|h| h as &mut dyn Host)
            .map_err(|_| missing("mutable host"))
    }
    pub(crate) fn level_write_host_mut() -> wasmtime::Result<&'static mut dyn LevelWriteHost> {
        current()?
            .level_write_host
            .map(|h| unsafe { &mut *h })
            .ok_or_else(|| missing("level write host"))
    }
    pub(crate) fn level_ticks_host_mut() -> wasmtime::Result<&'static mut dyn LevelTicksHost> {
        let frame = current()?;
        match (frame.level_ticks_host, frame.level_write_host) {
            (Some(h), _) => Ok(unsafe { &mut *h }),
            (None, Some(h)) => Ok(unsafe { &mut *h }),
            _ => Err(missing("level ticks host")),
        }
    }
    pub(crate) fn level_random_host_mut() -> wasmtime::Result<&'static mut dyn LevelRandomHost> {
        let frame = current()?;
        match (frame.level_random_host, frame.level_write_host) {
            (Some(h), _) => Ok(unsafe { &mut *h }),
            (None, Some(h)) => Ok(unsafe { &mut *h }),
            _ => Err(missing("level random host")),
        }
    }
    pub(crate) fn level_host() -> wasmtime::Result<&'static dyn LevelHost> {
        let frame = current()?;
        if let Some(h) = frame.level_host {
            return Ok(unsafe { &*h });
        }
        if let Some(h) = frame.level_write_host {
            return Ok(unsafe { &*h });
        }
        if let Some(h) = frame.level_ticks_host {
            return Ok(unsafe { &*h });
        }
        frame
            .level_random_host
            .map(|h| unsafe { &*h as &dyn LevelHost })
            .ok_or_else(|| missing("level host"))
    }
    pub(crate) fn draw_context_mut() -> wasmtime::Result<&'static mut dyn DrawContext> {
        current()?
            .draw_context
            .map(|h| unsafe { &mut *h })
            .ok_or_else(|| missing("draw context"))
    }
    pub(crate) fn draw_context() -> wasmtime::Result<&'static dyn DrawContext> {
        draw_context_mut().map(|h| &*h)
    }
    pub(crate) fn block_entity_draw_context_mut()
    -> wasmtime::Result<&'static mut dyn BlockEntityDrawContext> {
        current()?
            .block_entity_draw_context
            .map(|h| unsafe { &mut *h })
            .ok_or_else(|| missing("block entity draw context"))
    }
    pub(crate) fn block_entity_draw_context()
    -> wasmtime::Result<&'static dyn BlockEntityDrawContext> {
        block_entity_draw_context_mut().map(|h| &*h)
    }
    pub(crate) fn slot_view() -> wasmtime::Result<&'static dyn crate::menu::SlotView> {
        Err(missing("slot view"))
    }
    pub(crate) fn slot_view_mut() -> wasmtime::Result<&'static mut dyn crate::menu::SlotView> {
        Err(missing("slot view"))
    }
    pub(crate) fn config_files() -> wasmtime::Result<&'static dyn crate::ConfigFiles> {
        Ok(host()?.config_files())
    }
    pub(crate) fn backend() -> wasmtime::Result<&'static dyn crate::Backend> {
        host()?
            .backend()
            .map(crate::BackendRef::open)
            .ok_or_else(|| missing("game backend"))
    }
    pub(crate) fn backend_mut_mut() -> wasmtime::Result<&'static mut dyn BackendMut> {
        crate::backend_mut_of(server_host_mut()?).ok_or_else(|| missing("game backend"))
    }
}

fn array<T, const N: usize>(items: Vec<T>) -> Result<[T; N], Refused> {
    items.try_into().map_err(|_| Refused)
}

/// The one `&'static str` for each distinct string a mod names.
fn intern(value: String) -> &'static str {
    static INTERNED: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);
    let mut interned = INTERNED.lock().unwrap_or_else(|e| e.into_inner());
    let set = interned.get_or_insert_with(HashSet::new);
    match set.get(value.as_str()) {
        Some(existing) => existing,
        None => {
            let leaked: &'static str = Box::leak(value.into_boxed_str());
            set.insert(leaked);
            leaked
        }
    }
}

/// One call's storage for the values its borrowed arguments point into, as on the guest.
#[derive(Default)]
pub(crate) struct Keep {
    held: RefCell<Vec<(*mut u8, unsafe fn(*mut u8))>>,
}

impl Keep {
    fn hold<T>(&self, value: T) -> *mut T {
        unsafe fn drop_box<T>(pointer: *mut u8) {
            // SAFETY: `pointer` came from `Box::into_raw` of a `Box<T>` in `hold`, once.
            drop(unsafe { Box::from_raw(pointer.cast::<T>()) });
        }
        let pointer = Box::into_raw(Box::new(value));
        self.held.borrow_mut().push((pointer.cast(), drop_box::<T>));
        pointer
    }
    pub(crate) fn str(&self, value: String) -> &str {
        // SAFETY: the box stays allocated, unmoved and handed out once, until `self` drops.
        unsafe { &*self.hold(value) }
    }
    pub(crate) fn slice<T>(&self, value: Vec<T>) -> &[T] {
        // SAFETY: as in `str`.
        unsafe { &*self.hold(value) }
    }
    pub(crate) fn val<T>(&self, value: T) -> &T {
        // SAFETY: as in `str`.
        unsafe { &*self.hold(value) }
    }
    #[allow(clippy::mut_from_ref)]
    pub(crate) fn val_mut<T>(&self, value: T) -> &mut T {
        // SAFETY: as in `str`.
        unsafe { &mut *self.hold(value) }
    }
}

impl Drop for Keep {
    fn drop(&mut self) {
        for (pointer, drop) in self.held.get_mut().drain(..).rev() {
            // SAFETY: each entry is a live box `hold` made, dropped once here.
            unsafe { drop(pointer) };
        }
    }
}

/// How long a call waits for a group another thread is running before it refuses.
const GROUP_WAIT: Duration = Duration::from_secs(5);

/// One linked dependency group: one store, one memory, one copy of each member's code.
pub struct Group {
    store: Mutex<Store<State>>,
    /// The thread running the group now, so a call back into it from that thread refuses
    /// instead of deadlocking.
    running: AtomicU64,
    poisoned: AtomicBool,
    budgets: Budgets,
}

/// A thread's number, for `Group::running` (0 is no thread).
fn thread_number() -> u64 {
    thread_local!(static NUMBER: u64 = {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        NEXT.fetch_add(1, Ordering::Relaxed)
    });
    NUMBER.with(|n| *n)
}

/// Why a call into a group didn't run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallRefused {
    /// A trap stopped the group earlier; it runs no more calls.
    Stopped,
    /// The call came back into the group from inside one of its own calls.
    Reentered,
    /// Another thread kept the group busy past the wait.
    Busy,
}

impl Group {
    fn enter(&self) -> Result<MutexGuard<'_, Store<State>>, CallRefused> {
        if self.poisoned.load(Ordering::Relaxed) {
            return Err(CallRefused::Stopped);
        }
        let me = thread_number();
        if self.running.load(Ordering::Acquire) == me {
            return Err(CallRefused::Reentered);
        }
        let deadline = Instant::now() + GROUP_WAIT;
        let guard = loop {
            match self.store.try_lock() {
                Ok(guard) => break guard,
                Err(TryLockError::Poisoned(e)) => break e.into_inner(),
                Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                    std::thread::yield_now()
                }
                Err(TryLockError::WouldBlock) => return Err(CallRefused::Busy),
            }
        };
        self.running.store(me, Ordering::Release);
        Ok(guard)
    }
}

/// One wasm mod of a group: the [`Hooks`](crate::Hooks) the game dispatches to.
pub struct WasmMod {
    group: Arc<Group>,
    exports: glue::ModExports,
    id: &'static str,
    subscribed: &'static [Option<HookId>],
    fuel: AtomicU64,
}

/// One of a wasm mod's registered parts or handlers, by the index the mod keeps it under.
pub struct WasmHandler {
    module: Arc<WasmMod>,
    index: u32,
    subscribed: &'static [Option<HookId>],
}

/// What a call answers when the mod gives no answer (its call refused, or a trap): what a
/// mod not listening would.
pub(crate) trait NoAnswer {
    fn no_answer() -> Self;
}

impl<T: Default> NoAnswer for T {
    fn no_answer() -> Self {
        T::default()
    }
}

impl NoAnswer for crate::Flow {
    fn no_answer() -> Self {
        crate::Flow::Continue
    }
}

impl WasmMod {
    /// The fuel this mod's calls have spent.
    pub fn fuel_spent(&self) -> u64 {
        self.fuel.load(Ordering::Relaxed)
    }

    /// Runs one call with `frame`'s hosts, charging its fuel to this mod. A refused call or a
    /// trap answers `T::default()`, as no mod answering; a trap stops the whole group.
    fn call<T: NoAnswer>(
        &self,
        mut frame: Frame,
        f: impl FnOnce(&glue::ModExports, &mut Store<State>) -> wasmtime::Result<T>,
    ) -> T {
        let mut store = match self.group.enter() {
            Ok(store) => store,
            Err(refused) => {
                log::warn!("wasm mod {}: call refused: {refused:?}", self.id);
                return T::no_answer();
            }
        };
        let budget = self.group.budgets.call;
        let _ = store.set_fuel(budget);
        let previous = CURRENT.replace(&mut frame);
        let result = f(&self.exports, &mut store);
        CURRENT.set(previous);
        self.fuel
            .fetch_add(budget - store.get_fuel().unwrap_or(0), Ordering::Relaxed);
        self.group.running.store(0, Ordering::Release);
        drop(store);
        match result {
            Ok(value) => value,
            Err(error) => {
                self.group.poisoned.store(true, Ordering::Relaxed);
                log::error!("wasm mod {} stopped its group: {error:#}", self.id);
                T::no_answer()
            }
        }
    }
}

impl WasmHandler {
    fn call<T: NoAnswer>(
        &self,
        frame: Frame,
        f: impl FnOnce(&glue::ModExports, &mut Store<State>) -> wasmtime::Result<T>,
    ) -> T {
        self.module.call(frame, f)
    }
}

/// Instantiates a linked group and makes each member a mod entry. `members` are the mods'
/// ids in dependency order; their exports are `<id>-<interface>` in the group.
pub fn instantiate(
    engine: &wasmtime::Engine,
    component: &Component,
    members: &[&'static str],
    budgets: Budgets,
) -> Result<Vec<crate::ModEntry>, LoadError> {
    let mut linker = Linker::<State>::new(engine);
    glue::Mod::add_to_linker::<State, wasmtime::component::HasSelf<State>>(&mut linker, |s| s)
        .map_err(|e| LoadError::Import(e.to_string()))?;
    let limits = wasmtime::StoreLimitsBuilder::new()
        .memory_size(budgets.memory)
        .build();
    let mut store = Store::new(engine, State { limits });
    store.limiter(|state| &mut state.limits);
    let _ = store.set_fuel(budgets.init);
    let instance: Instance = linker
        .instantiate(&mut store, component)
        .map_err(|e| LoadError::Import(format!("{e:#}")))?;
    let mut found = Vec::new();
    for id in members {
        found.push((*id, glue::ModExports::find(&mut store, &instance, id)?));
    }
    let group = Arc::new(Group {
        store: Mutex::new(store),
        running: AtomicU64::new(0),
        poisoned: AtomicBool::new(false),
        budgets,
    });
    let mut entries = Vec::new();
    for (id, exports) in found {
        let module = WasmMod {
            group: group.clone(),
            exports,
            id,
            subscribed: &[],
            fuel: AtomicU64::new(0),
        };
        let names = module.call(Frame::default(), |m, store| {
            m.horizon_mod_api_hooks().call_subscribed(store)
        });
        let subscribed: &'static [Option<HookId>] = Box::leak(
            names
                .iter()
                .map(|n| glue::hook_id(n))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        entries.push(crate::ModEntry::new(
            id,
            Box::new(WasmMod {
                subscribed,
                ..module
            }),
        ));
    }
    Ok(entries)
}

mod codec {
    use super::Refused;
    codecs!(super::glue::horizon::mod_api::types);
}

#[allow(
    clippy::all,
    unused_imports,
    unused_variables,
    unused_parens,
    dead_code,
    non_snake_case,
    unused_mut
)]
mod glue {
    use super::{array, intern};
    include!(concat!(env!("OUT_DIR"), "/wasm_host_generated.rs"));
}
