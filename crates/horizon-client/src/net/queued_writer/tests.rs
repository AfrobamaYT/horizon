use super::*;
use horizon_protocol::connection::{Connection, Pipe};
use std::time::Duration;

#[test]
fn memory_payload_is_invisible_until_commit_and_keeps_large_body_exact() {
    let (a, b) = Pipe::pair();
    let (_, writer) = Connection::local(a).split();
    let writer = QueuedWriter::new(writer).unwrap();
    let mut peer = Connection::local(b);
    peer.reader
        .set_read_timeout(Some(Duration::from_millis(20)))
        .unwrap();
    let id = PayloadId("test:raw".into());
    let length = horizon_protocol::frame::MAX_UNCOMPRESSED + 1;
    let mut body = vec![42; length];
    body[0] = 7;
    let pending = writer.prepare_payload(body, &id).unwrap();
    assert!(peer.recv().is_err(), "uncommitted packet reached the pipe");
    assert!(writer.sender.stats().reserved_bytes >= length);
    assert_eq!(writer.sender.stats().queued_items, 0);
    peer.reader
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    pending.publish().unwrap();
    let (packet, bytes) = peer.recv().unwrap();
    assert_eq!(packet, 7);
    assert_eq!(bytes.len(), length - 1);
    assert!(bytes.iter().all(|byte| *byte == 42));
}

#[test]
fn abort_refunds_journal_and_buffer_space_and_shutdown_prevents_publication() {
    let (a, _b) = Pipe::pair();
    let (_, writer) = Connection::local(a).split();
    let writer = QueuedWriter::new(writer).unwrap();
    let id = PayloadId("test:raw".into());
    let pending = writer
        .prepare_payload(Vec::with_capacity(10000), &id)
        .unwrap();
    assert!(writer.sender.stats().reserved_bytes >= 10000 + PUBLICATION_OVERHEAD);
    drop(pending);
    assert_eq!(writer.sender.stats().reserved_bytes, 0);
    let pending = writer.prepare_payload(vec![7], &id).unwrap();
    writer.shutdown();
    assert_eq!(pending.publish(), Err(PayloadSendError::Disconnected));
    assert_eq!(writer.sender.stats().reserved_bytes, 0);
}
