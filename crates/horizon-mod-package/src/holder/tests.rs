use super::*;
use crate::{local_control::Endpoint, locks::Release};
use std::{
    fs,
    io::Write,
    process::{Child, Stdio},
};
struct Scene {
    _root: tempfile::TempDir,
    marker: Marker,
    request: Request,
    claims: Vec<Claim>,
    snapshots: Vec<Snapshot>,
}
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(3)
}
fn actor() -> PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
        .join(format!("holder_actor{}", std::env::consts::EXE_SUFFIX))
}
impl Scene {
    fn new(native: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        let name = {
            use std::os::unix::ffi::OsStringExt;
            OsString::from_vec(vec![110, 255])
        };
        #[cfg(windows)]
        let name = {
            use std::os::windows::ffi::OsStringExt;
            OsString::from_wide(&[110, 0xd800])
        };
        let path = root.path().canonicalize().unwrap().join(&name);
        fs::create_dir(&path).unwrap();
        let data = path.join("data");
        let registry = Registry::with_release(&data, Release::Close).unwrap();
        let snapshots: Vec<_> = [("image", !native), ("world", false), ("sdk", true)]
            .into_iter()
            .map(|(name, complete)| {
                let directory = path.join(name);
                fs::create_dir(&directory).unwrap();
                if complete {
                    fs::write(directory.join(".installed"), b"").unwrap();
                }
                Snapshot::capture(&directory, complete).unwrap()
            })
            .collect();
        let image = path.join("image").join(actor().file_name().unwrap());
        fs::copy(actor(), &image).unwrap();
        let claims = snapshots
            .iter()
            .map(|snapshot| registry.lock(&snapshot.directory, LockMode::Shared))
            .collect::<Result<_, _>>()
            .unwrap();
        let entry = Entry {
            version: crate::local_control::VERSION,
            data,
            trees: snapshots
                .iter()
                .map(|snapshot| snapshot.tree.clone())
                .collect(),
        };
        let program = Program {
            executable: image.into_os_string(),
            arguments: vec!["".into(), "alpha beta".into(), name],
            environment: vec![
                (ENV.into(), "must-not-recurse".into()),
                ("READY".into(), path.join("ready").into_os_string()),
            ],
            directory: path.join("world"),
            raw_windows: None,
        };
        Self {
            _root: root,
            marker: Marker::new(path.join("endpoint")).unwrap(),
            request: Request { entry, program },
            claims,
            snapshots,
        }
    }
    fn ready(&self) -> PathBuf {
        PathBuf::from(&self.request.program.environment[1].1)
    }
    fn begin(&self, global: bool, delay_exit: bool) -> (Endpoint, Child, io::Result<Channel>) {
        let mut endpoint = Endpoint::bind(&self.marker).unwrap();
        let image = if global {
            actor().into_os_string()
        } else {
            self.request.program.executable.clone()
        };
        let mut command = Command::new(image);
        command
            .env(ENV, serde_json::to_string(&self.marker).unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if delay_exit {
            command
                .env("EXIT_READY", self.ready().with_extension("exit"))
                .env("EXIT_STOP", self.ready().with_extension("stop"));
        }
        let child = command.spawn().unwrap();
        let result = endpoint
            .accept(deadline())
            .and_then(|stream| Channel::authenticate(stream, &self.marker, true, deadline()))
            .and_then(|channel| channel.send(&self.request))
            .and_then(|channel| {
                let (channel, reply) = channel.receive::<Reply>()?;
                assert!(matches!(reply, Reply::Acquired));
                Ok(channel)
            });
        (endpoint, child, result)
    }
    fn release(&mut self) {
        self.claims.clear();
        self.snapshots.clear();
    }
}
fn ready(path: &Path) {
    let end = deadline();
    while !path.is_file() {
        assert!(Instant::now() < end, "actor did not reach expected phase");
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn private_version_is_refused_before_normal_startup() {
    let status = Command::new(actor())
        .env(ENV, r#"{"version":99}"#)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(78));
}
#[test]
fn malformed_entry_is_noncreating_and_versioned() {
    let scene = Scene::new(false);
    let mut entry = scene.request.entry.clone();
    entry.data = scene._root.path().join("absent");
    assert!(entry.acquire().is_err());
    assert!(!entry.data.exists());
    let mut value = serde_json::to_value(entry).unwrap();
    for (key, value_bad) in [
        ("version", serde_json::json!(99)),
        ("unexpected", serde_json::json!(true)),
    ] {
        let mut bad = value.clone();
        bad[key] = value_bad;
        assert!(serde_json::from_value::<Entry>(bad).is_err());
    }
    value["trees"] = serde_json::json!([]);
    assert!(
        serde_json::from_value::<Entry>(value)
            .unwrap()
            .acquire()
            .is_err()
    );
}
#[test]
fn replaced_tree_completion_or_unreserved_image_never_starts_game() {
    for change in 0..3 {
        let mut scene = Scene::new(false);
        scene.snapshots.clear(); // Model stale GUI descriptions without directory delete-denial.
        if change == 0 {
            let world = &scene.request.entry.trees[1].path;
            fs::rename(world, world.with_extension("old")).unwrap();
            fs::create_dir(world).unwrap();
            let registry =
                Registry::with_release(&scene.request.entry.data, Release::Close).unwrap();
            drop(
                registry
                    .lock(&open_directory(world).unwrap(), LockMode::Shared)
                    .unwrap(),
            );
        }
        if change == 1 {
            let complete = scene.request.entry.trees[0].path.join(".installed");
            fs::remove_file(&complete).unwrap();
            fs::write(complete, b"").unwrap();
        }
        let (_endpoint, mut child, result) = scene.begin(change == 2, false);
        assert!(result.is_err());
        drop(child.stdin.take());
        assert_eq!(child.wait().unwrap().code(), Some(78));
        assert!(!scene.ready().exists());
    }
}
#[test]
fn eof_wrong_commit_and_postcommit_replacement_refuse_even_wrappers() {
    for change in 0..4 {
        let mut scene = Scene::new(false);
        if change == 3 {
            scene.request.program.raw_windows = Some("must not execute".into());
        }
        let (_endpoint, mut child, result) = scene.begin(false, false);
        let channel = result.unwrap();
        if change == 2 {
            let complete = scene.request.entry.trees[0].path.join(".installed");
            fs::remove_file(&complete).unwrap();
            fs::write(complete, b"").unwrap();
        }
        match change {
            0 => drop(channel),
            1 => drop(channel.send(&Reply::Acquired).unwrap()),
            _ => drop(channel.send(&Reply::Commit).unwrap()),
        }
        drop(child.stdin.take());
        assert_eq!(child.wait().unwrap().code(), Some(78));
        assert!(!scene.ready().exists());
    }
}
#[test]
fn native_paths_arguments_and_scrubbed_metadata_survive_committed_launch() {
    for native in [false, true] {
        let mut scene = Scene::new(native);
        let (_endpoint, mut child, result) = scene.begin(false, false);
        drop(result.unwrap().send(&Reply::Commit).unwrap());
        scene.release();
        ready(&scene.ready());
        let args: Vec<OsString> =
            serde_json::from_slice(&fs::read(scene.ready()).unwrap()).unwrap();
        assert_eq!(args, scene.request.program.arguments);
        child.stdin.take().unwrap().write_all(&[1]).unwrap();
        assert!(child.wait().unwrap().success());
    }
}
#[test]
#[cfg(target_os = "linux")]
fn independently_retained_objects_and_claims_survive_native_exit_callbacks() {
    use std::os::unix::fs::MetadataExt as Unix;
    let mut scene = Scene::new(false);
    let directory = fs::metadata(&scene.request.entry.trees[0].path).unwrap();
    let complete = fs::metadata(scene.request.entry.trees[0].path.join(".installed")).unwrap();
    let (_endpoint, mut child, result) = scene.begin(false, true);
    drop(result.unwrap().send(&Reply::Commit).unwrap());
    scene.release();
    ready(&scene.ready());
    let held: Vec<_> = fs::read_dir(format!("/proc/{}/fd", child.id()))
        .unwrap()
        .filter_map(|entry| fs::metadata(entry.ok()?.path()).ok())
        .map(|meta| (Unix::dev(&meta), Unix::ino(&meta)))
        .collect();
    child.stdin.take().unwrap().write_all(&[1]).unwrap();
    ready(&scene.ready().with_extension("exit"));
    let registry = Registry::existing(&scene.request.entry.data).unwrap();
    let protected = scene.request.entry.trees.iter().all(|tree| {
        matches!(
            registry.lock(&tree.recheck().unwrap(), LockMode::Exclusive),
            Err(crate::runtime_leases::Error::Busy)
        )
    });
    fs::write(scene.ready().with_extension("stop"), b"stop").unwrap();
    assert!(child.wait().unwrap().success());
    assert!(protected, "claims ended before native exit callbacks");
    assert!(held.contains(&(Unix::dev(&directory), Unix::ino(&directory))));
    assert!(held.contains(&(Unix::dev(&complete), Unix::ino(&complete))));
}
