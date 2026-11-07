use super::*;

fn guest(layout: u64, body: &str) -> Vec<u8> {
    wat::parse_str(format!(
        r#"(module
      (@custom "dylink.0" "\01\04\00\00\00\00")
      (import "env" "memory" (memory 1))
      (import "horizon" "host" (func $host (param i32 i32 i32 i32 i32) (result i32)))
      (import "horizon" "host_reply" (func $reply (param i32)))
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
fn invalid_import_input_is_a_trap_not_a_host_panic() {
    let module = module(
        "i32.const -1 i32.const -1 i32.const 16 i32.const 0 i32.const 0 call $host drop i64.const 0",
    );
    assert_eq!(call(&module), None);
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

#[test]
fn import_reply_retry_survives_a_nested_callback() {
    struct Random(WasmMod);
    impl mod_api::StructureRandom for Random {
        fn next_int_bound(&mut self, _: i32) -> i32 {
            0
        }
        fn next_float(&mut self) -> f32 {
            0.0
        }
        fn next_double(&mut self) -> f64 {
            0.0
        }
        fn next_boolean(&mut self) -> bool {
            false
        }
        fn next_long(&mut self) -> i64 {
            self.0
                .call(
                    CallClass::Callback,
                    Frame::default(),
                    1,
                    None,
                    |w| w.put(&70i64),
                    |r| r.take::<i64>(),
                )
                .unwrap_or(-1)
                + 1
        }
    }
    let id = crate::boundary::import_id("StructureRandom", "next_long").unwrap();
    let module = module(&format!(
        r#"
        local.get 0 i32.const 1 i32.eq
        if (result i64)
          i64.const 2199023255560
        else
          i32.const {id} i32.const 0 i32.const 0 i32.const 0 i32.const 0 call $host
          i32.const 8 i32.ne if unreachable end
          i32.const 1024 call $reply
          i64.const 4398046511112
        end"#
    ));
    let mut random = Random(module.clone());
    let mut frame = Frame::default();
    frame.set_structure_random_mut(&mut random);
    assert_eq!(
        module.call(CallClass::Hook, frame, 0, None, |_| {}, |r| r.take::<i64>()),
        Some(71)
    );
}

#[test]
fn memory_growth_limit_traps_and_poison_is_shared_by_proxies() {
    let module = module("i32.const 4096 memory.grow drop i64.const 0");
    let other = module.clone();
    assert_eq!(call(&module), None);
    assert!(other.running.poisoned.load(Ordering::Relaxed));
    assert_eq!(call(&other), None);
}
