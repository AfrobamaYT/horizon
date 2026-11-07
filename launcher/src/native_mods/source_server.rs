//! The dedicated ordering proof needs prepared Minecraft data and builds the real generated game.
use super::*;

#[test]
#[ignore = "requires HORIZON_TEST_RESOURCES (prepared 26.3), Rust, Python and a real game build"]
fn dedicated_function_slots_keep_order_and_write_back() {
    let resources = PathBuf::from(
        std::env::var_os("HORIZON_TEST_RESOURCES")
            .expect("set HORIZON_TEST_RESOURCES to prepared 26.3"),
    );
    assert!(resources.join("asset-index.json").is_file());
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let scratch = repo
        .join("target")
        .join(format!("source-server-{}", std::process::id()));
    fs::create_dir_all(&scratch).unwrap();
    let source = scratch.join("source");
    fs::create_dir(&source).unwrap();
    let archive = scratch.join("source.tar");
    let mut git = Command::new("git");
    git.current_dir(repo)
        .args(["archive", "HEAD"])
        .stdout(fs::File::create(&archive).unwrap());
    checked(git);
    let mut tar = Command::new("tar");
    tar.arg("-xf").arg(archive).arg("-C").arg(&source);
    checked(tar);
    let catalog_dir = scratch.join("catalog");
    fs::create_dir(&catalog_dir).unwrap();
    horizon_mod_api_gen::staging::stage(
        &source,
        &catalog_dir.join("points.json"),
        &catalog_dir.join("internals.json"),
        horizon_mod_api_gen::staging::Mode::Build,
    )
    .unwrap();
    let catalog = internals::Catalog::read(&catalog_dir).unwrap();
    let fixture = repo.join("tools/fixtures/native-slot-order");
    let mut bundle = bundle_in(&fixture).unwrap().unwrap();
    let copied = scratch.join("mod");
    copy_bundle(&bundle, &copied).unwrap();
    bundle.dir = copied;
    let native = bundle.manifest.native.as_mut().unwrap();
    for entry in &mut native.internals {
        entry.hash = catalog
            .internals
            .iter()
            .find(|item| internals::plain_path(&item.path) == internals::plain_path(&entry.path))
            .unwrap()
            .hash
            .clone();
    }
    for entry in &mut bundle.manifest.inject {
        entry.hash = catalog
            .internals
            .iter()
            .find(|item| internals::plain_path(&item.path) == internals::plain_path(&entry.at))
            .unwrap()
            .hash
            .clone();
        if let Some(call) = &entry.call {
            let site = catalog
                .sites
                .iter()
                .find(|site| {
                    internals::plain_path(&site.target) == internals::plain_path(&entry.at)
                        && &site.selector == call
                        && Some(site.ordinal) == entry.nth
                })
                .unwrap();
            entry.ctx = Some(site.hash.clone());
        }
    }
    let manifest_path = bundle.dir.join("horizon-mod.toml");
    let mut document: toml::Value =
        toml::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    document["native"]["horizon"] = String::from_utf8(
        Command::new("git")
            .current_dir(repo)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .into();
    document["native"]["internals"] = toml::Value::try_from(&bundle.native().internals).unwrap();
    document["inject"] = toml::Value::try_from(&bundle.manifest.inject).unwrap();
    fs::write(manifest_path, toml::to_string(&document).unwrap()).unwrap();
    catalog.check(&bundle.manifest).unwrap();
    assert!(api::stage(
        &source,
        &[api::Project::read(&bundle).unwrap()],
        &[vec![]],
        &dependencies::Resolution { order: vec![0] },
        &catalog
    )
    .unwrap());
    let target = scratch.join("target");
    let mut cargo = Command::new("cargo");
    cargo
        .current_dir(&source)
        .env("CARGO_TARGET_DIR", &target)
        .args([
            "build",
            "--profile",
            "fast",
            "--config",
            "profile.fast.package.horizon-client.opt-level=0",
            "--no-default-features",
            "-p",
            api::PACKAGE,
            "--bin",
            "horizon-client",
        ]);
    checked(cargo);
    copy_bundle(
        &bundle,
        &target.join("fast/horizon-native-mods/native_slot_order"),
    )
    .unwrap();
    let mut driver = Command::new("python3");
    driver
        .arg(repo.join("tools/test_native_slot_order.py"))
        .arg("--binary")
        .arg(target.join("fast/horizon-client"))
        .arg("--resources")
        .arg(resources)
        .arg("--scratch")
        .arg(scratch);
    checked(driver);
}

fn checked(mut command: Command) {
    let status = command.status().unwrap();
    assert!(status.success(), "{command:?}: {status}");
}
