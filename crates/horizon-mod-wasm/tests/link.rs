//! The group linker on small shared modules written by hand: one std, one library and one mod,
//! the way `rust-lld --shared` lays them out.
use horizon_mod_wasm::link::{Group, LinkError, Member};
use wasmtime::{Engine, Val};

/// std: one static counter, and the function that bumps it.
const STD: &str = r#"(module
  (@custom "dylink.0" "\01\04\04\02\00\00")
  (import "env" "memory" (memory 1))
  (import "env" "__memory_base" (global $base i32))
  (data (global.get $base) "\00\00\00\00")
  (func (export "bump") (result i32)
    (i32.store (global.get $base) (i32.add (i32.load (global.get $base)) (i32.const 1)))
    (i32.load (global.get $base)))
  (global (export "ALLOCS") i32 (i32.const 0)))"#;

/// A library: a function, and a static that starts at 40.
const LIB: &str = r#"(module
  (@custom "dylink.0" "\01\04\04\02\00\00")
  (import "env" "memory" (memory 1))
  (import "env" "__memory_base" (global $base i32))
  (import "env" "bump" (func $bump (result i32)))
  (data (global.get $base) "\28\00\00\00")
  (func (export "lib_add") (param i32 i32) (result i32)
    (drop (call $bump))
    (i32.add (local.get 0) (local.get 1)))
  (global (export "LIB_COUNTER") i32 (i32.const 0)))"#;

/// A mod calling the library's function and reading its static, and std's counter.
const MOD: &str = r#"(module
  (@custom "dylink.0" "\01\04\00\00\00\00")
  (import "env" "memory" (memory 1))
  (import "env" "lib_add" (func $add (param i32 i32) (result i32)))
  (import "env" "bump" (func $bump (result i32)))
  (import "GOT.mem" "LIB_COUNTER" (global $counter (mut i32)))
  (func (export "run") (result i32)
    (drop (call $bump))
    (i32.add (call $add (i32.const 2) (i32.const 3)) (i32.load (global.get $counter))))
  (func (export "bumps") (result i32) (call $bump)))"#;

fn bytes(text: &str) -> Vec<u8> {
    wat::parse_str(text).unwrap()
}

fn call(group: &mut Group, member: &str, function: &str) -> i32 {
    let mut out = [Val::I32(0)];
    group.call(member, function, &[], &mut out).unwrap();
    out[0].unwrap_i32()
}

#[test]
fn a_mod_calls_its_library_and_reads_its_static_over_one_shared_std() {
    let (std, lib, user) = (bytes(STD), bytes(LIB), bytes(MOD));
    let mut group = Group::link(
        &Engine::default(),
        &[
            Member {
                name: "std",
                module: &std,
            },
            Member {
                name: "lib",
                module: &lib,
            },
            Member {
                name: "mod",
                module: &user,
            },
        ],
    )
    .unwrap();
    // The static's initial 40, and the call's 2 + 3.
    assert_eq!(call(&mut group, "mod", "run"), 45);
    // Three bumps of one counter: the mod's own, the library's, and this one. Two copies of std
    // would have counted separately.
    assert_eq!(call(&mut group, "mod", "bumps"), 3);
}

#[test]
fn an_import_no_earlier_member_has_is_refused_naming_the_member() {
    let (std, user) = (bytes(STD), bytes(MOD));
    let err = Group::link(
        &Engine::default(),
        &[
            Member {
                name: "std",
                module: &std,
            },
            Member {
                name: "mod",
                module: &user,
            },
        ],
    )
    .err()
    .unwrap();
    assert_eq!(
        err,
        LinkError::Unresolved {
            member: "mod".into(),
            symbol: "env.lib_add".into()
        }
    );
}

#[test]
fn two_members_exporting_one_name_are_refused() {
    let std = bytes(STD);
    let err = Group::link(
        &Engine::default(),
        &[
            Member {
                name: "std",
                module: &std,
            },
            Member {
                name: "again",
                module: &std,
            },
        ],
    )
    .err()
    .unwrap();
    assert!(matches!(err, LinkError::Duplicate { ref symbol, .. } if symbol == "bump"));
}

#[test]
fn bytes_that_are_not_a_shared_module_are_refused_without_a_panic() {
    let plain = bytes("(module)");
    let engine = Engine::default();
    let err = Group::link(
        &engine,
        &[Member {
            name: "plain",
            module: &plain,
        }],
    )
    .err();
    assert_eq!(
        err,
        Some(LinkError::NoDylink {
            member: "plain".into()
        })
    );
    for junk in [
        &b""[..],
        b"\0asm",
        b"not wasm at all",
        b"\0asm\x01\0\0\0\x00\x05\x04dyli",
    ] {
        assert!(
            Group::link(
                &engine,
                &[Member {
                    name: "junk",
                    module: junk
                }]
            )
            .is_err()
        );
    }
    // A member that asks for more memory than a group has.
    let huge = bytes(
        r#"(module (@custom "dylink.0" "\01\08\ff\ff\ff\ff\0f\02\00\00") (import "env" "memory" (memory 1)))"#,
    );
    let err = Group::link(
        &engine,
        &[Member {
            name: "huge",
            module: &huge,
        }],
    )
    .err();
    assert_eq!(
        err,
        Some(LinkError::TooLarge {
            member: "huge".into()
        })
    );
}
