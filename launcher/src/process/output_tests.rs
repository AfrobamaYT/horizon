use cap_std::fs::Dir;
use horizon_mod_package::output::Capture;
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
    for mode in ["rotate", "descendant", "privacy", "buffered"] {
        if cfg!(windows) && mode == "privacy" {
            continue;
        }
        let root = tempfile::tempdir().unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "process::output_tests::actor", "--nocapture"])
            .env("HORIZON_CAPTURE_TEST_ROOT", root.path())
            .env("HORIZON_CAPTURE_TEST_MODE", mode)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        if mode == "rotate" {
            wait(&root.path().join("ready"));
            let deadline = Instant::now() + Duration::from_secs(30);
            while std::fs::read(root.path().join("stdout.log")).unwrap() != b"early\n" {
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(5));
            }
            std::fs::write(root.path().join("go"), b"go").unwrap();
        }
        assert!(child.wait().unwrap().success());
        if mode != "rotate" {
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
fn actor() {
    let Some(mode) = std::env::var_os("HORIZON_CAPTURE_TEST_MODE") else {
        return;
    };
    if mode == "hold" {
        std::thread::sleep(Duration::from_secs(30));
        return;
    }
    let root = PathBuf::from(std::env::var_os("HORIZON_CAPTURE_TEST_ROOT").unwrap());
    #[cfg(unix)]
    if mode == "privacy" {
        use std::os::unix::fs::PermissionsExt;
        let current = root.join("stdout.log");
        std::fs::write(root.join("sibling"), b"keep").unwrap();
        std::os::unix::fs::symlink("sibling", &current).unwrap();
        assert!(Capture::install(&directory(&root)).is_err());
        assert_eq!(std::fs::read(root.join("sibling")).unwrap(), b"keep");
        std::fs::remove_file(&current).unwrap();
        std::fs::write(&current, b"old").unwrap();
        std::fs::write(root.join("stdout.log.1"), b"previous").unwrap();
        for name in ["stdout.log", "stdout.log.1"] {
            std::fs::set_permissions(root.join(name), std::fs::Permissions::from_mode(0o644))
                .unwrap();
        }
        drop(Capture::install(&directory(&root)).unwrap());
        assert_eq!(std::fs::read(&current).unwrap(), b"old");
        for name in ["stdout.log", "stdout.log.1", "stderr.log"] {
            assert_eq!(
                std::fs::metadata(root.join(name))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        return;
    }
    let capture = Capture::install(&directory(&root)).unwrap();
    if mode == "buffered" {
        print!("rust-buffered");
        unsafe {
            libc::printf(c"c-buffered".as_ptr());
        }
        drop(capture);
        assert_eq!(
            std::fs::read(root.join("stdout.log")).unwrap(),
            b"rust-bufferedc-buffered"
        );
        return;
    }
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
        std::thread::sleep(Duration::from_millis(5)); // Non-overflow rotation case.
    }
    std::io::stdout().write_all(&[b'A'; 17]).unwrap();
    std::io::stderr().write_all(&[b'B'; 23]).unwrap();
    println!("after");
    eprintln!("err-after");
    // SAFETY: these small immutable buffers are valid for their complete lengths.
    #[cfg(unix)]
    unsafe {
        assert_eq!(libc::write(1, b"raw\n".as_ptr().cast(), 4), 4);
        assert_eq!(libc::write(2, b"raw-err\n".as_ptr().cast(), 8), 8);
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::BorrowedHandle;
        use windows_sys::Win32::System::Console::{
            GetStdHandle, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
        };
        for (channel, bytes) in [
            (STD_OUTPUT_HANDLE, b"raw\n".as_slice()),
            (STD_ERROR_HANDLE, b"raw-err\n"),
        ] {
            // SAFETY: capture installed valid handles and stays alive throughout each borrow.
            let handle = unsafe { BorrowedHandle::borrow_raw(GetStdHandle(channel)) };
            let mut file = std::fs::File::from(handle.try_clone_to_owned().unwrap());
            file.write_all(bytes).unwrap(); // File writes use WriteFile, not CRT descriptors.
        }
    }
    drop(capture);
    drop(Capture::install(&directory(&root)).unwrap());
}
