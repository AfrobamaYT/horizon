//! Sandboxed core-wasm transport. One lock, memory and poison flag per dependency group.
use crate::{
    Refusal,
    boundary::{Cx, Frame, serve},
    link::{Group, Member},
};
use mod_api::{
    HookId,
    wire::{Id, Reader, WireError, Writer},
};
use std::{
    cell::Cell,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use wasmtime::{
    AsContextMut, Caller, Engine, Extern, Func, Memory, Store, StoreContextMut, TypedFunc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallClass {
    Load,
    Hook,
    Worker,
    Callback,
}

pub(crate) struct State {
    limits: wasmtime::StoreLimits,
    cx: Box<Cx>,
    memory: Option<Memory>,
    replies: Vec<Writer>,
    depth: usize,
    group: usize,
}
impl Default for State {
    fn default() -> Self {
        Self {
            limits: wasmtime::StoreLimitsBuilder::new()
                .memory_size(256 << 20)
                .table_elements(1 << 20)
                .build(),
            cx: Box::default(),
            memory: None,
            replies: Vec::new(),
            depth: 0,
            group: 0,
        }
    }
}

pub(crate) fn engine() -> wasmtime::Result<Engine> {
    let mut config = wasmtime::Config::new();
    config.epoch_interruption(true);
    let engine = Engine::new(&config)?;
    let weak = engine.weak();
    std::thread::Builder::new()
        .name("wasm epoch".into())
        .spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(1));
                let Some(engine) = weak.upgrade() else { break };
                engine.increment_epoch();
            }
        })?;
    Ok(engine)
}

struct Running {
    group: Mutex<Group<State>>,
    poisoned: AtomicBool,
}
#[derive(Clone)]
pub struct WasmMod {
    running: Arc<Running>,
    input: TypedFunc<u32, u32>,
    hook: TypedFunc<(u32, u32, u32), u64>,
    pub subscribed: &'static [Option<HookId>],
    released: Arc<Mutex<Vec<Id>>>,
}
pub struct WasmHandler {
    pub index: Id,
    pub module: WasmMod,
}
impl Drop for WasmHandler {
    fn drop(&mut self) {
        self.module
            .released
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(self.index);
    }
}

// An import lends its context only while native code runs. Removing the pointer while
// guest code executes prevents double borrowing; a nested import installs a fresh node.
thread_local! { static LENT: Cell<*const Loan> = const { Cell::new(std::ptr::null()) }; }
struct Loan {
    group: usize,
    context: Cell<*mut StoreContextMut<'static, State>>,
    previous: *const Loan,
}
impl Drop for Loan {
    fn drop(&mut self) {
        LENT.set(self.previous);
    }
}
struct Resumed {
    loan: *const Loan,
    context: *mut StoreContextMut<'static, State>,
}
impl Resumed {
    fn take(group: usize) -> Option<Self> {
        let mut ptr = LENT.get();
        while !ptr.is_null() {
            // SAFETY: nodes live on synchronous import stacks on this thread.
            let loan = unsafe { &*ptr };
            if loan.group == group {
                let context = loan.context.replace(std::ptr::null_mut());
                return (!context.is_null()).then_some(Self { loan: ptr, context });
            }
            ptr = loan.previous;
        }
        None
    }
    fn context(&mut self) -> StoreContextMut<'_, State> {
        // SAFETY: this loan removed the context exclusively until Drop restores it.
        unsafe { &mut *self.context }.as_context_mut()
    }
}
impl Drop for Resumed {
    fn drop(&mut self) {
        unsafe { &*self.loan }.context.set(self.context);
    }
}

