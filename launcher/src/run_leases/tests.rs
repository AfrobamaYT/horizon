use super::*;
use std::io::{BufRead, Write};
use std::process::{Child, Stdio};

fn directory(path: &Path) -> Dir {
    fs::create_dir_all(path).unwrap();
    Dir::open_ambient_dir(path, cap_std::ambient_authority()).unwrap()
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

#[cfg(unix)]
#[test]
fn routed_game_keeps_its_lease_after_launcher_sigkill() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().canonicalize().unwrap();
    let install = data.join("install");
    let home = data.join("home");
    fs::create_dir_all(&install).unwrap();
    fs::create_dir_all(&home).unwrap();
    fs::write(install.join(".installed"), b"").unwrap();
    // Copy from a separate process, avoiding a writable image fd in libtest's concurrent forks.
    assert!(Command::new("cp")
        .arg(std::env::current_exe().unwrap())
        .arg(install.join("actor"))
        .status()
        .unwrap()
        .success());
    let mut supervisor = Actor(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "run_leases::tests::routing_actor", "--nocapture"])
            .env("ROUTING_ROOT", &data)
            .env("ROUTING_MODE", "launcher")
            .env("HORIZON_LAUNCHER_HOME", &data)
            .env("HORIZON_HOME", &home)
            .env("XDG_DATA_HOME", data.join("xdg-data"))
            .env("XDG_CONFIG_HOME", data.join("xdg-config"))
            .env("HORIZON_LAUNCHER_TRASH", data.join("trash"))
            .env("HORIZON_LAUNCHER_CLIPBOARD", data.join("clipboard"))
            .env("HORIZON_LAUNCHER_DESKTOP", data.join("desktop"))
            .env("HORIZON_LAUNCHER_NO_SELF_UPDATE", "1")
            .env("HORIZON_NO_AUDIO", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !data.join("routed-ready").exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "routed game did not start"
        );
        std::thread::yield_now();
    }
    supervisor.0.kill().unwrap();
    supervisor.0.wait().unwrap();
    let registry = Registry::at(&data).unwrap();
    assert!(matches!(
        registry.lock(&directory(&install), LockMode::Exclusive),
        Err(Error::Busy)
    ));
    fs::write(data.join("routed-stop"), b"").unwrap();
}

#[cfg(unix)]
#[test]
fn routing_actor() {
    horizon_mod_package::holder::enter();
    let Some(root) = std::env::var_os("ROUTING_ROOT") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    if std::env::var("ROUTING_MODE").as_deref() == Ok("launcher") {
        let mut game = Command::new(root.join("install/actor"));
        game.args(["--exact", "run_leases::tests::routing_actor"])
            .env("ROUTING_MODE", "game")
            .current_dir(root.join("home"));
        let game = crate::launch::wrap(game, &crate::launch::Launch::default());
        let (mut game, _runtime) = crate::backend::prepare_command(game).unwrap();
        let mut child = super::spawn(&mut game, false).unwrap();
        let _ = child.wait();
    } else {
        let (_capture, selection) = horizon_mod_package::holder::bootstrap();
        assert!(selection.is_some(), "game did not enter through the holder");
        fs::write(root.join("routed-ready"), b"").unwrap();
        let end = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while !root.join("routed-stop").exists() {
            assert!(std::time::Instant::now() < end);
            std::thread::yield_now();
        }
    }
}
