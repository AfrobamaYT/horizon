use super::*;
use std::{fs, io::Write, process::Stdio};
const MODE: &str = "HORIZON_HOLDER_TEST_MODE";
const READY: &str = "HORIZON_HOLDER_TEST_READY";
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(3)
}
pub(super) fn actor_command() -> Command {
    let tests = std::env::current_exe().unwrap();
    let actor = tests
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
        .join(format!("holder_actor{}", std::env::consts::EXE_SUFFIX));
    actor_at(actor)
}
fn actor_at(actor: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut command = Command::new(actor);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}
fn selected_actor(prepared: &Prepared) -> Command {
    let executable = prepared.request.entry.trees[0]
        .path
        .join(format!("holder_actor{}", std::env::consts::EXE_SUFFIX));
    let mut command = actor_at(executable);
    command.current_dir(&prepared.request.program.directory);
    command
}
fn fixture() -> (tempfile::TempDir, Marker, Prepared) {
    fixture_in(false)
}
fn fixture_in(native_path: bool) -> (tempfile::TempDir, Marker, Prepared) {
    fixture_layout(native_path, false)
}
fn fixture_layout(native_path: bool, native_image: bool) -> (tempfile::TempDir, Marker, Prepared) {
    let root = tempfile::tempdir().unwrap();
    let mut path = root.path().canonicalize().unwrap();
    if native_path {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            path.push(OsString::from_vec(vec![b'n', 255]));
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt;
            path.push(OsString::from_wide(&[110, 0xd800]));
        }
        fs::create_dir(&path).unwrap();
    }
    fs::create_dir(path.join("image")).unwrap();
    fs::create_dir(path.join("world")).unwrap();
    if !native_image {
        fs::write(path.join("image/.installed"), b"").unwrap();
    }
    let source = actor_command();
    let image = path
        .join("image")
        .join(Path::new(source.get_program()).file_name().unwrap());
    fs::copy(source.get_program(), &image).unwrap();
    let command = actor_at(image);
    let program = Program {
        executable: command.get_program().to_owned(),
        arguments: command.get_args().map(OsString::from).collect(),
        environment: vec![
            (MODE.into(), "game".into()),
            (READY.into(), path.join("ready").into_os_string()),
            (ENV.into(), "must-not-recurse".into()),
        ],
        directory: path.join("world"),
        raw_windows: None,
    };
    let mut snapshots = vec![
        (path.join("image"), !native_image),
        (path.join("world"), false),
    ];
    if native_image {
        fs::create_dir(path.join("sdk")).unwrap();
        fs::write(path.join("sdk/.installed"), b"").unwrap();
        snapshots.push((path.join("sdk"), true));
    }
    let prepared = Prepared::new(path.join("data"), snapshots, program).unwrap();
    (root, Marker::new(path.join("endpoint")).unwrap(), prepared)
}
fn assert_noninheriting(prepared: &Prepared) {
    for file in prepared._claims.iter().flat_map(Claim::files) {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            assert_ne!(
                unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
                0
            );
        }
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            use windows_sys::Win32::Foundation::{GetHandleInformation, HANDLE_FLAG_INHERIT};
            let mut flags = 0;
            assert_ne!(
                unsafe { GetHandleInformation(file.as_raw_handle(), &mut flags) },
                0
            );
            assert_eq!(flags & HANDLE_FLAG_INHERIT, 0);
        }
    }
}
fn assert_exclusive(entry: &Entry, available: bool) {
    let registry = Registry::existing(&entry.data).unwrap();
    for tree in &entry.trees {
        let directory = tree.recheck().unwrap();
        let claim = registry.lock(&directory, LockMode::Exclusive);
        assert_eq!(claim.is_ok(), available, "unexpected kernel exclusion");
    }
}
fn actor_ready(program: &Program) -> PathBuf {
    PathBuf::from(
        &program
            .environment
            .iter()
            .find(|(key, _)| key == READY)
            .unwrap()
            .1,
    )
}
fn ready(path: &Path) {
    let end = deadline();
    while !path.is_file() {
        assert!(
            Instant::now() < end,
            "game actor did not acquire entry claims"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn released(entry: &Entry) {
    let end = deadline();
    loop {
        let registry = Registry::existing(&entry.data).unwrap();
        if entry.trees.iter().all(|tree| {
            registry
                .lock(&tree.recheck().unwrap(), LockMode::Exclusive)
                .is_ok()
        }) {
            return;
        }
        assert!(Instant::now() < end, "actor left live claims after exit");
        std::thread::sleep(Duration::from_millis(5));
    }
}
// B: unsupported images must never receive private bootstrap state or be spawned as holders.
#[test]
fn old_image_returns_lossless_legacy_command_without_spawn() {
    let (root, marker, prepared) = fixture();
    let fake = root.path().join("old-game");
    fs::write(&fake, b"not a supported holder").unwrap();
    let legacy = prepared.request.program.executable.clone();
    let started = launch(Command::new(fake), &marker, &prepared, deadline()).unwrap();
    let Started::Legacy(command) = started else {
        panic!("spawned an old holder")
    };
    assert_eq!(command.get_program(), legacy);
    assert!(!root.path().join("endpoint").exists());
    assert!(!root.path().join("ready").exists());
    assert!(
        !command
            .get_envs()
            .any(|(key, value)| key == ENV && value.is_some())
    );
}
#[test]
fn capability_probe_handles_chunk_boundaries_and_missing_suffix() {
    let root = tempfile::tempdir().unwrap();
    let image = root.path().join("image");
    let mut bytes = vec![0; 8192 - 7];
    bytes.extend(CAPABILITY);
    fs::write(&image, &bytes).unwrap();
    assert!(capable(&image).unwrap());
    bytes.pop();
    fs::write(&image, bytes).unwrap();
    assert!(!capable(&image).unwrap());
}
#[test]
fn unknown_private_version_never_reaches_normal_initialization() {
    let (root, marker, _prepared) = fixture();
    let mut invalid = serde_json::to_value(marker).unwrap();
    invalid["version"] = 99.into();
    let status = actor_command()
        .env(ENV, serde_json::to_string(&invalid).unwrap())
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(78));
    assert!(!root.path().join("ready").exists());
}
// C: identity and authoritative .installed generation must survive both sides of acquisition.
#[test]
fn replacing_tree_or_completion_refuses_before_game_spawn() {
    for replace_tree in [false, true] {
        let (root, marker, prepared) = fixture();
        if replace_tree {
            fs::rename(root.path().join("image"), root.path().join("old-image")).unwrap();
            fs::create_dir(root.path().join("image")).unwrap();
        } else {
            fs::remove_file(root.path().join("image/.installed")).unwrap();
        }
        fs::write(root.path().join("image/.installed"), b"").unwrap();
        assert!(launch(selected_actor(&prepared), &marker, &prepared, deadline()).is_err());
        assert!(!root.path().join("ready").exists());
    }
}
#[test]
fn holder_image_outside_the_captured_trees_is_refused() {
    let (_root, marker, prepared) = fixture();
    let result = launch(actor_command(), &marker, &prepared, deadline());
    let rejected = result.is_err();
    if let Ok(Started::Holder { mut child, .. }) = result {
        child.stdin.take().unwrap().write_all(&[1]).unwrap();
        child.wait().unwrap();
    }
    assert!(rejected, "holder's actual image tree was not reserved");
}
#[test]
fn changed_ancillary_tree_is_refused_even_if_its_new_lock_inode_exists() {
    let (root, marker, prepared) = fixture();
    fs::rename(root.path().join("world"), root.path().join("old-world")).unwrap();
    fs::create_dir(root.path().join("world")).unwrap();
    let registry = Registry::with_release(&prepared.request.entry.data, Release::Close).unwrap();
    let directory = open_directory(&root.path().join("world")).unwrap();
    drop(registry.lock(&directory, LockMode::Shared).unwrap());
    assert!(launch(selected_actor(&prepared), &marker, &prepared, deadline()).is_err());
    assert!(!root.path().join("ready").exists());
}
#[test]
fn gui_rechecks_completion_after_acquired_before_committing() {
    let (_root, marker, prepared) = fixture();
    let mut command = selected_actor(&prepared);
    command.env(MODE, "swap-acquired");
    let result = launch(command, &marker, &prepared, deadline());
    assert!(matches!(result, Err(error) if error.kind() == io::ErrorKind::InvalidData));
}
#[test]
fn holder_rechecks_completion_after_commit_before_running_even_a_wrapper() {
    let (root, marker, mut prepared) = fixture();
    prepared
        .request
        .program
        .environment
        .iter_mut()
        .find(|(key, _)| key == MODE)
        .unwrap()
        .1 = "wrapper".into();
    let mut endpoint = Endpoint::bind(&marker).unwrap();
    let mut child = selected_actor(&prepared)
        .env(ENV, serde_json::to_string(&marker).unwrap())
        .spawn()
        .unwrap();
    let channel = Channel::authenticate(
        endpoint.accept(deadline()).unwrap(),
        &marker,
        true,
        deadline(),
    )
    .unwrap()
    .send(&prepared.request)
    .unwrap();
    let (channel, reply) = channel.receive::<Reply>().unwrap();
    assert!(matches!(reply, Reply::Acquired));
    let complete = root.path().join("image/.installed");
    fs::remove_file(&complete).unwrap();
    fs::write(complete, b"").unwrap();
    drop(channel.send(&Reply::Commit).unwrap());
    assert_eq!(child.wait().unwrap().code(), Some(78));
    assert!(!root.path().join("ready").exists());
}
#[test]
fn missing_or_nonempty_completion_and_relative_roots_are_refused() {
    let root = tempfile::tempdir().unwrap();
    assert!(Snapshot::capture(root.path(), true).is_err());
    fs::write(root.path().join(".installed"), b"partial").unwrap();
    assert!(Snapshot::capture(root.path(), true).is_err());
    assert!(Snapshot::capture(Path::new("relative"), false).is_err());
}
// E: no recursion secret is placed in the game; entry carries only typed ownership metadata.
#[test]
fn game_environment_scrubs_holder_marker_and_preserves_os_arguments() {
    let (_root, _marker, prepared) = fixture();
    let mut program = prepared.request.program;
    program
        .environment
        .push((ENV.into(), "secret-bootstrap".into()));
    program
        .environment
        .push((ENTRY.into(), "stale-entry".into()));
    program.arguments = vec!["a b".into(), "quoted\"value".into(), "".into()];
    let mut command = program.command().unwrap();
    command
        .env(
            ENTRY,
            serde_json::to_string(&prepared.request.entry).unwrap(),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = command.spawn().unwrap();
    let path = actor_ready(&program);
    ready(&path);
    let observed: Vec<OsString> = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(observed, program.arguments);
    child.stdin.take().unwrap().write_all(&[1]).unwrap();
    assert!(child.wait().unwrap().success());
}
#[test]
#[cfg(target_os = "linux")]
fn independent_holder_retains_tree_and_completion_objects_against_identity_reuse() {
    use std::os::unix::fs::MetadataExt as UnixMetadata;
    let (root, marker, prepared) = fixture();
    let tree = fs::metadata(root.path().join("image")).unwrap();
    let completion = fs::metadata(root.path().join("image/.installed")).unwrap();
    let Started::Holder { mut child, .. } =
        launch(selected_actor(&prepared), &marker, &prepared, deadline()).unwrap()
    else {
        panic!("missing actor capability")
    };
    drop(prepared);
    ready(&root.path().join("ready"));
    let held: Vec<_> = fs::read_dir(format!("/proc/{}/fd", child.id()))
        .unwrap()
        .filter_map(|entry| fs::metadata(entry.ok()?.path()).ok())
        .map(|metadata| (UnixMetadata::dev(&metadata), UnixMetadata::ino(&metadata)))
        .collect();
    child.stdin.take().unwrap().write_all(&[1]).unwrap();
    assert!(child.wait().unwrap().success());
    assert!(
        held.contains(&(UnixMetadata::dev(&tree), UnixMetadata::ino(&tree))),
        "holder did not retain its tree object"
    );
    assert!(
        held.contains(&(
            UnixMetadata::dev(&completion),
            UnixMetadata::ino(&completion)
        )),
        "holder did not retain its completion object"
    );
}
#[test]
fn native_image_tree_and_separate_completed_sdk_are_both_protected() {
    let (root, marker, prepared) = fixture_layout(false, true);
    let entry = prepared.request.entry.clone();
    let Started::Holder { mut child, .. } =
        launch(selected_actor(&prepared), &marker, &prepared, deadline()).unwrap()
    else {
        panic!("missing actor capability")
    };
    drop(prepared);
    ready(&root.path().join("ready"));
    assert_exclusive(&entry, false);
    child.stdin.take().unwrap().write_all(&[1]).unwrap();
    assert!(child.wait().unwrap().success());
    released(&entry);
}
#[test]
fn native_os_paths_survive_the_entire_bootstrap_and_independent_entry() {
    let (_root, marker, prepared) = fixture_in(true);
    let path = actor_ready(&prepared.request.program);
    let Started::Holder { mut child, .. } =
        launch(selected_actor(&prepared), &marker, &prepared, deadline()).unwrap()
    else {
        panic!("missing actor capability")
    };
    drop(prepared);
    ready(&path);
    child.stdin.take().unwrap().write_all(&[1]).unwrap();
    assert!(child.wait().unwrap().success());
}
// F/G/H: acquired->commit ownership handoff and a child which independently acquires the record.
#[test]
fn independent_game_claims_survive_holder_death_and_release_on_game_exit() {
    let (root, marker, prepared) = fixture();
    let entry = prepared.request.entry.clone();
    assert_exclusive(&entry, false);
    assert_noninheriting(&prepared);
    let Started::Holder { mut child, .. } =
        launch(selected_actor(&prepared), &marker, &prepared, deadline()).unwrap()
    else {
        panic!("actor capability missing")
    };
    assert_noninheriting(&prepared);
    drop(prepared);
    ready(&root.path().join("ready"));
    let mut input = child.stdin.take().unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
    assert_exclusive(&entry, false);
    // Discover the record from disk, not from the implementation's claimed-tree list.
    let record = fs::read_dir(entry.data.join("run-records"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let directory = open_directory(&record).unwrap();
    let registry = Registry::existing(&entry.data).unwrap();
    let protected = matches!(
        registry.lock(&directory, LockMode::Exclusive),
        Err(crate::runtime_leases::Error::Busy)
    );
    input.write_all(&[1]).unwrap();
    drop(input);
    released(&entry);
    assert!(
        protected,
        "independent game did not own its persistent record"
    );
    assert!(registry.lock(&directory, LockMode::Exclusive).is_ok());
}
#[test]
#[cfg(unix)]
fn holder_claims_survive_until_kernel_exit_including_native_exit_callbacks() {
    let (root, marker, prepared) = fixture();
    let entry = prepared.request.entry.clone();
    let exit_ready = root.path().join("exit-ready");
    let exit_stop = root.path().join("exit-stop");
    let mut command = selected_actor(&prepared);
    command
        .env(MODE, "exit-delay")
        .env("HORIZON_HOLDER_TEST_EXIT_READY", &exit_ready)
        .env("HORIZON_HOLDER_TEST_EXIT_STOP", &exit_stop);
    let Started::Holder { mut child, .. } =
        launch(command, &marker, &prepared, deadline()).unwrap()
    else {
        panic!("missing actor capability")
    };
    drop(prepared);
    ready(&root.path().join("ready"));
    child.stdin.take().unwrap().write_all(&[1]).unwrap();
    ready(&exit_ready);
    let registry = Registry::existing(&entry.data).unwrap();
    let protected = entry.trees.iter().all(|tree| {
        matches!(
            registry.lock(&tree.recheck().unwrap(), LockMode::Exclusive),
            Err(crate::runtime_leases::Error::Busy)
        )
    });
    // Release and reap the controlled callback even when checking a broken implementation.
    fs::write(exit_stop, b"stop").unwrap();
    assert!(child.wait().unwrap().success());
    assert!(
        protected,
        "holder dropped claims before native exit callbacks finished"
    );
    released(&entry);
}
#[test]
fn committed_holder_and_game_survive_original_gui_process_exit() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap();
    let status = actor_command()
        .env(MODE, "gui")
        .env("HORIZON_HOLDER_TEST_GUI_ROOT", &path)
        .status()
        .unwrap();
    assert!(status.success());
    ready(&path.join("ready"));
    let record = fs::read_dir(path.join("data/run-records"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let entry: Entry =
        serde_json::from_slice(&fs::read(record.join("launch.json")).unwrap()).unwrap();
    assert_exclusive(&entry, false);
    fs::write(path.join("stop"), b"stop").unwrap();
    released(&entry);
}
#[test]
fn precommit_eof_aborts_without_starting_game() {
    for wrong_commit in [false, true] {
        let (root, marker, prepared) = fixture();
        let entry = prepared.request.entry.clone();
        let mut endpoint = Endpoint::bind(&marker).unwrap();
        let mut child = selected_actor(&prepared)
            .env(ENV, serde_json::to_string(&marker).unwrap())
            .spawn()
            .unwrap();
        let stream = endpoint.accept(deadline()).unwrap();
        let channel = Channel::authenticate(stream, &marker, true, deadline()).unwrap();
        let channel = channel.send(&prepared.request).unwrap();
        let (channel, reply) = channel.receive::<Reply>().unwrap();
        assert!(matches!(reply, Reply::Acquired));
        if wrong_commit {
            drop(channel.send(&Reply::Acquired).unwrap());
        } else {
            drop(channel);
        }
        drop(child.stdin.take());
        assert_eq!(child.wait().unwrap().code(), Some(78));
        assert!(!root.path().join("ready").exists());
        drop(prepared);
        released(&entry);
    }
}
#[test]
fn commit_instead_of_acquired_is_refused_and_reaped() {
    let (_root, marker, prepared) = fixture();
    let mut command = selected_actor(&prepared);
    command.env(MODE, "wrong-ack");
    let result = launch(command, &marker, &prepared, deadline());
    assert!(matches!(result, Err(error) if error.kind() == io::ErrorKind::InvalidData));
}
#[test]
fn deadline_kills_and_reaps_late_holder_before_releasing_gui_claims() {
    let (root, marker, prepared) = fixture();
    let entry = prepared.request.entry.clone();
    let mut command = selected_actor(&prepared);
    command.env(MODE, "late");
    let started = Instant::now();
    let result = launch(
        command,
        &marker,
        &prepared,
        Instant::now() + Duration::from_millis(500),
    );
    assert!(matches!(result, Err(error) if error.kind() == io::ErrorKind::TimedOut));
    assert!(
        started.elapsed() < Duration::from_millis(1800),
        "late holder was not killed promptly"
    );
    assert_noninheriting(&prepared);
    drop(prepared);
    released(&entry);
    assert!(!root.path().join("ready").exists());
}
#[test]
fn failed_game_exec_releases_all_holder_claims() {
    let (_root, marker, mut prepared) = fixture();
    let entry = prepared.request.entry.clone();
    prepared.request.program.executable = "/no-such-program/ever".into();
    let Started::Holder { mut child, .. } =
        launch(selected_actor(&prepared), &marker, &prepared, deadline()).unwrap()
    else {
        panic!("actor capability missing")
    };
    assert_eq!(child.wait().unwrap().code(), Some(78));
    drop(prepared);
    released(&entry);
}
#[test]
#[cfg(target_os = "linux")]
fn cloexec_fork_overlap_is_conservatively_busy_until_exec_or_exit() {
    use std::{io::Read, os::fd::AsRawFd};
    let (_root, _marker, prepared) = fixture();
    let entry = prepared.request.entry.clone();
    assert_noninheriting(&prepared);
    let (mut ready_read, ready_write) = os_pipe::pipe().unwrap();
    let (control_read, mut control_write) = os_pipe::pipe().unwrap();
    let (ready_fd, read_fd, write_fd) = (
        ready_write.as_raw_fd(),
        control_read.as_raw_fd(),
        control_write.as_raw_fd(),
    );
    // SAFETY: the fork child uses only close/read/write/_exit, never Rust locks or allocation.
    let pid = unsafe { libc::fork() };
    assert!(pid >= 0);
    if pid == 0 {
        unsafe {
            libc::close(write_fd);
            libc::write(ready_fd, [1u8].as_ptr().cast(), 1);
            libc::read(read_fd, [0u8].as_mut_ptr().cast(), 1);
            libc::_exit(0);
        }
    }
    drop(ready_write);
    drop(control_read);
    ready_read.read_exact(&mut [0]).unwrap();
    drop(prepared);
    assert_exclusive(&entry, false);
    control_write.write_all(&[1]).unwrap();
    let mut status = 0;
    assert_eq!(unsafe { libc::waitpid(pid, &mut status, 0) }, pid);
    assert!(libc::WIFEXITED(status));
    released(&entry);
}
#[test]
fn malformed_entry_is_noncreating_and_refuses_unknown_fields() {
    let (root, _marker, prepared) = fixture();
    let mut entry = prepared.request.entry.clone();
    entry.data = root.path().join("absent");
    assert!(entry.acquire().is_err());
    assert!(!entry.data.exists());
    let mut value = serde_json::to_value(&entry).unwrap();
    value["version"] = 99.into();
    assert!(serde_json::from_value::<Entry>(value.clone()).is_err());
    value["version"] = crate::local_control::VERSION.into();
    value["unexpected"] = true.into();
    assert!(serde_json::from_value::<Entry>(value).is_err());
}
