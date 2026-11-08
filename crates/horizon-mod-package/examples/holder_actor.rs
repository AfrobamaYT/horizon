//! Standalone developer test actor: no libtest threads before private bootstrap.
use horizon_mod_package::{
    holder::{self, Entry},
    local_control::{Channel, Marker, connect},
};
use std::{
    io::{Read, Write},
    time::{Duration, Instant},
};
#[path = "holder_actor/scene.rs"]
mod scene;
#[cfg(windows)]
#[path = "holder_actor/windows_job.rs"]
mod windows_job;
const MODE: &str = "HORIZON_HOLDER_TEST_MODE";
const READY: &str = "HORIZON_HOLDER_TEST_READY";
#[cfg(unix)]
extern "C" fn delayed_exit() {
    let ready =
        std::path::PathBuf::from(std::env::var_os("HORIZON_HOLDER_TEST_EXIT_READY").unwrap());
    let stop = std::path::PathBuf::from(std::env::var_os("HORIZON_HOLDER_TEST_EXIT_STOP").unwrap());
    std::fs::write(ready, b"atexit").unwrap();
    scene::until(|| stop.is_file());
}
fn main() {
    if std::env::var_os(holder::ENV).is_some() {
        #[cfg(unix)]
        if std::env::var_os(MODE).as_deref() == Some(std::ffi::OsStr::new("exit-delay")) {
            // Model a native library's atexit callback, which runs after Rust main finishes.
            assert_eq!(unsafe { libc::atexit(delayed_exit) }, 0);
        }
        let mode = std::env::var_os(MODE);
        if mode.as_deref() == Some(std::ffi::OsStr::new("late"))
            || mode.as_deref() == Some(std::ffi::OsStr::new("swap-acquired"))
            || mode.as_deref() == Some(std::ffi::OsStr::new("wrong-ack"))
        {
            let marker: Marker =
                serde_json::from_str(&std::env::var(holder::ENV).unwrap()).unwrap();
            let deadline = Instant::now() + Duration::from_secs(3);
            let channel = Channel::authenticate(
                connect(&marker, deadline).unwrap(),
                &marker,
                false,
                deadline,
            )
            .unwrap();
            let (channel, request) = channel.receive::<serde_json::Value>().unwrap();
            let entry: Entry = serde_json::from_value(request["entry"].clone()).unwrap();
            let _claims = entry.acquire().unwrap();
            if mode.as_deref() == Some(std::ffi::OsStr::new("wrong-ack")) {
                let channel = channel.send(&"Commit").unwrap();
                let _ = channel.receive::<String>();
            } else if mode.as_deref() == Some(std::ffi::OsStr::new("swap-acquired")) {
                let complete = std::env::current_exe()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .join(".installed");
                std::fs::remove_file(&complete).unwrap();
                std::fs::write(complete, b"").unwrap();
                let channel = channel.send(&"Acquired").unwrap();
                let _ = channel.receive::<String>();
            } else {
                std::thread::sleep(Duration::from_secs(2));
                let _ = channel.send(&"Acquired");
            }
            std::process::exit(0);
        }
        holder::enter();
        panic!("private bootstrap reached ordinary initialization");
    }
    std::hint::black_box(&holder::CAPABILITY);
    #[cfg(windows)]
    if std::env::var_os(MODE).as_deref() == Some(std::ffi::OsStr::new("job")) {
        windows_job::run();
        return;
    }
    let mode = std::env::var_os(MODE);
    if mode.as_deref() == Some(std::ffi::OsStr::new("gui")) {
        let root =
            std::path::PathBuf::from(std::env::var_os("HORIZON_HOLDER_TEST_GUI_ROOT").unwrap());
        let _started = scene::start(&root, "game-event");
        std::process::exit(0);
    }
    if mode.as_deref() == Some(std::ffi::OsStr::new("wrapper")) {
        std::fs::write(std::env::var_os(READY).unwrap(), b"wrapper-ran").unwrap();
        return;
    }
    let event = mode.as_deref() == Some(std::ffi::OsStr::new("game-event"));
    if mode.as_deref() != Some(std::ffi::OsStr::new("game")) && !event {
        return;
    }
    assert!(std::env::var_os(holder::ENV).is_none());
    let encoded = std::env::var(holder::ENTRY).unwrap();
    assert!(
        serde_json::from_str::<serde_json::Value>(&encoded)
            .unwrap()
            .get("program")
            .is_none()
    );
    let entry: Entry = serde_json::from_str(&encoded).unwrap();
    let _claims = entry.acquire().unwrap();
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let ready = std::path::PathBuf::from(std::env::var_os(READY).unwrap());
    let partial = ready.with_extension("partial");
    std::fs::write(&partial, serde_json::to_vec(&arguments).unwrap()).unwrap();
    std::fs::rename(partial, ready).unwrap();
    if event {
        let path = std::path::PathBuf::from(std::env::var_os("HORIZON_HOLDER_TEST_STOP").unwrap());
        scene::until(|| path.is_file());
    } else {
        let _ = std::io::stdin().read_exact(&mut [0]);
    }
    std::io::stdout().flush().unwrap();
    std::process::exit(0);
}
