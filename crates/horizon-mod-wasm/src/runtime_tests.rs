use super::*;

fn guest(layout: u64, body: &str) -> Vec<u8> {
    wat::parse_str(format!(
        r#"(module
      (@custom "dylink.0" "\01\04\00\00\00\00")
      (import "env" "memory" (memory 1))
      (func (export "horizon_layout") (result i64) i64.const {layout})
      (func (export "horizon_in") (param i32) (result i32) i32.const 512)
      (func (export "horizon_hook") (param i32 i32 i32) (result i64) {body})
    )"#
    ))
    .unwrap()
}
fn module(body: &str) -> WasmMod {
    let engine = engine().unwrap();
    let bytes = guest(mod_api::wire::LAYOUT, body);
    WasmMod::instantiate(
        &engine,
        &[Member {
            name: "test",
            module: &bytes,
        }],
    )
    .unwrap()
    .remove(0)
}
fn call(module: &WasmMod) -> Option<u32> {
    module.call(
        CallClass::Hook,
        Frame::default(),
        0,
        None,
        |w| w.put(&42u32),
        |r| r.take(),
    )
}
#[test]
fn hook_round_trip_and_malformed_reply_poison() {
    // Echo the four-byte input from horizon_in.
    let module = module("i64.const 2199023255556"); // 512 << 32 | 4
    assert_eq!(call(&module), Some(42));
    assert!(!module.running.poisoned.load(Ordering::Relaxed));
    let module = super::runtime_tests::module("i64.const -4294967292"); // ptr = u32::MAX
    assert_eq!(call(&module), None);
    assert!(module.running.poisoned.load(Ordering::Relaxed));
    assert_eq!(call(&module), None);
}
#[test]
fn trap_and_deadline_stop_the_group() {
    let module = module("unreachable");
    assert_eq!(call(&module), None);
    assert!(module.running.poisoned.load(Ordering::Relaxed));
    let module = super::runtime_tests::module("(loop br 0) unreachable");
    let started = Instant::now();
    assert_eq!(call(&module), None);
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(module.running.poisoned.load(Ordering::Relaxed));
}
#[test]
fn layout_mismatch_is_a_mods_page_refusal() {
    let engine = engine().unwrap();
    let bytes = guest(mod_api::wire::LAYOUT ^ 1, "i64.const 0");
    let refusal = WasmMod::instantiate(
        &engine,
        &[Member {
            name: "test",
            module: &bytes,
        }],
    )
    .err()
    .unwrap();
    assert!(matches!(refusal, Refusal::Layout { .. }));
    assert_eq!(
        refusal.to_string(),
        "built for another version of the mod API"
    );
}
#[test]
fn callback_borrows_only_the_lent_context_and_restores_depth() {
    let module = module("i64.const 2199023255556");
    let mut group = module.running.group.lock().unwrap();
    group.store.set_epoch_deadline(100);
    group.store.data_mut().depth = 1;
    group.store.data_mut().cx.frames.push(Frame::default());
    let mut context = group.store.as_context_mut();
    let loan = Loan {
        group: Arc::as_ptr(&module.running) as usize,
        context: Cell::new((&mut context as *mut StoreContextMut<'_, State>).cast()),
        previous: LENT.get(),
    };
    LENT.set(&loan);
    assert_eq!(
        module.call(
            CallClass::Callback,
            Frame::default(),
            0,
            None,
            |w| w.put(&99u32),
            |r| r.take::<u32>()
        ),
        Some(99)
    );
    drop(loan);
    assert_eq!(context.data().depth, 1);
    assert_eq!(context.data().cx.frames.len(), 1);
}
