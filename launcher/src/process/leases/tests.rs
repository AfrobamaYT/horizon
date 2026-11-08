use super::*;
use std::{
    io::{BufRead, Write},
    process::{Command, Stdio},
};

fn directory(path: &Path) -> Dir {
    std::fs::create_dir_all(path).unwrap();
    Dir::open_ambient_dir(path, cap_std::ambient_authority()).unwrap()
}

#[cfg(unix)]
fn released(registry: &std::sync::Arc<Registry>, tree: &Dir) {
    // Another parallel fork can briefly retain an untouched CLOEXEC copy until its exec.
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
    loop {
        match registry.lock(tree, LockMode::Exclusive) {
            Ok(claim) => {
                drop(claim);
                return;
            }
            Err(Error::Busy) if std::time::Instant::now() < deadline => std::thread::yield_now(),
            _ => panic!("claim remained held after failed exec or child exit"),
        }
    }
}

#[cfg(unix)]
#[test]
fn child_keeps_gate_and_tree_after_creator_closes_its_copies() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("data");
    let trees = [directory(&root.path().join("tree"))];
    let claims = ChildLeases::acquire(&data, &trees).unwrap();
    use std::os::fd::AsRawFd;
    for file in claims.files() {
        // SAFETY: file owns this descriptor in the current process.
        assert_ne!(
            unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
            0
        );
    }
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "process::leases::tests::bridge_actor",
            "--nocapture",
        ])
        .env("HORIZON_LEASE_BRIDGE_ACTOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    let mut child = claims.spawn(command).unwrap();
    let mut output = std::io::BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    while !line.contains("BRIDGE-READY") {
        assert_ne!(
            output.read_line(&mut line).unwrap(),
            0,
            "actor exited before readiness"
        );
    }
    let registry = Registry::with_release(&data, Release::Unlock).unwrap();
    assert!(matches!(
        registry.lock(&trees[0], LockMode::Exclusive),
        Err(Error::Busy)
    ));
    drop(
        registry
            .lock(
                &directory(&root.path().join("unrelated")),
                LockMode::Exclusive,
            )
            .unwrap(),
    );
    let gate = File::options()
        .read(true)
        .write(true)
        .open(data.join("run-leases/gate"))
        .unwrap();
    assert!(matches!(
        gate.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    child.stdin.take().unwrap().write_all(b"exit\n").unwrap();
    assert!(child.wait().unwrap().success());
    released(&registry, &trees[0]);
    drop(registry);
    gate.try_lock().unwrap();
    gate.unlock().unwrap();
}

#[cfg(unix)]
#[test]
fn failed_exec_leaves_no_tree_or_gate_owner() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("data");
    let trees = [directory(&root.path().join("tree"))];
    let claims = ChildLeases::acquire(&data, &trees).unwrap();
    assert!(claims
        .spawn(Command::new(root.path().join("missing-program")))
        .is_err());
    let registry = Registry::with_release(&data, Release::Unlock).unwrap();
    released(&registry, &trees[0]);
    drop(registry);
    let gate = File::options()
        .read(true)
        .write(true)
        .open(data.join("run-leases/gate"))
        .unwrap();
    gate.try_lock().unwrap();
    gate.unlock().unwrap();
}

#[test]
fn bridge_actor() {
    if std::env::var_os("HORIZON_LEASE_BRIDGE_ACTOR").is_none() {
        return;
    }
    println!("BRIDGE-READY");
    std::io::stdout().flush().unwrap();
    std::io::stdin().read_line(&mut String::new()).unwrap();
}
