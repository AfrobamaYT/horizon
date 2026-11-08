use super::*;

fn guest(layout: u64, body: &str) -> Vec<u8> {
    wat::parse_str(format!(
        r#"(module
      (@custom "dylink.0" "\01\04\00\00\00\00")
      (import "env" "memory" (memory 1))
      (import "env" "__indirect_function_table" (table 1 funcref))
      (import "horizon" "host" (func $host (param i32 i32 i32 i32 i32) (result i32)))
      (import "horizon" "host_reply" (func $reply (param i32)))
      (func (export "horizon_layout") (result i64) i64.const {layout})
      (global $in_len (mut i32) (i32.const 0))
      (func (export "horizon_in") (param i32) (result i32) local.get 0 global.set $in_len i32.const 512)
      (func (export "horizon_hook") (param i32) (result i64) (local i32 i32)
        i32.const 512 local.set 1 global.get $in_len local.set 2 {body})
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
fn a_member_cannot_bypass_group_limits_with_private_memories() {
    let bytes =
        wat::parse_str(r#"(module (@custom "dylink.0" "\01\04\00\00\00\00") (memory 1))"#).unwrap();
    let refusal = WasmMod::instantiate(
        &engine().unwrap(),
        &[Member {
            name: "private",
            module: &bytes,
        }],
    )
    .err()
    .unwrap();
    assert!(refusal.to_string().contains("must import the group memory"));
}

#[test]
fn real_members_link_when_built() {
    let Some(dir) = std::env::var_os("HORIZON_WASM_TEST_MEMBERS") else {
        return;
    };
    let dir = std::path::Path::new(&dir);
    let (Ok(std), Ok(fixture)) = (
        std::fs::read(dir.join("std.wasm")),
        std::fs::read(dir.join("fixture.wasm")),
    ) else {
        return;
    };
    let engine = engine().unwrap();
    let modules = WasmMod::instantiate(
        &engine,
        &[
            Member {
                name: "std",
                module: &std,
            },
            Member {
                name: "fixture",
                module: &fixture,
            },
        ],
    );
    let modules = modules.unwrap_or_else(|why| panic!("real shared members did not load: {why}"));
    assert_eq!(modules.len(), 1);
    let subscriptions: Vec<String> = modules[0]
        .call(
            CallClass::Load,
            Frame::default(),
            crate::boundary::SUBSCRIBED,
            None,
            |_| {},
            |r| r.take(),
        )
        .expect("real guest subscription hook failed");
    assert!(
        subscriptions
            .iter()
            .all(|name| crate::boundary::hook_id(name).is_some())
    );
}

#[test]
fn unresolved_namespaced_functions_are_not_stubbed() {
    let bytes = wat::parse_str(r#"(module (@custom "dylink.0" "\01\04\00\00\00\00") (import "env" "fixture:missing" (func)))"#).unwrap();
    let error = Group::link(
        &wasmtime::Engine::default(),
        &[Member {
            name: "test",
            module: &bytes,
        }],
    )
    .err()
    .unwrap();
    assert!(
        matches!(error, crate::link::LinkError::Unresolved { symbol, .. } if symbol == "env.fixture:missing")
    );
}

#[test]
fn linker_heap_symbols_follow_all_members_static_data() {
    let first = wat::parse_str(r#"(module (@custom "dylink.0" "\01\04\10\00\00\00"))"#).unwrap();
    let second = wat::parse_str(
        r#"(module (@custom "dylink.0" "\01\04\20\00\00\00")
        (import "GOT.mem" "__heap_base" (global $heap (mut i32)))
        (import "GOT.mem" "__data_end" (global $end (mut i32)))
        (import "GOT.mem" "__heap_end" (global $limit (mut i32)))
        (func (export "heap") (result i32) global.get $heap)
        (func (export "data") (result i32) global.get $end)
        (func (export "limit") (result i32) global.get $limit))"#,
    )
    .unwrap();
    let engine = wasmtime::Engine::default();
    let mut group = Group::link(
        &engine,
        &[
            Member {
                name: "first",
                module: &first,
            },
            Member {
                name: "second",
                module: &second,
            },
        ],
    )
    .unwrap();
    for symbol in ["heap", "data"] {
        let mut output = [wasmtime::Val::I32(0)];
        group.call("second", symbol, &[], &mut output).unwrap();
        assert_eq!(output[0].i32(), Some(262192)); // 256 KiB stack + 16 + 32 bytes.
    }
    let mut output = [wasmtime::Val::I32(0)];
    group.call("second", "limit", &[], &mut output).unwrap();
    assert_eq!(output[0].i32(), Some(327680)); // Five initial memory pages.
}

#[test]
fn boundary_exports_are_per_member_but_other_horizon_symbols_are_not() {
    let bytes = guest(mod_api::wire::LAYOUT, "i64.const 0");
    let engine = engine().unwrap();
    let modules = WasmMod::instantiate(
        &engine,
        &[
            Member {
                name: "first",
                module: &bytes,
            },
            Member {
                name: "second",
                module: &bytes,
            },
        ],
    )
    .unwrap();
    assert_eq!(modules.len(), 2);
    let bytes = wat::parse_str(
        r#"(module (@custom "dylink.0" "\01\04\00\00\00\00") (func (export "horizon_shared")))"#,
    )
    .unwrap();
    let refusal = WasmMod::instantiate(
        &engine,
        &[
            Member {
                name: "first",
                module: &bytes,
            },
            Member {
                name: "second",
                module: &bytes,
            },
        ],
    )
    .err()
    .unwrap();
    assert!(
        refusal
            .to_string()
            .contains("exports horizon_shared, which first already does")
    );
}

#[test]
fn loader_initializes_and_installs_a_wasm_entry() {
    use mod_api::__private::game::Fields;
    let std = wat::parse_str(
        r#"(module (@custom "dylink.0" "\01\04\00\00\00\00") (import "env" "memory" (memory 1)))"#,
    )
    .unwrap();
    let module = guest(
        mod_api::wire::LAYOUT,
        r#"local.get 0 i32.const -256 i32.eq if (result i64)
            i32.const 4096 local.get 2 i32.store
            i32.const 2048 local.get 1 local.get 2 memory.copy
            i64.const 0
          else local.get 0 i32.const 2 i32.eq if (result i64)
            i64.const 8796093022208 i32.const 4096 i32.load i64.extend_i32_u i64.or
          else i64.const 4398046511108 end end"#,
    );
    let manifest = horizon_mod_package::manifest::Manifest::parse(&format!("id = 'roundtrip'\nname = 'Roundtrip'\nversion = '1.0.0'\napi = '0.2'\ncapabilities = ['registries']\n[wasm]\nsha512 = '{}'\ntoolchain = 'test'\nbuilt_with = 'test'\n", "a".repeat(128))).unwrap();
    let loaded = crate::load(&std, vec![crate::WasmPackage { manifest, module }]);
    assert!(loaded.refused.is_empty(), "{:?}", loaded.refused);
    assert_eq!(loaded.mods.len(), 1);
    assert_eq!(loaded.mods[0].fields().id, "roundtrip");
    assert_eq!(loaded.manifests[0].id, "roundtrip");
    let inventory: Vec<_> = ["native", "roundtrip", "content"]
        .into_iter()
        .map(|id| {
            mod_api::PackageInfo::new(
                id.into(),
                "1.0.0".into(),
                mod_api::PackageKind::Code,
                mod_api::Side::Both,
                vec![id.into()],
                Vec::new(),
            )
        })
        .collect();
    crate::initialize_modules(&loaded.modules, &inventory);
    let echoed: Vec<mod_api::PackageInfo> = loaded.modules[0]
        .call(
            CallClass::Hook,
            Frame::default(),
            2,
            None,
            |_| {},
            |r| r.take(),
        )
        .unwrap();
    assert_eq!(
        echoed.iter().map(|p| p.id()).collect::<Vec<_>>(),
        ["native", "roundtrip", "content"]
    );
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
          i32.const 0 i64.load i64.const 0 i64.ne if unreachable end
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
fn dropping_a_handler_releases_its_id_on_the_next_call() {
    let module = module(
        r#"
        local.get 0 i32.const -255 i32.eq
        if (result i64)
          local.get 1 i32.load i32.const 1 i32.ne if unreachable end
          local.get 1 i64.load offset=4 i64.const 123 i64.ne if unreachable end
          i32.const 1024 i32.const 1 i32.store
          i64.const 0
        else
          i64.const 4398046511108
        end"#,
    );
    assert_eq!(call(&module), Some(0));
    drop(WasmHandler {
        module: module.clone(),
        index: Id(123),
    });
    assert_eq!(call(&module), Some(1));
    assert!(!module.running.poisoned.load(Ordering::Relaxed));
}

#[test]
fn memory_growth_limit_traps_and_poison_is_shared_by_proxies() {
    let module = module("i32.const 4096 memory.grow drop i64.const 0");
    let other = module.clone();
    assert_eq!(call(&module), None);
    assert!(other.running.poisoned.load(Ordering::Relaxed));
    assert_eq!(call(&other), None);
    let module =
        super::runtime_tests::module("ref.null func i32.const 1048576 table.grow drop i64.const 0");
    assert_eq!(call(&module), None);
    assert!(module.running.poisoned.load(Ordering::Relaxed));
}
