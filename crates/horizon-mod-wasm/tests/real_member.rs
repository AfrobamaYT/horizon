//! The build-tool fixture is a real shared module, not a component.
use horizon_mod_package::manifest::Package;
use horizon_mod_wasm::link::{Group, Member};
use wasmtime::{Engine, Val};

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
}
