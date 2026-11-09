use super::*;

#[test]
fn exported_server_cache_is_retained_without_launch_exclusion() {
    let home = tempfile::tempdir().unwrap();
    for index in 0..6 {
        let build = home.path().join(format!("build-{index}"));
        fs::create_dir(&build).unwrap();
        fs::write(build.join("server"), b"directly executable cached runtime").unwrap();
        let marker = fs::File::create(build.join(".installed")).unwrap();
        marker
            .set_times(
                fs::FileTimes::new().set_modified(
                    std::time::UNIX_EPOCH + std::time::Duration::from_secs(index + 1),
                ),
            )
            .unwrap();
    }
    prune_server_cache(home.path(), "build-5");
    for index in 0..6 {
        let build = home.path().join(format!("build-{index}"));
        assert!(build.join(".installed").exists());
        assert_eq!(
            fs::read(build.join("server")).unwrap(),
            b"directly executable cached runtime"
        );
    }
}
