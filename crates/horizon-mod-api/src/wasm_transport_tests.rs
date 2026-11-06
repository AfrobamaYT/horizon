//! Exercise stock Wasmtime list cursors through a real canonical resource call.
use super::*;
use std::alloc::{GlobalAlloc, Layout, System};
use wasmtime::AsContextMut;
use wasmtime::component::{Resource, ResourceType, WasmList};

thread_local! {
    static COUNT: Cell<Option<usize>> = const { Cell::new(None) };
}
struct Allocator;
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
fn allocation() {
    let _ = COUNT.try_with(|count| {
        if let Some(n) = count.get() {
            count.set(Some(n + 1));
        }
    });
}
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        allocation();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        allocation();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        allocation();
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

fn store(engine: &wasmtime::Engine) -> Store<State> {
    Store::new(
        engine,
        State {
            limits: wasmtime::StoreLimitsBuilder::new().build(),
            buffers: HashMap::new(),
        },
    )
}

#[test]
fn generated_draw_list_reuses_transport() {
    struct Draw;
    impl crate::WorldDrawContext for Draw {
        fn lines(&mut self, lines: &[crate::WorldLine]) -> Result<(), crate::DrawError> {
            assert_eq!(lines.len(), 8);
            assert_eq!(lines[0].from, [1.0, 2.0, 3.0]);
            Ok(())
        }
    }
    let engine = wasmtime::Engine::default();
    let source = r#"(component
      (import "horizon:mod-api/world-draw-context@0.1.0" (instance $api
        (type $depth (enum "tested" "through-walls"))
        (type $point (tuple float64 float64 float64))
        (type $line (record (field "from" $point) (field "to" $point) (field "color" u32) (field "width" float32) (field "depth" $depth)))
        (type $lines (list $line))
        (type $error (enum "non-finite" "invalid-width" "capacity-exceeded"))
        (export "lines" (func (param "lines" $lines) (result (result (error $error)))))))
      (alias export $api "lines" (func $lines))
      (core module $memory (memory (export "memory") 1)
        (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 8192))
      (core instance $mem (instantiate $memory))
      (alias core export $mem "memory" (core memory $memory))
      (alias core export $mem "realloc" (core func $realloc))
      (core func $lower (canon lower (func $lines) (memory $memory) (realloc $realloc)))
      (core instance $imports (export "lines" (func $lower)))
      (core module $runner (import "imports" "lines" (func $lines (param i32 i32 i32)))
        (func (export "run") (param i32 i32) (result i32)
          local.get 0 local.get 1 i32.const 32 call $lines i32.const 32))
      (core instance $run (instantiate $runner (with "imports" (instance $imports))))
      (alias core export $run "run" (core func $run))
      (type $depth (enum "tested" "through-walls"))
      (type $point (tuple float64 float64 float64))
      (type $line (record (field "from" $point) (field "to" $point) (field "color" u32) (field "width" float32) (field "depth" $depth)))
      (type $lines (list $line))
      (type $error (enum "non-finite" "invalid-width" "capacity-exceeded"))
      (func (export "run") (param "lines" $lines) (result (result (error $error)))
        (canon lift (core func $run) (memory $memory) (realloc $realloc))))"#;
    let component = Component::new(&engine, wat::parse_str(source).unwrap()).unwrap();
    let mut linker = Linker::<State>::new(&engine);
    glue::add_flat_lists(&mut linker).unwrap();
    let mut store = store(&engine);
    let instance = linker.instantiate(&mut store, &component).unwrap();
    use glue::horizon::mod_api::types::{DrawError, WorldDepth, WorldLine};
    let call = instance
        .get_typed_func::<(&[WorldLine],), (Result<(), DrawError>,)>(&mut store, "run")
        .unwrap();
    let values: Vec<_> = (0..8)
        .map(|_| WorldLine {
            from: (1.0, 2.0, 3.0),
            to: (4.0, 5.0, 6.0),
            color: 1,
            width: 1.0,
            depth: WorldDepth::Tested,
        })
        .collect();
    let mut draw = Draw;
    let mut frame = Frame::default();
    frame.world_draw_context_mut(&mut draw);
    let _scope = CallScope {
        frame: CURRENT.replace(&mut frame),
        caller: CALLER.replace(None),
    };
    for _ in 0..10 {
        assert!(call.call(&mut store, (&values,)).unwrap().0.is_ok());
    }
    COUNT.set(Some(0));
    for _ in 0..1000 {
        assert!(call.call(&mut store, (&values,)).unwrap().0.is_ok());
    }
    let count = COUNT.replace(None).unwrap();
    eprintln!("generated draw transport: {count} allocations per 1000 calls");
}

