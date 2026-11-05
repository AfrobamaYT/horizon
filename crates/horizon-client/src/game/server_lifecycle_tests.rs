use crate::game::{execution_role::ExecutionRole, native_entity_refs::tests::with_game};
use mod_api::{Hooks, ModEntry, ServerHost, testing::with_mods};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

struct PanicAt {
    stopping: AtomicBool,
    save: AtomicBool,
}

/// Exercise the real stop/save paths, not just direct calls to hook dispatch.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn every_started_server_stops_and_saves_even_when_startup_or_ticks_fail() {
    struct Log(Arc<Mutex<Vec<&'static str>>>, Arc<PanicAt>);
    #[mod_api::__subscriptions]
    impl Hooks for Log {
        fn server_starting(&self, _: &mut dyn ServerHost, _: &mut mod_api::ServerStartingEvent) {
            self.0.lock().unwrap().push("starting");
        }
        fn server_stopping(&self, _: &mut dyn ServerHost, _: &mut mod_api::ServerStoppingEvent) {
            self.0.lock().unwrap().push("stopping");
            if self.1.stopping.load(Ordering::Relaxed) {
                panic!("stopping listener");
            }
        }
        fn world_saved(&self, _: &mut dyn ServerHost, _: &mut mod_api::WorldSavedEvent) {
            self.0.lock().unwrap().push("saved");
            if self.1.save.load(Ordering::Relaxed) {
                panic!("save listener");
            }
        }
        fn server_stopped(&self, _: &mut dyn ServerHost, _: &mut mod_api::ServerStoppedEvent) {
            self.0.lock().unwrap().push("stopped");
        }
    }
    let log = Arc::new(Mutex::new(Vec::new()));
    let panic_at = Arc::new(PanicAt {
        stopping: AtomicBool::new(false),
        save: AtomicBool::new(false),
    });
    with_mods(
        vec![ModEntry::new(
            "log",
            Box::new(Log(log.clone(), panic_at.clone())),
        )]
        .into(),
        || {
            for dedicated in [false, true] {
                for case in [
                    "normal",
                    "startup-error",
                    "tick-error",
                    "panic",
                    "save-error",
                    "unopened",
                    "locked-unadopted",
                    "stopping-panic",
                    "save-panic",
                    "before-start",
                ] {
                    log.lock().unwrap().clear();
                    panic_at
                        .stopping
                        .store(case == "stopping-panic", Ordering::Relaxed);
                    panic_at.save.store(case == "save-panic", Ordering::Relaxed);
                    with_game(|game| {
                        game.options.execution_role = if dedicated {
                            ExecutionRole::DedicatedServer
                        } else {
                            ExecutionRole::IntegratedServer
                        };
                        let root = std::env::temp_dir().join(format!(
                            "server-lifecycle-{}-{dedicated}-{case}",
                            std::process::id()
                        ));
                        std::fs::create_dir_all(&root).unwrap();
                        game.world_root = root.clone();
                        game.save = Arc::new(horizon_client::save::SaveDir::new(
                            root.clone(),
                            horizon_client::save::LevelShape::OVERWORLD,
                        ));
                        game.level.name = "lifecycle final save".into();
                        let adopted =
                            !matches!(case, "unopened" | "before-start" | "locked-unadopted");
                        if adopted || case == "locked-unadopted" {
                            game.session_lock =
                                Some(horizon_client::save::SessionLock::acquire(&root).unwrap());
                        }
                        // The lock is held while tags are readable, before the prepared
                        // level replaces the placeholder. Only adoption may save.
                        game.world_adopted = adopted;
                        if matches!(case, "unopened" | "locked-unadopted") {
                            // A failed open must not overwrite an existing, unreadable save.
                            std::fs::write(root.join("level.dat"), b"unadopted save").unwrap();
                        }
                        let result = game.run_server_lifecycle(dedicated, |game, began| {
                            if case == "before-start" {
                                return Err("before startup".into());
                            }
                            *began = true;
                            game.native_server_about_to_start(dedicated);
                            game.native_server_starting(dedicated);
                            game.native_lifecycle_open_started();
                            if matches!(case, "startup-error" | "unopened" | "locked-unadopted") {
                                return Err("failed init".into());
                            }
                            game.native_lifecycle_world_ready();
                            match case {
                                "tick-error" => Err("failed tick".into()),
                                "panic" => panic!("failed tick"),
                                "save-error" => {
                                    let file = root.join("not-a-directory");
                                    std::fs::write(&file, b"occupied").unwrap();
                                    game.world_root = file;
                                    Ok(())
                                }
                                _ => Ok(()),
                            }
                        });
                        assert_eq!(result.is_ok(), case == "normal", "{dedicated} {case}");
                        if matches!(case, "stopping-panic" | "save-panic") {
                            let message = result.unwrap_err().to_string();
                            let needle = if case == "stopping-panic" {
                                "stopping listener"
                            } else {
                                "save listener"
                            };
                            assert!(message.contains(needle), "{dedicated} {case}: {message}");
                        }
                        let expected: &[&str] = match case {
                            "before-start" => &[],
                            "unopened" | "save-error" | "locked-unadopted" => {
                                &["starting", "stopping", "stopped"]
                            }
                            _ => &["starting", "stopping", "saved", "stopped"],
                        };
                        assert_eq!(*log.lock().unwrap(), expected, "{dedicated} {case}");
                        assert!(game.session_lock.is_none(), "storage released after {case}");
                        assert!(!game.world_adopted, "adoption cleared after {case}");
                        if matches!(case, "unopened" | "locked-unadopted") {
                            assert_eq!(
                                std::fs::read(root.join("level.dat")).unwrap(),
                                b"unadopted save",
                                "{dedicated} {case}"
                            );
                        } else if !matches!(case, "save-error" | "before-start") {
                            let saved = horizon_client::save::Level::read(&root).unwrap();
                            assert_eq!(saved.name, "lifecycle final save");
                        }
                        std::fs::remove_dir_all(root).unwrap();
                    });
                }
            }
        },
    );
}
