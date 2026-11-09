//! Standalone receiver actor; no libtest threads before private bootstrap.
use horizon_mod_package::holder;
use std::{
    io::{Read, Write},
    path::PathBuf,
    time::{Duration, Instant},
};
#[cfg(unix)]
extern "C" fn delayed_exit() {
    std::fs::write(std::env::var_os("EXIT_READY").unwrap(), b"atexit").unwrap();
    let stop = PathBuf::from(std::env::var_os("EXIT_STOP").unwrap());
    let end = Instant::now() + Duration::from_secs(5);
    while !stop.is_file() {
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn main() {
    #[cfg(unix)]
    if std::env::var_os("EXIT_READY").is_some() {
        assert_eq!(unsafe { libc::atexit(delayed_exit) }, 0);
    }
    holder::enter();
    assert!(std::env::var_os(holder::ENV).is_none());
    let entry: serde_json::Value =
        serde_json::from_str(&std::env::var(holder::ENTRY).unwrap()).unwrap();
    assert!(entry.get("program").is_none());
    let ready = PathBuf::from(std::env::var_os("READY").unwrap());
    let partial = ready.with_extension("partial");
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    std::fs::write(&partial, serde_json::to_vec(&arguments).unwrap()).unwrap();
    std::fs::rename(partial, ready).unwrap();
    let _ = std::io::stdin().read_exact(&mut [0]);
    std::io::stdout().flush().unwrap();
}
