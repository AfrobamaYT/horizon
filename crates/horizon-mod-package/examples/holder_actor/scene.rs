use horizon_mod_package::{
    holder::{self, Prepared, Program, Started},
    local_control::Marker,
};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
pub fn until(mut ready: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(5);
    while !ready() {
        assert!(Instant::now() < end, "controlled actor timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}
pub fn start(root: &Path, mode: &str) -> Started {
    fs::create_dir(root.join("image")).unwrap();
    fs::create_dir(root.join("world")).unwrap();
    fs::write(root.join("image/.installed"), b"").unwrap();
    let source = std::env::current_exe().unwrap();
    let image = root.join("image").join(source.file_name().unwrap());
    fs::copy(source, &image).unwrap();
    let program = Program {
        executable: image.clone().into_os_string(),
        arguments: vec![],
        environment: vec![
            ("HORIZON_HOLDER_TEST_MODE".into(), mode.into()),
            (
                "HORIZON_HOLDER_TEST_READY".into(),
                root.join("ready").into_os_string(),
            ),
            (
                "HORIZON_HOLDER_TEST_STOP".into(),
                root.join("stop").into_os_string(),
            ),
        ],
        directory: root.join("world"),
        raw_windows: None,
    };
    let prepared = Prepared::new(
        root.join("data"),
        vec![(root.join("image"), true), (root.join("world"), false)],
        program,
    )
    .unwrap();
    let marker = Marker::new(root.join("endpoint")).unwrap();
    let mut command = Command::new(image);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // GUI bootstrap claims drop on return, strictly after the acquired->commit handshake.
    holder::launch(
        command,
        &marker,
        &prepared,
        Instant::now() + Duration::from_secs(5),
    )
    .unwrap()
}
