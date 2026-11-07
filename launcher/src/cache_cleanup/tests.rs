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
