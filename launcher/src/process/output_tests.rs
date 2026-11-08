use cap_std::fs::Dir;
use horizon_mod_package::output::{Capture, Tail};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn wait(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !path.exists() {
        assert!(Instant::now() < deadline, "capture rendezvous timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn directory(path: &Path) -> Dir {
    Dir::open_ambient_dir(path, cap_std::ambient_authority()).unwrap()
}

#[test]
fn durable_raw_streams_rotate_without_a_tailer_and_drain_is_bounded() {
    for mode in ["rotate", "descendant"] {
        let root = tempfile::tempdir().unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "process::output_tests::actor", "--nocapture"])
            .env("HORIZON_CAPTURE_TEST_ROOT", root.path())
            .env("HORIZON_CAPTURE_TEST_MODE", mode)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        if mode == "rotate" {
            wait(&root.path().join("ready"));
            let mut tail = Tail::new(directory(root.path()), "stdout.log");
            let mut early = [0; 6];
            let deadline = Instant::now() + Duration::from_secs(30);
            while tail.read(&mut early).unwrap() == 0 {
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(5));
            }
            assert_eq!(&early, b"early\n");
            drop(tail);
            std::fs::write(root.path().join("go"), b"go").unwrap();
        }
        assert!(child.wait().unwrap().success());
        if mode == "descendant" {
            continue;
        }
        for (name, byte, count, ending) in [
            ("stdout.log", b'A', 23, b"after\nraw\n".as_slice()),
            ("stderr.log", b'B', 23, b"err-after\nraw-err\n".as_slice()),
        ] {
            let previous = std::fs::read(root.path().join(format!("{name}.1"))).unwrap();
            assert_eq!(previous.len(), 16 * 1024 * 1024);
            assert!(previous.iter().all(|&value| value == byte));
            let mut expected = vec![byte; count];
            expected.extend_from_slice(ending);
            assert_eq!(std::fs::read(root.path().join(name)).unwrap(), expected);
        }
    }
}

#[test]
fn tail_drains_retained_history_then_switches_without_pinning_files() {
    let root = tempfile::tempdir().unwrap();
    let current = root.path().join("stdout.log");
    let previous = root.path().join("stdout.log.1");
    std::fs::write(&current, b"old").unwrap();
    let mut tail = Tail::new(directory(root.path()), "stdout.log");
    let mut buffer = [0; 4];
    assert_eq!(tail.read(&mut buffer[..1]).unwrap(), 1);
    assert_eq!(buffer[0], b'o');
    std::fs::rename(&current, &previous).unwrap();
    std::fs::write(&current, b"new").unwrap();
    assert_eq!(tail.read(&mut buffer).unwrap(), 2);
    assert_eq!(&buffer[..2], b"ld");
    assert_eq!(tail.read(&mut buffer).unwrap(), 3);
    assert_eq!(&buffer[..3], b"new");
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        assert!(std::fs::File::options()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(&current)
            .is_ok());
    }
    std::fs::remove_file(previous).unwrap();
    std::fs::remove_file(current).unwrap();
}

#[test]
fn actor() {
    let Some(mode) = std::env::var_os("HORIZON_CAPTURE_TEST_MODE") else {
        return;
    };
    if mode == "hold" {
        std::thread::sleep(Duration::from_secs(30));
        return;
    }
    let root = PathBuf::from(std::env::var_os("HORIZON_CAPTURE_TEST_ROOT").unwrap());
    let capture = Capture::install(&directory(&root)).unwrap();
    if mode == "descendant" {
        let mut descendant = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "process::output_tests::actor", "--nocapture"])
            .env("HORIZON_CAPTURE_TEST_MODE", "hold")
            .spawn()
            .unwrap();
        let started = Instant::now();
        drop(capture);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "waited for a descendant's stdout"
        );
        descendant.kill().unwrap();
        descendant.wait().unwrap();
        return;
    }
    println!("early");
    std::fs::write(root.join("ready"), b"ready").unwrap();
    wait(&root.join("go"));
    let a = [b'A'; 64 * 1024];
    let b = [b'B'; 64 * 1024];
    for _ in 0..768 {
        std::io::stdout().write_all(&a).unwrap();
        std::io::stderr().write_all(&b).unwrap();
    }
    std::io::stdout().write_all(&[b'A'; 17]).unwrap();
    std::io::stderr().write_all(&[b'B'; 23]).unwrap();
    println!("after");
    eprintln!("err-after");
    // SAFETY: these small immutable buffers are valid for their complete lengths.
    unsafe {
        assert_eq!(libc::write(1, b"raw\n".as_ptr().cast(), 4), 4);
        assert_eq!(libc::write(2, b"raw-err\n".as_ptr().cast(), 8), 8);
    }
    drop(capture);
    let reopened = Capture::install(&directory(&root)).unwrap();
    drop(reopened);
}
