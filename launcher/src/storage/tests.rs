use super::*;

struct Home(PathBuf);
impl Home {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "horizon-storage-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn seed(&self, path: &str) -> PathBuf {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, vec![7; 8192]).unwrap();
        path
    }
}
impl Drop for Home {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn hard_links_count_once() {
    let home = Home::new();
    let file = home.seed("rust/a");
    let before = allocated_bytes(&home.0.join("rust")).unwrap();
    fs::hard_link(file, home.0.join("rust/b")).unwrap();
    assert_eq!(allocated_bytes(&home.0.join("rust")).unwrap(), before);
}

#[test]
fn clears_only_fixed_cache_and_sweeps_detached_tree() {
    let home = Home::new();
    home.seed("native/source/revision/lib.rs");
    home.seed("settings.json");
    home.seed("worlds/keep/level.dat");
    let storage = Storage::at(&home.0).unwrap();
    assert!(storage.bytes(BuildCache::Source).unwrap() > 0);
    assert!(storage.clear(BuildCache::Source).unwrap() > 0);
    assert!(!home.0.join("native/source").exists());
    assert!(home.0.join("settings.json").exists());
    assert!(home.0.join("worlds/keep/level.dat").exists());
    home.seed(".trash-interrupted/file");
    storage.sweep().unwrap();
    assert!(!home.0.join(".trash-interrupted").exists());
}

#[test]
fn cleanup_obeys_build_lease() {
    let home = Home::new();
    let _storage = Storage::at(&home.0).unwrap();
    assert!(matches!(Storage::at(&home.0), Err(Error::Busy)));
}

#[cfg(unix)]
#[test]
fn refuses_linked_ancestor_and_descendant_before_deletion() {
    use std::os::unix::fs::symlink;
    let home = Home::new();
    home.seed("worlds/keep");
    let storage = Storage::at(&home.0).unwrap();
    symlink(home.0.join("worlds"), home.0.join("native/source")).unwrap();
    assert!(storage.clear(BuildCache::Source).is_err());
    fs::remove_file(home.0.join("native/source")).unwrap();
    home.seed("native/source/good");
    symlink(home.0.join("worlds"), home.0.join("native/source/bad")).unwrap();
    assert!(storage.clear(BuildCache::Source).is_err());
    assert!(home.0.join("native/source/good").exists());
    assert!(home.0.join("worlds/keep").exists());
    fs::rename(home.0.join("native"), home.0.join("moved")).unwrap();
    symlink(home.0.join("moved"), home.0.join("native")).unwrap();
    assert!(storage.clear(BuildCache::Source).is_err());
}

#[test]
fn read_only_file_can_be_removed() {
    let home = Home::new();
    let file = home.seed("native/tools/tool");
    let mut permissions = fs::metadata(&file).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(file, permissions).unwrap();
    let storage = Storage::at(&home.0).unwrap();
    storage.clear(BuildCache::Tools).unwrap();
    assert!(!home.0.join("native/tools").exists());
}

#[cfg(windows)]
#[test]
fn held_open_file_is_kept_for_next_sweep() {
    use std::os::windows::fs::OpenOptionsExt;
    let home = Home::new();
    let file = home.seed("native/source/file");
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(file)
        .unwrap();
    let storage = Storage::at(&home.0).unwrap();
    assert!(storage.clear(BuildCache::Source).is_err());
    drop(held);
    storage.sweep().unwrap();
    storage.clear(BuildCache::Source).unwrap();
}

#[cfg(unix)]
#[test]
fn root_replacement_cannot_redirect_sweep() {
    let home = Home::new();
    let outside = Home::new();
    outside.seed(".trash-precious/world");
    let storage = Storage::at(&home.0).unwrap();
    let moved = home.0.with_extension("moved");
    fs::rename(&home.0, &moved).unwrap();
    std::os::unix::fs::symlink(&outside.0, &home.0).unwrap();
    assert!(storage.sweep().is_err());
    assert!(outside.0.join(".trash-precious/world").exists());
    fs::remove_file(&home.0).unwrap();
    fs::rename(moved, &home.0).unwrap();
}

#[test]
fn child_cargo_locks_prevent_clear_and_survive_idle_cleanup() {
    for (cache, directory) in [
        (BuildCache::InternalsSync, "native/internals-sync/rev"),
        (BuildCache::Tools, "native/tools/rev"),
    ] {
        let home = Home::new();
        let lock_path = home.seed(&format!("{directory}/target/debug/.cargo-lock"));
        let output = home.seed(&format!("{directory}/target/debug/output"));
        let held = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock_path)
            .unwrap();
        held.lock().unwrap();
        let storage = Storage::at(&home.0).unwrap();
        assert!(matches!(storage.clear(cache), Err(Error::Busy)));
        assert!(output.exists());
        held.unlock().unwrap();
        storage.clear(cache).unwrap();
        assert!(!output.exists());
        assert!(lock_path.exists());
    }
}