#[test]
fn resource_list_transport_reuses_host_buffers() {
    struct Node;
    let engine = wasmtime::Engine::default();
    let component = Component::new(&engine, wat::parse_str(r#"
        (component
          (import "test" (instance $host
            (export "node" (type $node (sub resource)))
            (type $list (list u32))
            (export "[method]node.echo" (func (param "self" (borrow $node)) (param "values" $list) (result $list)))))
          (alias export $host "node" (type $node))
          (alias export $host "[method]node.echo" (func $echo))
          (core module $memory
            (memory (export "memory") 1)
            (func (export "realloc") (param i32 i32 i32 i32) (result i32) i32.const 8192))
          (core instance $mem (instantiate $memory))
          (alias core export $mem "memory" (core memory $memory))
          (alias core export $mem "realloc" (core func $realloc))
          (core func $lower (canon lower (func $echo) (memory $memory) (realloc $realloc)))
          (core func $drop (canon resource.drop $node))
          (core instance $imports (export "echo" (func $lower)) (export "drop" (func $drop)))
          (core module $runner
            (import "imports" "echo" (func $echo (param i32 i32 i32 i32)))
            (import "imports" "drop" (func $drop (param i32)))
            (import "mem" "memory" (memory 1))
            (func (export "run") (param i32 i32 i32) (result i32)
              local.get 0 local.get 1 local.get 2 i32.const 32 call $echo
              local.get 0 call $drop i32.const 32))
          (core instance $run (instantiate $runner (with "imports" (instance $imports)) (with "mem" (instance $mem))))
          (alias core export $run "run" (core func $run))
          (type $list (list u32))
          (func (export "run") (param "node" (borrow $node)) (param "values" $list) (result $list)
            (canon lift (core func $run) (memory $memory) (realloc $realloc))))
    "#).unwrap()).unwrap();
    let mut counts = [0; 3];
    for route in 0..3 {
        let mut linker = Linker::<State>::new(&engine);
        let mut host = linker.instance("test").unwrap();
        host.resource("node", ResourceType::host::<Node>(), |_, _| Ok(()))
            .unwrap();
        if route != 0 {
            let empty = Arc::new(Vec::<u32>::new());
            host.func_wrap(
                "[method]node.echo",
                move |store, (node, values): (Resource<Node>, WasmList<u32>)| {
                    assert_eq!(node.rep(), 7);
                    if route == 1 {
                        return Ok((empty.clone(),));
                    }
                    State::with_buffer::<Arc<Vec<u32>>, _>(store, |buffer, store| {
                        let output = Arc::get_mut(buffer).unwrap();
                        output.clear();
                        output.extend_from_slice(values.as_le_slice(&store));
                        for value in output {
                            *value = u32::from_le(*value) + 1;
                        }
                        Ok((buffer.clone(),))
                    })
                },
            )
            .unwrap();
        } else {
            host.func_wrap(
                "[method]node.echo",
                |_, (node, values): (Resource<Node>, Vec<u32>)| {
                    assert_eq!(node.rep(), 7);
                    Ok((values.iter().map(|n| n + 1).collect::<Vec<_>>(),))
                },
            )
            .unwrap();
        }
        let mut store = store(&engine);
        let instance = linker.instantiate(&mut store, &component).unwrap();
        let call = instance
            .get_typed_func::<(Resource<Node>, &[u32]), (WasmList<u32>,)>(&mut store, "run")
            .unwrap();
        let values: Vec<u32> = (0..1000).collect();
        let mut run = || {
            let (result,) = call
                .call(&mut store, (Resource::new_borrow(7), &values))
                .unwrap();
            if route != 1 {
                for (i, value) in result.iter(store.as_context_mut()).unwrap().enumerate() {
                    assert_eq!(value.unwrap(), i as u32 + 1);
                }
            }
        };
        for _ in 0..10 {
            run();
        }
        COUNT.set(Some(0));
        let start = Instant::now();
        const CALLS: usize = 100_000;
        for _ in 0..CALLS {
            run();
        }
        let elapsed = start.elapsed();
        let allocations = COUNT.replace(None).unwrap();
        counts[route] = allocations;
        eprintln!(
            "1,000 values each direction (route = {route}): {} ns/call, {allocations} host allocations / {CALLS} calls; guest fixture uses two fixed-buffer canonical realloc calls per round trip",
            elapsed.as_nanos() / CALLS as u128
        );
    }
    assert_eq!(
        counts[0] - counts[1],
        2 * 100_000,
        "owned list transport allocations"
    );
    assert_eq!(
        counts[2], counts[1],
        "reused list transport adds no allocations"
    );
}

#[test]
fn scratch_is_store_local_and_restored_after_unwinding() {
    let engine = wasmtime::Engine::default();
    let mut first = store(&engine);
    let mut second = store(&engine);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        State::with_buffer::<Vec<u32>, _>(first.as_context_mut(), |buffer, _| {
            buffer.push(17);
            panic!("test trap");
        });
    }));
    State::with_buffer::<Vec<u32>, _>(first.as_context_mut(), |buffer, _| {
        assert_eq!(buffer, &[17])
    });
    State::with_buffer::<Vec<u32>, _>(second.as_context_mut(), |buffer, _| {
        assert!(buffer.is_empty())
    });
}
