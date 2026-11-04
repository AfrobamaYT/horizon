//! The modding book's wasm sample, built by `cargo horizon-mod build --wasm`, loaded as the game
//! loads it. CI builds the sample first and points `HORIZON_WASM_SAMPLE` at its `.hmod` and
//! `HORIZON_WASM_STD` at the std member; the test is ignored elsewhere.

#[test]
#[ignore = "needs the sample built to mod.wasm (CI's wasm-mods job)"]
fn the_sample_mod_loads_and_subscribes_to_its_hook() {
    let path = |var: &str| std::path::PathBuf::from(std::env::var(var).expect(var));
    let package = path("HORIZON_WASM_SAMPLE");
    let std_member = std::fs::read(path("HORIZON_WASM_STD")).unwrap();
    let manifest = horizon_mods::Package::read(&package).unwrap().manifest;
    let module = horizon_mods::Package::read_wasm(&package).unwrap();
    let loaded = horizon_mod_wasm::load(
        &horizon_mod_wasm::RuntimeConfig::default(),
        &std_member,
        vec![horizon_mod_wasm::WasmPackage { manifest, module }],
    );
    assert!(loaded.refused.is_empty(), "{:?}", loaded.refused);
    let [entry] = loaded.mods.as_slice() else {
        panic!("one mod");
    };
    use mod_api::__private::game::Fields;
    let subscribed = entry.fields().hooks.subscribed();
    assert!(
        subscribed
            .iter()
            .any(|hook| matches!(hook, Some(mod_api::HookId::ServerChat))),
        "the sample listens to server_chat"
    );
}
