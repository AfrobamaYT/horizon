use super::*;
use std::fs;

struct Home(PathBuf);
impl Home {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let home = std::env::temp_dir().join(format!(
            "horizon-cleanup-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&home).unwrap();
        Self(home)
    }
    fn file(&self, relative: &str) -> PathBuf {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"old compiler output").unwrap();
        path
    }
    fn dir(&self) -> Dir {
        Dir::open_ambient_dir(&self.0, cap_std::ambient_authority()).unwrap()
    }
}
impl Drop for Home {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn late_cargo_cannot_lock_an_existing_profile_and_new_profiles_survive() {
    let home = Home::new();
    let output = home.file("target/debug/output");
    let directory = home.dir();
    let plan = Plan::pin(&directory).unwrap();
    // The reviewer's late-start callback window: after pin/enumeration and before deletion.
    let profile_lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(home.0.join("target/debug/.cargo-lock"))
        .unwrap();
    assert!(matches!(
        profile_lock.try_lock(),
        Err(fs::TryLockError::WouldBlock)
    ));
    let target_lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(home.0.join("target/.cargo-lock"))
        .unwrap();
    assert!(matches!(
        target_lock.try_lock(),
        Err(fs::TryLockError::WouldBlock)
    ));
    let late = home.file("target/new-profile/new-output");
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(home.0.join("target/new-profile/.cargo-lock"))
        .unwrap();
    lock.lock().unwrap();
    plan.clear(&directory).unwrap();
    assert!(!output.exists());
    assert!(late.exists());
    assert!(home.0.join("target/debug/.cargo-lock").exists());
    assert!(profile_lock.try_lock().is_ok());
    profile_lock.unlock().unwrap();
}

#[test]
fn rewritten_snapshot_is_retained() {
    let home = Home::new();
    let output = home.file("target/debug/output");
    let directory = home.dir();
    let plan = Plan::pin(&directory).unwrap();
    fs::write(&output, b"rewritten after snapshot; must survive").unwrap();
    plan.clear(&directory).unwrap();
    assert_eq!(
        fs::read(output).unwrap(),
        b"rewritten after snapshot; must survive"
    );
}

#[cfg(unix)]
#[test]
fn swapped_directory_and_file_links_never_delete_external_data() {
    let home = Home::new();
    let outside = Home::new();
    let precious = outside.file("world");
    home.file("cache/output");
    let selected = home.file("selected");
    let directory = home.dir();
    let plan = Plan::pin(&directory).unwrap();
    fs::rename(home.0.join("cache"), home.0.join("moved")).unwrap();
    std::os::unix::fs::symlink(&outside.0, home.0.join("cache")).unwrap();
    fs::remove_file(&selected).unwrap();
    std::os::unix::fs::symlink(&precious, &selected).unwrap();
    plan.clear(&directory).unwrap();
    assert_eq!(fs::read(precious).unwrap(), b"old compiler output");
    assert!(selected
        .symlink_metadata()
        .unwrap()
        .file_type()
        .is_symlink());
}

#[test]
fn existing_profile_lock_refuses_pin_before_any_deletion() {
    let home = Home::new();
    let output = home.file("target/debug/output");
    let lock = home.file("target/debug/.cargo-lock");
    let held = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(lock)
        .unwrap();
    held.lock().unwrap();
    assert!(matches!(Plan::pin(&home.dir()), Err(Error::Busy)));
    assert!(output.exists());
}

#[test]
fn pin_file_clears_only_one_file_and_refuses_lock_or_nested_names() {
    let home = Home::new();
    let selected = home.file("selected");
    let kept = home.file("kept");
    home.file("nested/output");
    home.file(".build.lock");
    let directory = home.dir();
    for name in [
        ".cargo-lock",
        ".package-cache",
        ".package-cache-mutate",
        ".build.lock",
        "nested/output",
    ] {
        assert!(Plan::pin_file(&directory, Path::new(name)).is_err());
    }
    Plan::pin_file(&directory, Path::new("selected"))
        .unwrap()
        .clear(&directory)
        .unwrap();
    assert!(!selected.exists());
    assert!(kept.exists());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&kept, home.0.join("link")).unwrap();
        assert!(Plan::pin_file(&directory, Path::new("link")).is_err());
        assert!(kept.exists());
    }
}

#[cfg(unix)]
#[test]
fn flat_cache_fits_a_small_descriptor_limit() {
    if std::env::var_os("HORIZON_CLEANUP_FD_CHILD").is_none() {
        use std::os::unix::process::CommandExt;
        let mut child = std::process::Command::new(std::env::current_exe().unwrap());
        child
            .args([
                "--exact",
                "cache_cleanup::tests::flat_cache_fits_a_small_descriptor_limit",
            ])
            .env("HORIZON_CLEANUP_FD_CHILD", "1");
        // Only the isolated child gets the lowered limit, never another test's process.
        unsafe {
            child.pre_exec(|| {
                let limit = libc::rlimit {
                    rlim_cur: 1024,
                    rlim_max: 1024,
                };
                if libc::setrlimit(libc::RLIMIT_NOFILE, &limit) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let output = child.output().unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let home = Home::new();
    for index in 0..5000 {
        home.file(&format!("output-{index}"));
    }
    let directory = home.dir();
    let plan = Plan::pin(&directory).unwrap();
    assert_eq!(plan.outputs.len(), 5000);
    plan.clear(&directory).unwrap();
    assert!(!home.0.join("output-4999").exists());
}
