use super::scene;
use horizon_mod_package::{
    holder::Started,
    locks::LockMode,
    runtime_leases::{Error, Registry},
};
use std::{fs, io::Write, path::Path};
pub fn run() {
    let root = std::path::PathBuf::from(std::env::var_os("HORIZON_HOLDER_TEST_JOB_ROOT").unwrap());
    scene::until(|| root.join("assigned").is_file());
    let Started::Holder {
        mut child,
        degraded,
    } = scene::start(&root, "game")
    else {
        panic!("missing actor capability")
    };
    assert!(
        degraded,
        "did not execute the real breakaway refusal/retry branch"
    );
    scene::until(|| root.join("ready").is_file());
    let mut input = child.stdin.take().unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
    let data = root.join("data");
    let record = fs::read_dir(data.join("run-records"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let directory =
        cap_std::fs::Dir::open_ambient_dir(Path::new(&record), cap_std::ambient_authority())
            .unwrap();
    let registry = Registry::existing(&data).unwrap();
    assert!(
        matches!(
            registry.lock(&directory, LockMode::Exclusive),
            Err(Error::Busy)
        ),
        "game lost the record after holder death while job stayed open"
    );
    input.write_all(&[1]).unwrap();
    drop(input);
    scene::until(|| registry.lock(&directory, LockMode::Exclusive).is_ok());
    fs::write(root.join("proof"), b"degraded-held-released").unwrap();
}
