//! Real shared member versus the same source natively. Run in release with the fixture and std
//! built first; this measures 1,000 dispatches into fake, in-memory block entities, not a world tick.
use horizon_mod_package::manifest::Package;
use horizon_mod_wasm::{WasmPackage, load};
use mod_api::{Host, ModBlockEntity, ModEntry, ServerHost};
use std::time::Instant;

#[path = "fixture/src/lib.rs"]
mod fixture;

struct World;
impl Host for World {}
mod_api::impl_host_areas!(World);
impl ServerHost for World {}
mod_api::impl_server_host_areas!(World);

fn measure(label: &str, mods: Vec<ModEntry>) {
    mod_api::testing::with_mods(mods.into(), || {
        mod_api::__private::install_parts(
            mod_api::__private::mods(),
            &mod_api::__private::part_tests::rule_schema(),
        )
        .unwrap();
        mod_api::__private::mark_game_thread();
        let mut entities: Vec<_> = (0..1000)
            .map(|_| ModBlockEntity::new("wasm_fixture:counter".into(), Default::default()))
            .collect();
        let mut samples = Vec::with_capacity(100);
        for round in 0..120 {
            let now = Instant::now();
            for entity in &mut entities {
                mod_api::dispatch_block_entity_tick(&mut World, [0, 64, 0], entity);
            }
            let elapsed = now.elapsed().as_nanos();
            if round >= 20 {
                samples.push(elapsed);
            }
        }
        assert!(entities.iter().all(|entity| entity.data == [23]
            && entity.nbt.get(&"counter".into()) == Some(&mod_api::Tag::Int(17))));
        samples.sort_unstable();
        let mean = samples.iter().sum::<u128>() / samples.len() as u128;
        println!(
            "{label}: 1000 entities mean={mean} ns p95={} ns",
            samples[94]
        );
    });
}

#[test]
#[ignore = "measurement; requires a built fixture and shared std"]
fn block_entity_ticks() {
    let path = std::env::var("HORIZON_WASM_FIXTURE").unwrap();
    let path = std::path::Path::new(&path);
    let std = std::fs::read(std::env::var("HORIZON_WASM_STD").unwrap()).unwrap();
    let package = WasmPackage {
        manifest: Package::read(path).unwrap().manifest,
        module: Package::read_wasm(path).unwrap(),
    };
    let loaded = load(&std, vec![package]);
    assert!(loaded.refused.is_empty(), "{:?}", loaded.refused);
    assert_eq!(loaded.mods.len(), 1);
    measure(
        "native",
        vec![ModEntry::new("wasm_fixture", Box::new(fixture::Mod))],
    );
    measure("wasm", loaded.mods);
}
