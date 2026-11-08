//! The build-tool fixture is a real shared module, not a component.
use horizon_mod_package::manifest::Package;
use horizon_mod_wasm::link::{Group, Member};
use wasmtime::{Engine, Val};

#[test]
fn dump_members() {
    for (name, path) in [
        ("std", "HORIZON_WASM_STD"),
        ("fixture", "HORIZON_WASM_FIXTURE_RAW"),
    ] {
        let Ok(path) = std::env::var(path) else {
            return;
        };
        let bytes = std::fs::read(path).unwrap();
        println!("=== {name} ===");
        for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
            match payload.unwrap() {
                wasmparser::Payload::CustomSection(s) if s.name() == "dylink.0" => {
                    println!("dylink.0: {:02x?}", s.data())
                }
                wasmparser::Payload::ImportSection(s) => {
                    for i in s.into_imports() {
                        println!("import: {:?}", i.unwrap());
                    }
                }
                wasmparser::Payload::ExportSection(s) => {
                    for e in s {
                        println!("export: {:?}", e.unwrap());
                    }
                }
                _ => {}
            }
        }
    }
}

#[test]
fn real_member() {
    let (Ok(fixture), Ok(std)) = (
        std::env::var("HORIZON_WASM_FIXTURE"),
        std::env::var("HORIZON_WASM_STD"),
    ) else {
        eprintln!("skipping: HORIZON_WASM_FIXTURE and HORIZON_WASM_STD are not set");
        return;
    };
    let module = Package::read_wasm(std::path::Path::new(&fixture)).unwrap();
    let std = std::fs::read(std).unwrap();
    let mut group = Group::link(
        &Engine::default(),
        &[
            Member {
                name: "std",
                module: &std,
            },
            Member {
                name: "fixture",
                module: &module,
            },
        ],
    )
    .unwrap();
    let mut result = [Val::I64(0)];
    group
        .call("fixture", "horizon_layout", &[], &mut result)
        .unwrap();
    assert_ne!(result[0].unwrap_i64(), 0);
    // Hooks::subscribed has no arguments. An empty fixture returns an empty vector.
    group
        .call("fixture", "horizon_in", &[Val::I32(0)], &mut [Val::I32(0)])
        .unwrap();
    let mut reply = [Val::I64(0)];
    group
        .call("fixture", "horizon_hook", &[Val::I32(0)], &mut reply)
        .unwrap();
    assert_eq!(reply[0].unwrap_i64() as u32, 4);
}
