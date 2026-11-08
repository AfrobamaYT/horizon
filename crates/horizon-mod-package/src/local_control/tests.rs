use super::*;
use std::os::unix::ffi::OsStrExt;

fn marker() -> (tempfile::TempDir, Marker) {
    let root = tempfile::tempdir().unwrap();
    let parent = root.path().canonicalize().unwrap();
    let directory = parent.join(std::ffi::OsStr::from_bytes(b"fresh-\xff"));
    let marker = Marker::new(directory).unwrap();
    (root, marker)
}
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(3)
}
#[derive(Debug, Serialize, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
enum Message {
    Acquired,
    Commit,
    Status(i32),
}

#[test]
fn authenticated_typed_exchange_consumes_the_single_listener() {
    let (_root, marker) = marker();
    let mut endpoint = Endpoint::bind(&marker).unwrap();
    let other = marker.clone();
    let client = std::thread::spawn(move || {
        let stream = connect(&other, deadline()).unwrap();
        let channel = Channel::authenticate(stream, &other, false, deadline()).unwrap();
        let (channel, message) = channel.receive::<Message>().unwrap();
        assert_eq!(message, Message::Acquired);
        let channel = channel.send(&Message::Commit).unwrap();
        let (_channel, message) = channel.receive::<Message>().unwrap();
        assert_eq!(message, Message::Status(37));
    });
    let stream = endpoint.accept(deadline()).unwrap();
    assert_eq!(
        endpoint.accept(deadline()).err().unwrap().kind(),
        io::ErrorKind::NotConnected
    );
    let channel = Channel::authenticate(stream, &marker, true, deadline()).unwrap();
    let channel = channel.send(&Message::Acquired).unwrap();
    let (channel, message) = channel.receive::<Message>().unwrap();
    assert_eq!(message, Message::Commit);
    let _channel = channel.send(&Message::Status(37)).unwrap();
    client.join().unwrap();
}

#[test]
fn wrong_key_version_oversize_and_unknown_messages_close_the_connection() {
    for case in ["key", "version", "size", "unknown", "eof"] {
        let (_root, marker) = marker();
        let mut endpoint = Endpoint::bind(&marker).unwrap();
        let other = marker.clone();
        let client = std::thread::spawn(move || {
            let mut stream = connect(&other, deadline()).unwrap();
            let mut hello = other.key.to_vec();
            hello.push(if case == "version" { 9 } else { VERSION });
            if case == "key" {
                hello[0] ^= 1;
            }
            stream.write_all(&hello).unwrap();
            if case == "size" {
                stream
                    .write_all(&((LIMIT + 1) as u32).to_le_bytes())
                    .unwrap();
            }
            if case == "unknown" {
                let bytes = b"\"Unknown\"";
                stream
                    .write_all(&(bytes.len() as u32).to_le_bytes())
                    .unwrap();
                stream.write_all(bytes).unwrap();
            }
        });
        let stream = endpoint.accept(deadline()).unwrap();
        let result = Channel::authenticate(stream, &marker, true, deadline());
        if matches!(case, "key" | "version") {
            assert_eq!(
                result.err().map(|error| error.kind()),
                Some(io::ErrorKind::PermissionDenied)
            );
        } else {
            let error = result.unwrap().receive::<Message>().err().unwrap();
            let expected = if case == "eof" {
                io::ErrorKind::UnexpectedEof
            } else {
                io::ErrorKind::InvalidData
            };
            assert_eq!(error.kind(), expected, "{case}");
        }
        client.join().unwrap();
    }
}

#[test]
fn first_instance_is_exclusive_and_endpoint_is_reclaimed() {
    let (_root, marker) = marker();
    let endpoint = Endpoint::bind(&marker).unwrap();
    assert!(
        Endpoint::bind(&marker).is_err(),
        "reused a preexisting endpoint"
    );
    drop(endpoint);
    assert!(!marker.directory.exists());
    assert!(Endpoint::bind(&marker).is_ok());
}

#[test]
fn quiet_peer_times_out_and_a_second_connection_is_refused() {
    let (_root, marker) = marker();
    let mut endpoint = Endpoint::bind(&marker).unwrap();
    let other = marker.clone();
    let client = std::thread::spawn(move || {
        let _stream = connect(&other, deadline()).unwrap();
        std::thread::sleep(Duration::from_millis(750));
    });
    let stream = endpoint.accept(deadline()).unwrap();
    {
        use std::os::fd::AsRawFd;
        assert_ne!(
            unsafe { libc::fcntl(stream.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
            0
        );
    }
    let short = Instant::now() + Duration::from_millis(20);
    assert!(connect(&marker, short).is_err());
    let result = Channel::authenticate(
        stream,
        &marker,
        true,
        Instant::now() + Duration::from_millis(20),
    );
    assert_eq!(result.err().unwrap().kind(), io::ErrorKind::TimedOut);
    client.join().unwrap();
}

#[test]
fn unsafe_parents_are_refused() {
    use std::{
        fs,
        os::unix::fs::{PermissionsExt, symlink},
    };
    let (_root, marker) = marker();
    let parent = marker.directory.parent().unwrap();
    symlink(parent, parent.join("link")).unwrap();
    let redirected = Marker::new(parent.join("link/fresh")).unwrap();
    assert!(Endpoint::bind(&redirected).is_err());
    fs::create_dir(parent.join("safe")).unwrap();
    let nested = Marker::new(parent.join("safe/fresh")).unwrap();
    fs::set_permissions(parent, fs::Permissions::from_mode(0o777)).unwrap();
    assert!(Endpoint::bind(&marker).is_err());
    assert!(Endpoint::bind(&nested).is_err());
    fs::set_permissions(parent, fs::Permissions::from_mode(0o700)).unwrap();
}

#[test]
fn full_backlog_cannot_block_connect_past_its_deadline() {
    use std::{
        os::{fd::AsRawFd, unix::fs::DirBuilderExt},
        sync::mpsc,
    };
    let (_root, marker) = marker();
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&marker.directory)
        .unwrap();
    let listener =
        std::os::unix::net::UnixListener::bind(marker.directory.join("control")).unwrap();
    // SAFETY: live listener, lowering the backlog exposes exhaustion with one queued client.
    assert_eq!(unsafe { libc::listen(listener.as_raw_fd(), 0) }, 0);
    let _queued = connect(&marker, deadline()).unwrap();
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        let result = connect(&marker, Instant::now() + Duration::from_millis(20)).map(drop);
        let _ = send.send(result);
    });
    let result = receive.recv_timeout(Duration::from_secs(1));
    assert!(
        matches!(result, Ok(Err(_))),
        "connect blocked behind the full backlog: {result:?}"
    );
}

#[test]
fn marker_rejects_unknown_versions() {
    let (_root, marker) = marker();
    let mut invalid = serde_json::to_value(&marker).unwrap();
    invalid["version"] = 17.into();
    assert!(serde_json::from_value::<Marker>(invalid).is_err());
}