fn host(
    mut caller: Caller<'_, State>,
    id: u32,
    ptr: u32,
    len: u32,
    out: u32,
    cap: u32,
) -> wasmtime::Result<u32> {
    let memory = caller
        .data()
        .memory
        .ok_or_else(|| wasmtime::Error::msg("host import during instantiation"))?;
    let end = (ptr as usize)
        .checked_add(len as usize)
        .ok_or_else(|| wasmtime::Error::msg("input range overflow"))?;
    let input = memory
        .data(&caller)
        .get(ptr as usize..end)
        .ok_or_else(|| wasmtime::Error::msg("input outside memory"))?
        .to_vec();
    let depth = caller.data().depth;
    let mut reply = std::mem::take(&mut caller.data_mut().replies[depth]);
    reply.clear();
    let cx = &mut *caller.data_mut().cx as *mut Cx;
    let mut context = caller.as_context_mut();
    let loan = Loan {
        group: context.data().group,
        context: Cell::new((&mut context as *mut StoreContextMut<'_, State>).cast()),
        previous: LENT.get(),
    };
    LENT.set(&loan);
    let result = if id == mod_api::wire::HOST_RELEASE {
        let mut reader = Reader::new(&input);
        let id = reader.take::<Id>()?;
        unsafe { &mut *cx }
            .held
            .remove(id)
            .map(|_| ())
            .map_err(|e| wasmtime::Error::msg(e.to_string()))
    } else {
        // SAFETY: Cx is boxed and stable; serve must release its frame borrows before
        // invoking native code that can synchronously reenter the guest.
        unsafe { serve(id, cx, &input, &mut reply) }
            .map_err(|e| wasmtime::Error::msg(e.to_string()))
    };
    drop(loan);
    result?;
    let size = u32::try_from(reply.bytes().len())?;
    if size <= cap {
        memory.write(&mut caller, out as usize, reply.bytes())?;
    }
    caller.data_mut().replies[depth] = reply;
    Ok(size)
}
fn host_reply(mut caller: Caller<'_, State>, ptr: u32) -> wasmtime::Result<()> {
    let state = caller.data();
    let memory = state
        .memory
        .ok_or_else(|| wasmtime::Error::msg("no group memory"))?;
    let bytes = state.replies[state.depth].bytes().to_vec();
    memory.write(&mut caller, ptr as usize, &bytes)?;
    Ok(())
}

impl WasmMod {
    pub(crate) fn instantiate(
        engine: &Engine,
        members: &[Member<'_>],
    ) -> Result<Vec<Self>, Refusal> {
        let mut store = Store::new(engine, State::default());
        store.limiter(|state| &mut state.limits);
        store.set_epoch_deadline(30_000);
        let mut group = Group::link_with(store, members, |store, module, name| {
            if module != "horizon" {
                return None;
            }
            match name {
                "host" => Some(Extern::Func(Func::wrap(store, host))),
                "host_reply" => Some(Extern::Func(Func::wrap(store, host_reply))),
                _ => None,
            }
        })
        .map_err(|e| Refusal::Runtime(e.to_string()))?;
        group.store.data_mut().memory = Some(group.memory);
        let mut exports = Vec::new();
        for (_, instance) in group.members.clone() {
            // The std member is a library, not a mod.
            if instance
                .get_func(&mut group.store, "horizon_layout")
                .is_none()
            {
                continue;
            }
            let load = |e: wasmtime::Error| Refusal::Runtime(format!("{e:#}"));
            let layout = instance
                .get_typed_func::<(), u64>(&mut group.store, "horizon_layout")
                .map_err(load)?
                .call(&mut group.store, ())
                .map_err(load)?;
            if layout != mod_api::wire::LAYOUT {
                return Err(Refusal::Layout {
                    built: layout,
                    game: mod_api::wire::LAYOUT,
                });
            }
            let input = instance
                .get_typed_func(&mut group.store, "horizon_in")
                .map_err(load)?;
            let hook = instance
                .get_typed_func(&mut group.store, "horizon_hook")
                .map_err(load)?;
            exports.push((input, hook));
        }
        let running = Arc::new(Running {
            group: Mutex::new(group),
            poisoned: AtomicBool::new(false),
        });
        running
            .group
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .store
            .data_mut()
            .group = Arc::as_ptr(&running) as usize;
        Ok(exports
            .into_iter()
            .map(|(input, hook)| Self {
                running: running.clone(),
                input,
                hook,
                subscribed: &[],
                released: Arc::default(),
            })
            .collect())
    }
    pub fn call<R>(
        &self,
        class: CallClass,
        frame: Frame,
        id: u32,
        handler: Option<Id>,
        args: impl FnOnce(&mut Writer),
        reply: impl FnOnce(&mut Reader<'_>) -> Result<R, WireError>,
    ) -> Option<R> {
        if self.running.poisoned.load(Ordering::Relaxed) {
            return None;
        }
        let key = Arc::as_ptr(&self.running) as usize;
        let mut resumed = if class == CallClass::Callback {
            Resumed::take(key)
        } else {
            None
        };
        let mut locked = None;
        if resumed.is_none() {
            let start = Instant::now();
            loop {
                match self.running.group.try_lock() {
                    Ok(group) => {
                        locked = Some(group);
                        break;
                    }
                    Err(std::sync::TryLockError::Poisoned(e)) => {
                        locked = Some(e.into_inner());
                        break;
                    }
                    Err(_)
                        if class != CallClass::Worker
                            && start.elapsed() >= Duration::from_secs(5) =>
                    {
                        log::warn!("wasm group busy");
                        return None;
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(1)),
                }
            }
        }
        if self.running.poisoned.load(Ordering::Relaxed) {
            return None;
        }
        let mut store = match resumed.as_mut() {
            Some(resumed) => resumed.context(),
            None => {
                let store = &mut locked.as_mut()?.store;
                store.set_epoch_deadline(if class == CallClass::Load {
                    30_000
                } else {
                    100
                });
                store.as_context_mut()
            }
        };
        let depth = store.data().depth + 1;
        if depth > 64 {
            self.running.poisoned.store(true, Ordering::Relaxed);
            return None;
        }
        store.data_mut().depth = depth;
        if store.data().replies.len() <= depth {
            store.data_mut().replies.resize_with(depth + 1, Writer::new);
        }
        store.data_mut().cx.frames.push(frame);
        let result = (|| -> wasmtime::Result<R> {
            let released =
                std::mem::take(&mut *self.released.lock().unwrap_or_else(|e| e.into_inner()));
            if !released.is_empty() {
                let mut w = Writer::new();
                w.put(&released);
                self.invoke(&mut store, mod_api::wire::RELEASE, &w)?;
            }
            let mut w = Writer::new();
            if let Some(handler) = handler {
                w.put(&handler);
            }
            args(&mut w);
            let bytes = self.invoke(&mut store, id, &w)?;
            let mut r = Reader::new(&bytes);
            let value = reply(&mut r)?;
            if !r.rest().is_empty() {
                return Err(WireError::Trailing.into());
            }
            Ok(value)
        })();
        store.data_mut().cx.frames.pop();
        store.data_mut().depth -= 1;
        match result {
            Ok(value) => Some(value),
            Err(e) => {
                self.running.poisoned.store(true, Ordering::Relaxed);
                log::error!("wasm group stopped: {e:#}");
                None
            }
        }
    }
    fn invoke(
        &self,
        store: &mut StoreContextMut<'_, State>,
        id: u32,
        input: &Writer,
    ) -> wasmtime::Result<Vec<u8>> {
        let len = u32::try_from(input.bytes().len())?;
        let ptr = self.input.call(&mut *store, len)?;
        let memory = store
            .data()
            .memory
            .ok_or_else(|| wasmtime::Error::msg("no memory"))?;
        memory.write(&mut *store, ptr as usize, input.bytes())?;
        let result = self.hook.call(&mut *store, (id, ptr, len))?;
        let ptr = (result >> 32) as usize;
        let len = result as u32;
        if len == u32::MAX {
            return Err(wasmtime::Error::msg("guest refused call"));
        }
        let end = ptr
            .checked_add(len as usize)
            .ok_or_else(|| wasmtime::Error::msg("reply overflow"))?;
        Ok(memory
            .data(&*store)
            .get(ptr..end)
            .ok_or_else(|| wasmtime::Error::msg("reply outside memory"))?
            .to_vec())
    }
}
