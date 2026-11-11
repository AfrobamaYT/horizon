use super::*;
use horizon_protocol::payload_negotiation::{Flow, Manifest, RegistrationPhase};
use std::{sync::mpsc, time::Duration};

fn connection() -> Arc<Connection> {
    let mut manifest = Manifest::builder();
    manifest
        .register(
            RegistrationPhase::Play,
            "test:raw",
            Flow::ToServer,
            "1",
            true,
        )
        .unwrap();
    let local = Arc::new(Local::new(manifest.finish().unwrap(), None));
    let connection = Connection::new(local.clone(), Side::Client);
    connection.accept(local.manifest.entries()).unwrap();
    connection
}
#[test]
fn configuration_acknowledgement_waits_for_main_root_commit() {
    let connection = connection();
    let old = connection.epoch().unwrap();
    let pending = old.main_task();
    let (started, start) = mpsc::channel();
    let (done, result) = mpsc::channel();
    let owner = connection.clone();
    let thread = std::thread::spawn(move || {
        started.send(()).unwrap();
        done.send(owner.finish_configuration()).unwrap();
    });
    start.recv().unwrap();
    assert!(result.recv_timeout(Duration::from_millis(20)).is_err());
    drop(pending);
    result
        .recv_timeout(Duration::from_secs(2))
        .unwrap()
        .unwrap();
    thread.join().unwrap();
    assert!(!old.active());
    assert_eq!(connection.epoch().unwrap().phase, Phase::Play);
    connection.begin_configuration();
    assert!(connection.epoch().is_none());
}
#[test]
fn close_releases_a_waiting_configuration_without_running_queued_work() {
    let connection = connection();
    let old = connection.epoch().unwrap();
    let pending = old.main_task();
    let (done, result) = mpsc::channel();
    let owner = connection.clone();
    let thread = std::thread::spawn(move || {
        done.send(owner.finish_configuration()).unwrap();
    });
    connection.close();
    assert!(
        result
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .is_err()
    );
    assert!(!old.active());
    assert!(connection.epoch().is_none());
    drop(pending);
    thread.join().unwrap();
}
