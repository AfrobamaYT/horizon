use super::*;
use crate::game::{native_entity_refs::tests::with_game, world_loading::WorldRequest};
use horizon_client::mod_api::{
    self, AddServerReloadListenersEvent, Hooks, ModEntry, ServerReloadListener,
};
use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

struct DataListener {
    live: Arc<Mutex<u64>>,
    reject: Arc<AtomicBool>,
}
impl ServerReloadListener for DataListener {
    fn prepare(
        &self,
        context: &ServerReloadContext,
    ) -> Result<ServerReloadApply, ServerReloadError> {
        let id = Identifier::parse("reload_test:machines/value.json").unwrap();
        assert_eq!(context.list().collect::<Vec<_>>(), [&id]);
        // Lookup never turns an arbitrary mod-supplied ID into a filesystem path.
        assert!(
            context
                .read(&Identifier::parse("reload_test:machines/../../private.json").unwrap())
                .is_none()
        );
        let value = serde_json::from_slice::<u64>(context.read(&id).unwrap())
            .map_err(|error| ServerReloadError::new(id.clone(), error))?;
        let live = self.live.clone();
        Ok(ServerReloadApply::new(move || {
            *live.lock().unwrap() = value
        }))
    }
}

struct CheckListener(Arc<AtomicBool>);
impl ServerReloadListener for CheckListener {
    fn prepare(&self, _: &ServerReloadContext) -> Result<ServerReloadApply, ServerReloadError> {
        if self.0.load(Ordering::Relaxed) {
            return Err(ServerReloadError::new(
                Identifier::parse("reload_test:machines/value.json").unwrap(),
                std::io::Error::other("rejected new machine data"),
            ));
        }
        Ok(ServerReloadApply::new(|| {}))
    }
}
impl Hooks for DataListener {
    fn add_server_reload_listeners(&self, event: &mut AddServerReloadListenersEvent<'_>) {
        let id = |text| Identifier::parse(text).unwrap();
        event
            .listeners
            .add_listener(
                id("reload_test:data"),
                id("reload_test:machines/"),
                Arc::new(Self {
                    live: self.live.clone(),
                    reject: self.reject.clone(),
                }),
            )
            .unwrap();
        event
            .listeners
            .add_listener(
                id("reload_test:check"),
                id("reload_test:machines/"),
                Arc::new(CheckListener(self.reject.clone())),
            )
            .unwrap();
        event
            .listeners
            .add_dependency("reload_test:data", "reload_test:check")
            .unwrap();
    }
}

fn write(path: &Path, bytes: impl AsRef<[u8]>) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn request(root: &Path) -> WorldRequest {
    WorldRequest {
        save_root: root.to_owned(),
        folder: "test".into(),
        name: Some("test".into()),
        seed: Some(1),
        creative: true,
        create: None,
    }
}

