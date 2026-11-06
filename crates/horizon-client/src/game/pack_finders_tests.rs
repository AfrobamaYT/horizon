use super::*;
use horizon_client::datapack::{DataStore, FunctionLine};
use mod_api::{AddPackFindersEvent, Hooks, Identifier, ModEntry, Text, testing::with_mods};
use std::{fs, io::Write, path::PathBuf};

const ASSET_PACK: &str = "assets/packmod/resourcepacks/optional";
const DATA_PACK: &str = "data/packmod/datapacks/optional";
const ASSET: &str = "assets/packmod/lang/en_us.json";

struct Fixture {
    root: PathBuf,
    package: PathBuf,
    archive: bool,
    files: Vec<(String, Vec<u8>)>,
}
impl Fixture {
    fn new(archive: bool) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let number = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "pack-finders-{}-{}-{number}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        fs::create_dir(&root).unwrap();
        let metadata = |format: (i64, i64)| {
            serde_json::to_vec(&serde_json::json!({
                "pack": {"description": {"translate": "packmod.description"},
                    "min_format": [format.0, format.1], "max_format": [format.0, format.1]},
                "filter": {"block": [{"namespace": "packmod", "path": "lang/hidden\\.json"}]},
            }))
            .unwrap()
        };
        let package = root.join(if archive { "packmod.hmod" } else { "packmod" });
        let fixture = Self {
            root, package, archive,
            files: vec![
                ("horizon-mod.toml".into(), b"id='packmod'\nname='Pack mod'\nversion='1'\napi='0.2'\ncapabilities=['registries']\n".to_vec()),
                (format!("{ASSET_PACK}/pack.mcmeta"), metadata(horizon_core::version::RESOURCE_PACK_FORMAT)),
                (format!("{ASSET_PACK}/{ASSET}"), br#"{"packmod.optional":"first"}"#.to_vec()),
                (format!("{DATA_PACK}/pack.mcmeta"), metadata(horizon_core::version::DATA_PACK_FORMAT)),
                (format!("{DATA_PACK}/data/packmod/function/probe.mcfunction"), b"say from-optional\n".to_vec()),
            ],
        };
        fixture.publish();
        fixture
    }
    fn publish(&self) {
        if self.archive {
            let next = self.package.with_extension("next.hmod");
            let mut zip = zip::ZipWriter::new(fs::File::create(&next).unwrap());
            for (name, bytes) in &self.files {
                zip.start_file(name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(bytes).unwrap();
            }
            zip.finish().unwrap();
            fs::rename(next, &self.package).unwrap();
        } else {
            for (name, bytes) in &self.files {
                let path = self.package.join(name);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, bytes).unwrap();
            }
        }
    }
    fn sources(&self) -> ModsState {
        ModsState {
            selected: vec![horizon_mods::selection::SelectedPackage {
                path: self.package.clone(),
                package: horizon_mods::Package::read(&self.package).unwrap(),
            }]
            .into(),
            ..Default::default()
        }
    }
    fn asset_id(&self) -> String {
        format!("mod/packmod:{ASSET_PACK}")
    }
    fn data_id(&self) -> String {
        format!("mod/packmod:{DATA_PACK}")
    }
    fn replace_asset(&mut self) {
        self.files
            .iter_mut()
            .find(|(name, _)| name == &format!("{ASSET_PACK}/{ASSET}"))
            .unwrap()
            .1 = br#"{"packmod.optional":"second"}"#.to_vec();
        self.publish();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

struct Finder;
#[mod_api::__subscriptions]
impl Hooks for Finder {
    fn add_pack_finders(&self, _host: HostSide<'_>, event: &mut AddPackFindersEvent<'_>) {
        for (kind, path) in [
            (PackType::ClientResources, ASSET_PACK),
            (PackType::ServerData, DATA_PACK),
        ] {
            event
                .packs
                .add_pack_finders(PackRegistration::new(
                    Identifier::parse(&format!("packmod:{path}")).unwrap(),
                    kind,
                    Text::translate("packmod.optional.title", &[]),
                ))
                .unwrap();
        }
    }
}
fn with_finder(test: impl FnOnce()) {
    with_mods(
        vec![ModEntry::new("packmod", Box::new(Finder))].into(),
        test,
    );
}

#[test]
fn contributed_packs_are_discoverable_and_only_selected_resources_apply() {
    for archive in [false, true] {
        let fixture = Fixture::new(archive);
        let sources = fixture.sources();
        with_finder(|| {
            let packs = discover(PackType::ClientResources, &sources);
            assert_eq!(packs.len(), 1);
            let infos = crate::game::packs::pack_infos(&fixture.root, &[], &packs);
            assert!(
                infos
                    .iter()
                    .any(|info| info.id == fixture.asset_id() && !info.required)
            );
            let base = fixture.root.join("base");
            fs::create_dir_all(base.join("assets/packmod/lang")).unwrap();
            fs::write(base.join(ASSET), b"base").unwrap();
            fs::write(base.join("assets/packmod/lang/hidden.json"), b"hidden").unwrap();
            let base = ResourceSource::directory(base);
            let off = packs::resources(base.clone(), &packs::enabled(&packs, &[])).unwrap();
            assert_eq!(off.read(ASSET).unwrap(), b"base");
            let selected = packs::enabled(&packs, &[fixture.asset_id()]);
            let on = packs::resources(base, &selected).unwrap();
            assert_eq!(on.read(ASSET).unwrap(), br#"{"packmod.optional":"first"}"#);
            assert!(on.read("assets/packmod/lang/hidden.json").is_err());

            let data = data_packs(&sources);
            assert_eq!(data.len(), 1);
            let world = fixture.root.join("world");
            fs::create_dir(&world).unwrap();
            let repository = data_repository(&world, &fixture.root, &data);
            assert!(horizon_client::datapack::enabled_ids(&world, &repository, &[]).is_empty());
            fs::write(
                world.join("datapacks.json"),
                serde_json::json!({
                    "enabled": [fixture.data_id()], "disabled": [],
                })
                .to_string(),
            )
            .unwrap();
            let stack = crate::game::functions::pack_stack(
                &world,
                &fixture.root,
                &Default::default(),
                &Default::default(),
                true,
                &[],
                &data,
            );
            let enabled = stack.enabled_packs().collect::<Vec<_>>();
            let store = DataStore::load(&enabled);
            assert_eq!(
                store.functions["packmod:probe"].lines,
                [FunctionLine::Command("say from-optional".into())]
            );
            if archive {
                assert!(!fixture.root.join("packmod").exists());
            }
        });
    }
}

#[test]
fn rediscovery_opens_the_new_archive_without_changing_old_readers() {
    let mut fixture = Fixture::new(true);
    let sources = fixture.sources();
    with_finder(|| {
        let old = discover(PackType::ClientResources, &sources);
        fixture.replace_asset();
        let new = discover(PackType::ClientResources, &sources);
        assert_eq!(
            old[0].source.open().unwrap().read(ASSET).unwrap(),
            br#"{"packmod.optional":"first"}"#
        );
        assert_eq!(
            new[0].source.open().unwrap().read(ASSET).unwrap(),
            br#"{"packmod.optional":"second"}"#
        );
        assert!(!fixture.root.join("packmod").exists());
    });
}

#[test]
fn refused_registrations_leave_the_repository_unchanged() {
    let fixture = Fixture::new(false);
    let mut registry = Registry::new(PackType::ClientResources, fixture.sources());
    let entry = |path: &str| {
        PackRegistration::new(
            Identifier::parse(path).unwrap(),
            PackType::ClientResources,
            Text::translate("packmod.optional.title", &[]),
        )
    };
    let valid = format!("packmod:{ASSET_PACK}");
    registry.add_pack_finders(entry(&valid)).unwrap();
    for (path, error) in [
        (valid.as_str(), PackFinderError::Duplicate),
        ("packmod:../outside", PackFinderError::InvalidPath),
        ("absent:optional", PackFinderError::MissingPackage),
        ("packmod:missing", PackFinderError::InvalidMetadata),
    ] {
        assert_eq!(registry.add_pack_finders(entry(path)), Err(error));
        assert_eq!(registry.packs.len(), 1);
    }
    assert_eq!(
        registry.packs[0]
            .source
            .open()
            .unwrap()
            .read(ASSET)
            .unwrap(),
        br#"{"packmod.optional":"first"}"#
    );
}

#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn a_mods_pack_is_on_the_screen_and_applies_through_client_reload() {
    let mut fixture = Fixture::new(true);
    let mut ran = false;
    with_finder(|| {
        crate::game::native_entity_refs::tests::with_client_game(|game| {
            ran = true;
            game.mods = fixture.sources();
            game.refresh_pack_lists();
            assert!(
                game.ui
                    .options
                    .pack_infos
                    .iter()
                    .any(|info| info.id == fixture.asset_id())
            );
            game.settings.resource_packs = vec![fixture.asset_id()];
            for expected in ["first", "second"] {
                let listeners = game.register_client_reload_listeners();
                let staged = crate::game::packs::stage_pack(
                    &game.vanilla_root(),
                    &game.settings.resource_packs,
                    &[],
                    &game.mods,
                    listeners,
                    &game.pack.assets,
                    &game.res,
                    &game.worker_context(),
                    false,
                    false,
                    &crate::game::Progress::new(),
                )
                .unwrap();
                assert_eq!(staged.language.get("packmod.optional"), Some(expected));
                game.apply_staged_pack(staged).unwrap();
                assert_eq!(horizon_client::lang::get("packmod.optional"), expected);
                assert!(
                    !game
                        .options
                        .resources
                        .join(format!("{ASSET_PACK}/{ASSET}"))
                        .exists()
                );
                if expected == "first" {
                    fixture.replace_asset();
                }
            }
        });
    });
    assert!(ran, "the prepared-resource fixture must run this case");
}
