//! Exercise the generated proxies and service against the real guest dispatcher, without wasm.
#![allow(dead_code, unused_variables, unused_mut, unsafe_op_in_unsafe_fn)]
use mod_api::wire::{Id, Reader, WireError, Writer, guest};
use mod_api::{Hooks, LoopContext, StructurePiece};
use std::{rc::Rc, sync::Arc};
mod boundary {
    include!(concat!(env!("OUT_DIR"), "/wire_host_generated.rs"));
}
#[derive(Clone)]
struct WasmMod {
    subscribed: &'static [Option<mod_api::HookId>],
    corrupt: bool,
}
impl std::fmt::Debug for WasmMod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Loopback")
    }
}
#[derive(Debug)]
struct WasmHandler {
    index: Id,
    module: WasmMod,
}
#[derive(Clone, Copy)]
enum CallClass {
    Load,
    Hook,
    Worker,
    Callback,
}
trait NoAnswer {
    fn no_answer() -> Self;
}
impl<T: Default> NoAnswer for T {
    fn no_answer() -> Self {
        Self::default()
    }
}
fn call_refused<E: From<(mod_api::Identifier, String)>>() -> E {
    E::from((
        mod_api::Identifier::parse("test:refused").unwrap(),
        "refused".into(),
    ))
}
impl WasmHandler {
    fn subscriptions(&self, _: &'static str, _: u32) -> &'static [Option<mod_api::HookId>] {
        &[]
    }
}
impl WasmMod {
    fn call<R>(
        &self,
        _: CallClass,
        frame: boundary::Frame,
        id: u32,
        handler: Option<Id>,
        encode: impl FnOnce(&mut Writer),
        decode: impl FnOnce(&mut Reader<'_>) -> Result<R, WireError>,
    ) -> Option<R> {
        let mut cx = Box::new(boundary::Cx::default());
        cx.frames.push(frame);
        let pointer = &mut *cx as *mut boundary::Cx;
        // The pointer remains live until this synchronous call finishes; guest callbacks never retain it.
        guest::set_transport(Some(Rc::new(move |id, bytes| {
            let mut out = Writer::new();
            unsafe { boundary::serve(id, pointer, bytes, &mut out) }.unwrap();
            out.into_bytes()
        })));
        let mut input = Writer::new();
        if let Some(handler) = handler {
            input.put(&handler);
        }
        encode(&mut input);
        let at = guest::input(input.bytes().len());
        unsafe {
            std::ptr::copy_nonoverlapping(input.bytes().as_ptr(), at, input.bytes().len());
        }
        let (at, len) = guest::run(id);
        guest::set_transport(None);
        if len == u32::MAX {
            return None;
        }
        let bytes = unsafe { std::slice::from_raw_parts(at, len as usize) };
        let bytes = if self.corrupt {
            &bytes[..bytes.len().saturating_sub(1)]
        } else {
            bytes
        };
        decode(&mut Reader::new(bytes)).ok()
    }
}
struct Context;
impl LoopContext for Context {
    fn side(&self) -> mod_api::LoopSide {
        mod_api::LoopSide::Server
    }
    fn registry(&self, _: &str) -> Option<Arc<[String]>> {
        None
    }
    fn setting(&self, _: &str, _: &str) -> Option<String> {
        None
    }
    fn mods(&self) -> mod_api::LoadedMods {
        unreachable!("not used by this fixture")
    }
}
#[unsafe(no_mangle)]
fn __horizon_mod_hooks() -> Box<dyn Hooks> {
    struct Mod;
    impl Hooks for Mod {
        fn subscribed(&self) -> &'static [Option<mod_api::HookId>] {
            &[]
        }
        fn living_jump_power(
            &self,
            context: &dyn LoopContext,
            event: &mut mod_api::LivingJumpPowerEvent<'_>,
        ) {
            // This is a plain import made through the generated Guest, not a stubbed reply.
            assert!(matches!(context.side(), mod_api::LoopSide::Server));
            event.entity = 99;
            event.power = 7.25;
            event.kind = "must not write back";
        }
    }
    Box::new(Mod)
}
#[derive(Debug)]
struct Piece;
impl StructurePiece for Piece {
    fn bounding_box(&self) -> mod_api::BoundingBox {
        mod_api::BoundingBox::new([1, 2, 3], [4, 5, 6])
    }
    fn post_process(
        &self,
        _: &mut dyn mod_api::PieceContext,
    ) -> Result<(), mod_api::StructureError> {
        Ok(())
    }
    fn add_additional_save_data(&self, _: &mut mod_api::Compound) {}
}
#[test]
fn wire_loopback_owner_thread_guards_access_and_drop() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Local {
        _local: Rc<()>,
        drops: Arc<AtomicUsize>,
    }
    impl Drop for Local {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::SeqCst);
        }
    }
    let drops = Arc::new(AtomicUsize::new(0));
    let local = boundary::OwnerThread::new(Local {
        _local: Rc::new(()),
        drops: drops.clone(),
    });
    assert!(local.get().is_ok());
    std::thread::spawn(move || {
        assert!(matches!(
            local.get(),
            Err(boundary::ServeError::WrongThread)
        ));
        drop(local);
    })
    .join()
    .unwrap();
    assert_eq!(
        drops.load(Ordering::SeqCst),
        0,
        "foreign-thread destruction must leak the thread-affine value"
    );
    drop(boundary::OwnerThread::new(Local {
        _local: Rc::new(()),
        drops: drops.clone(),
    }));
    assert_eq!(
        drops.load(Ordering::SeqCst),
        1,
        "creator-thread destruction must release the value"
    );
}
#[test]
fn wire_loopback_roundtrip_and_atomic_writeback() {
    let module = WasmMod {
        subscribed: &[],
        corrupt: false,
    };
    let mut event = mod_api::LivingJumpPowerEvent {
        entity: 42,
        kind: "original",
        power: 1.5,
    };
    module.living_jump_power(&Context, &mut event);
    assert_eq!(event.power, 7.25);
    assert_eq!(event.kind, "original");
    let bad = WasmMod {
        subscribed: &[],
        corrupt: true,
    };
    event.entity = 42;
    event.power = 2.0;
    bad.living_jump_power(&Context, &mut event);
    assert_eq!(
        event.power, 2.0,
        "a truncated last field must leave every host field untouched"
    );
    assert_eq!(event.entity, 42);
    let id = guest::add_handler(vec![guest::view::<dyn StructurePiece>(Arc::new(Piece))]);
    let proxy = WasmHandler { index: id, module };
    assert_eq!(
        proxy.bounding_box(),
        mod_api::BoundingBox::new([1, 2, 3], [4, 5, 6])
    );
    let mut release = Writer::new();
    release.put(&vec![id]);
    assert_eq!(
        proxy.module.call(
            CallClass::Load,
            boundary::Frame::default(),
            mod_api::wire::RELEASE,
            None,
            |w| w.put_bytes(release.bytes()),
            |r| r.take::<()>()
        ),
        Some(())
    );
    assert!(
        proxy
            .module
            .call::<mod_api::BoundingBox>(
                CallClass::Worker,
                boundary::Frame::default(),
                boundary::export_id("StructurePiece", "bounding_box").unwrap(),
                Some(id),
                |_| {},
                |r| r.take()
            )
            .is_none(),
        "released handler must be refused"
    );
}