#[test]
#[ignore = "needs prepared resources"]
fn server_reload_delivers_mod_files_overrides_and_preserves_all_data_on_failure() {
    let root = std::env::current_dir()
        .unwrap()
        .join("target")
        .join(format!("server-reload-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let manifest = "id = \"reload_test\"\nname = \"Reload Test\"\nversion = \"0.1.0\"\napi = \"0.2\"\nside = \"both\"\ncapabilities = [\"registries\"]\n";
    let package = root.join("reload_test.hmod");
    write(
        &package,
        horizon_mods::Package::write_files(
            manifest,
            &[("data/reload_test/machines/value.json", b"1")],
        )
        .unwrap(),
    );
    let package_manifest = horizon_mods::Package::read(&package).unwrap().manifest;
    let content = horizon_mods::Content::read(&package, &package_manifest).unwrap();
    let resources = root.join("resources");
    let base = std::path::PathBuf::from(std::env::var_os("HORIZON_TEST_RESOURCES").unwrap());
    horizon_client::mod_content::build(
        &base,
        &[horizon_client::mod_content::ContentMod {
            id: "reload_test".into(),
            content,
        }],
        &resources,
    )
    .unwrap();

    let world = root.join("world");
    let pack = world.join("datapacks/test");
    write(&pack.join("pack.mcmeta"), "{}");
    let data = pack.join("data/reload_test/machines/value.json");
    write(&data, "2");
    let function = pack.join("data/reload_test/function/keep.mcfunction");
    write(&function, "say old");
    write(
        &world.join("datapacks.json"),
        r#"{"enabled":["file/test"],"disabled":[]}"#,
    );
    let live = Arc::new(Mutex::new(0));
    let reject = Arc::new(AtomicBool::new(false));
    let mods = vec![ModEntry::new(
        "reload_test",
        Box::new(DataListener {
            live: live.clone(),
            reject: reject.clone(),
        }),
    )]
    .into();
    mod_api::testing::with_mods(mods, || {
        with_game(|game| {
            game.options.resources = resources;
            game.load_world(request(&world)).unwrap();
            assert_eq!(*live.lock().unwrap(), 2);
            assert!(game.run_command("datapack disable \"file/test\""));
            assert_eq!(
                *live.lock().unwrap(),
                1,
                "the mod's own packaged file is the fallback"
            );
            assert!(game.run_command("datapack enable \"file/test\""));
            assert_eq!(*live.lock().unwrap(), 2);
            write(&data, "3");
            assert!(game.run_command("reload"));
            assert_eq!(*live.lock().unwrap(), 3);

            let old_function = game.fns.store.functions["reload_test:keep"].clone();
            let old_selection = std::fs::read(world.join("datapacks.json")).unwrap();
            write(&data, "4");
            write(&function, "say new");
            reject.store(true, Ordering::Relaxed);
            assert!(!game.run_command("reload"));
            assert_eq!(
                *live.lock().unwrap(),
                3,
                "earlier successful prepare must not publish"
            );
            assert_eq!(game.fns.store.functions["reload_test:keep"], old_function);
            let error = game
                .reload_datapacks(true, &game.level.enabled_features.clone())
                .err()
                .unwrap()
                .to_string();
            assert!(error.contains("mod reload_test"), "{error}");
            assert!(
                error.contains("reload listener reload_test:check"),
                "{error}"
            );
            assert!(error.contains("reload_test:machines/value.json"), "{error}");
            assert!(!game.run_command("datapack disable \"file/test\""));
            assert_eq!(
                std::fs::read(world.join("datapacks.json")).unwrap(),
                old_selection
            );
            assert!(game.fns.store.functions.contains_key("reload_test:keep"));
            assert_eq!(*live.lock().unwrap(), 3);
            reject.store(false, Ordering::Relaxed);
            write(&data, "not JSON");
            let error = game
                .reload_datapacks(true, &game.level.enabled_features.clone())
                .err()
                .unwrap()
                .to_string();
            assert!(error.contains("mod reload_test: reload listener reload_test:data: reload_test:machines/value.json"), "{error}");
            assert_eq!(*live.lock().unwrap(), 3);
            assert_eq!(game.fns.store.functions["reload_test:keep"], old_function);
        })
    });
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "needs prepared resources"]
fn server_reload_registration_failure_stops_world_open_and_quiet_mods_are_pruned() {
    struct Quiet;
    impl Hooks for Quiet {}
    struct Broken;
    impl Hooks for Broken {
        fn add_server_reload_listeners(&self, event: &mut AddServerReloadListenersEvent<'_>) {
            let _ = event
                .listeners
                .add_dependency("mistyped:listener", VanillaServerListeners::FUNCTIONS);
        }
    }
    let root = std::env::current_dir()
        .unwrap()
        .join("target")
        .join(format!("server-reload-open-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    mod_api::testing::with_mods(vec![ModEntry::new("quiet", Box::new(Quiet))].into(), || {
        with_game(|game| {
            assert!(mod_api::listened(mod_api::HookId::AddServerReloadListeners));
            game.load_world(request(&root)).unwrap();
            assert!(!mod_api::listened(
                mod_api::HookId::AddServerReloadListeners
            ));
            assert!(game.run_command("reload"));
            assert!(!mod_api::listened(
                mod_api::HookId::AddServerReloadListeners
            ));
        })
    });
    std::fs::remove_dir_all(&root).unwrap();
    mod_api::testing::with_mods(
        vec![ModEntry::new("broken_mod", Box::new(Broken))].into(),
        || {
            with_game(|game| {
                let error = game.load_world(request(&root)).unwrap_err().to_string();
                assert!(
                    error.contains("mod broken_mod: unknown reload listener mistyped:listener"),
                    "{error}"
                );
                assert!(game.fns.store.functions.is_empty());
            })
        },
    );
    std::fs::remove_dir_all(root).unwrap();
}
