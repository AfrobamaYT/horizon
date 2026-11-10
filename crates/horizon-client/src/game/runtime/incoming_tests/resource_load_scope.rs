//! The loader's worker must retain the owning test installation, not an empty TLS context.
use super::*;
use std::sync::Arc;

struct Owner;
impl mod_api::Hooks for Owner {
    fn subscribed(&self) -> &'static [Option<mod_api::HookId>] {
        &[]
    }
    fn register(&self, registrar: &mut dyn mod_api::Registrar, _: &mod_api::RegistrationContext) {
        registrar.block_key("minecraft:stone");
    }
}

#[test]
#[ignore = "requires HORIZON_TEST_RESOURCES pointing to prepared vanilla resources"]
fn loader_workers_keep_owning_mod_sets_and_share_only_matching_numeric_payloads() {
    let prepared = std::path::PathBuf::from(
        std::env::var_os("HORIZON_TEST_RESOURCES").expect("prepared resources are required"),
    );
    let scratch = tempfile::tempdir().unwrap();
    // The real loader needs these three inputs. Cache writes must never touch a shared root.
    for name in ["horizon-resources.json", "facts.json", "block-tags.json"] {
        std::fs::copy(prepared.join(name), scratch.path().join(name)).unwrap();
    }
    let first: mod_api::Mods = Arc::from([mod_api::ModEntry::new("loader:first", Box::new(Owner))]);
    let second: mod_api::Mods =
        Arc::from([mod_api::ModEntry::new("loader:second", Box::new(Owner))]);
    let root = scratch.path();
    let loaded: Vec<_> = std::thread::scope(|owners| {
        [first.clone(), first, second]
            .into_iter()
            .map(|mods| {
                owners.spawn(move || {
                    mod_api::testing::with_mods(mods, || {
                        mod_api::__private::part_tests::install_parts_fixture(&[]).unwrap();
                        let ids: Vec<_> = mod_api::__private::installed_code_ids().collect();
                        let stone = mod_api::__private::part_tests::block_key("minecraft:stone");
                        std::thread::scope(|workers| {
                            // A plain spawn loses the test-local installation.
                            assert!(
                                workers
                                    .spawn(|| mod_api::__private::mods().is_empty())
                                    .join()
                                    .unwrap()
                            );
                            named_scope(workers, "installation-test", move || {
                                assert_eq!(
                                    mod_api::__private::installed_code_ids().collect::<Vec<_>>(),
                                    ids
                                );
                                assert_eq!(
                                    mod_api::__private::part_tests::block_key("minecraft:stone"),
                                    stone
                                );
                                horizon_client::assets::Assets::load(root).unwrap().registry
                            })
                            .join()
                            .unwrap()
                        })
                    })
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|owner| owner.join().unwrap())
            .collect()
    });
    // Different Parts epochs keep different worker owners, even with the same Mods Arc.
    assert!(!std::ptr::eq(loaded[0], loaded[1]));
    let state = |registry: &'static horizon_world::registry::BlockRegistry| {
        registry
            .state(registry.default_state("minecraft:stone").unwrap())
            .unwrap()
    };
    // References to the same immutable Vec element prove payload sharing without a new API.
    assert!(std::ptr::eq(state(loaded[0]), state(loaded[1])));
    assert!(!std::ptr::eq(state(loaded[0]), state(loaded[2])));
}
