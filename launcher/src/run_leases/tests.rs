use super::*;
use std::io::{BufRead, Write};
use std::process::{Child, Stdio};

fn directory(path: &Path) -> Dir {
    fs::create_dir_all(path).unwrap();
    Dir::open_ambient_dir(path, cap_std::ambient_authority()).unwrap()
}

#[test]
fn shared_readers_exclude_cleanup_but_not_an_unrelated_tree() {
    let root = tempfile::tempdir().unwrap();
    let registry = Registry::at(&root.path().join("data")).unwrap();
    let first = directory(&root.path().join("first"));
    let other = directory(&root.path().join("other"));
    let a = registry.lock(&first, LockMode::Shared).unwrap();
    let b = registry.lock(&first, LockMode::Shared).unwrap();
    assert!(matches!(
        registry.lock(&first, LockMode::Exclusive),
        Err(Error::Busy)
    ));
    drop(registry.lock(&other, LockMode::Exclusive).unwrap());
    drop(a);
    assert!(matches!(
        registry.lock(&first, LockMode::Exclusive),
        Err(Error::Busy)
    ));
    drop(b);
    drop(registry.lock(&first, LockMode::Exclusive).unwrap());
}

#[cfg(unix)]
#[test]
fn a_moved_tree_keeps_its_identity_and_redirected_registries_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("data");
    let registry = Registry::at(&data).unwrap();
    let path = root.path().join("tree");
    let tree = directory(&path);
    let held = registry.lock(&tree, LockMode::Shared).unwrap();
    let moved = root.path().join("moved");
    fs::rename(&path, &moved).unwrap();
    let alias = directory(&moved);
    assert!(matches!(
        registry.lock(&alias, LockMode::Exclusive),
        Err(Error::Busy)
    ));
    let replacement = directory(&path);
    drop(registry.lock(&replacement, LockMode::Exclusive).unwrap());
    drop(held);
    drop(registry);
    fs::remove_dir_all(data.join("run-leases")).unwrap();
    let outside = root.path().join("outside");
    fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, data.join("run-leases")).unwrap();
    assert!(Registry::at(&data).is_err());
    assert_eq!(fs::read_dir(outside).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn a_lock_file_cannot_redirect_access_outside_the_registry() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("data");
    let registry = Registry::at(&data).unwrap();
    let tree = directory(&root.path().join("tree"));
    drop(registry.lock(&tree, LockMode::Shared).unwrap());
    let file = fs::read_dir(data.join("run-leases"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::remove_file(&file).unwrap();
    let precious = root.path().join("precious");
    fs::write(&precious, b"keep").unwrap();
    std::os::unix::fs::symlink(&precious, &file).unwrap();
    assert!(registry.lock(&tree, LockMode::Exclusive).is_err());
    // Capabilities also confine followed links; an in-registry alias must still be refused.
    fs::remove_file(&file).unwrap();
    fs::write(data.join("run-leases/sibling"), b"keep").unwrap();
    std::os::unix::fs::symlink("sibling", &file).unwrap();
    assert!(registry.lock(&tree, LockMode::Exclusive).is_err());
    assert_eq!(fs::read(precious).unwrap(), b"keep");
}

struct Actor(Child);
impl Drop for Actor {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn killed_holder_releases_production_home_source_and_parent_leases() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let game = root.path().join("game");
    let source = root.path().join("versions/native-fixture");
    let data = root.path().join("data");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&game).unwrap();
    fs::create_dir_all(&source).unwrap();
    #[cfg(windows)]
    fs::copy(
        std::env::current_exe().unwrap(),
        source.join(crate::backend::CLIENT),
    )
    .unwrap();
    let mut actor = Actor(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "run_leases::tests::lease_holder_actor",
                "--nocapture",
            ])
            .env("HORIZON_LEASE_ACTOR_ROOT", root.path())
            .env("HORIZON_LAUNCHER_HOME", &data)
            .env("HORIZON_HOME", &home)
            .env("XDG_DATA_HOME", root.path().join("xdg-data"))
            .env("XDG_CONFIG_HOME", root.path().join("xdg-config"))
            .env("HORIZON_LAUNCHER_TRASH", root.path().join("trash"))
            .env("HORIZON_LAUNCHER_CLIPBOARD", root.path().join("clipboard"))
            .env("HORIZON_LAUNCHER_DESKTOP", root.path().join("desktop"))
            .env("HORIZON_LAUNCHER_NO_SELF_UPDATE", "1")
            .env("HORIZON_NO_AUDIO", "1")
            .current_dir(root.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let output = actor.0.stdout.take().unwrap();
    let ready = std::io::BufReader::new(output)
        .lines()
        .map_while(Result::ok)
        .any(|line| line.contains("LEASE-READY"));
    assert!(ready, "holder did not acquire its production runtime guard");
    let registry = Registry::at(&data).unwrap();
    for path in [&home, &game, &source, source.parent().unwrap()] {
        assert!(
            matches!(
                registry.lock(&directory(path), LockMode::Exclusive),
                Err(Error::Busy)
            ),
            "active tree was not protected: {}",
            path.display()
        );
    }
    actor.0.kill().unwrap(); // SIGKILL on Unix; no Drop/marker cleanup in the holder.
    actor.0.wait().unwrap();
    for path in [&home, &game, &source, source.parent().unwrap()] {
        drop(
            registry
                .lock(&directory(path), LockMode::Exclusive)
                .unwrap(),
        );
    }
}

#[test]
fn gate_excludes_collection_mid_acquire_and_until_last_claim_drops() {
    let root = tempfile::tempdir().unwrap();
    let registry = Registry::at(root.path()).unwrap();
    let held = registry.directory.try_clone().unwrap();
    assert!(matches!(
        open_lock(&held, "gate", LockMode::Exclusive, Release::Unlock),
        Err(Error::Busy)
    ));
    let claim = registry
        .lock(&directory(&root.path().join("tree")), LockMode::Shared)
        .unwrap();
    drop(registry);
    assert!(matches!(
        open_lock(&held, "gate", LockMode::Exclusive, Release::Unlock),
        Err(Error::Busy)
    ));
    drop(claim);
    let exclusive = open_lock(&held, "gate", LockMode::Exclusive, Release::Unlock).unwrap();
    assert!(matches!(Registry::at(root.path()), Err(Error::Busy)));
    drop(exclusive);
    assert!(Registry::at(root.path()).is_ok());
}

#[cfg(target_os = "linux")]
#[test]
fn unsupported_volume_refuses_registry_and_tree_without_creating_entries() {
    assert!(matches!(
        Registry::at(Path::new("/proc")),
        Err(Error::UnsupportedFilesystem)
    ));
    let root = tempfile::tempdir().unwrap();
    let registry = Registry::at(root.path()).unwrap();
    let before = registry.directory.entries().unwrap().count();
    let proc = Dir::open_ambient_dir("/proc", cap_std::ambient_authority()).unwrap();
    assert!(matches!(
        registry.lock(&proc, LockMode::Shared),
        Err(Error::UnsupportedFilesystem)
    ));
    assert_eq!(registry.directory.entries().unwrap().count(), before);
}

#[test]
fn lease_holder_actor() {
    let Some(root) = std::env::var_os("HORIZON_LEASE_ACTOR_ROOT") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .current_dir(root.join("game"))
        .env("HORIZON_HOME", root.join("home"));
    mark(&mut command, &root.join("versions/native-fixture"));
    let (_command, _guard) = crate::backend::prepare_command(command).unwrap();
    println!("LEASE-READY");
    std::io::stdout().flush().unwrap();
    let mut byte = String::new();
    std::io::stdin().read_line(&mut byte).unwrap();
}
