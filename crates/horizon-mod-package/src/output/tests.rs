use super::*;
use std::{sync::Mutex, time::Duration};
struct Stalled {
    gate: Option<Receiver<()>>,
    bytes: Arc<Mutex<Vec<u8>>>,
}
impl Write for Stalled {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if let Some(gate) = self.gate.take() {
            gate.recv().map_err(|_| io::ErrorKind::BrokenPipe)?;
        }
        self.bytes.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[test]
fn stalled_disk_drops_newest_bytes_without_blocking_the_game_pipe() {
    let (reader, mut writer) = os_pipe::pipe().unwrap();
    let (release, gate) = mpsc::channel();
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let done = drain(
        reader,
        Stalled {
            gate: Some(gate),
            bytes: bytes.clone(),
        },
        "stall-test",
    )
    .unwrap();
    let (produced, progress) = mpsc::channel();
    let producer = std::thread::spawn(move || {
        for _ in 0..256 {
            writer.write_all(&[b'A'; 16 * 1024]).unwrap();
        }
        produced.send(()).unwrap();
    });
    let prompt = progress.recv_timeout(Duration::from_secs(2)).is_ok();
    release.send(()).unwrap();
    producer.join().unwrap();
    done.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(
        prompt,
        "game-side write blocked behind the stalled filesystem"
    );
    let bytes = bytes.lock().unwrap();
    let written = bytes.iter().filter(|&&byte| byte == b'A').count();
    let text = String::from_utf8(bytes.clone()).unwrap();
    let dropped: usize = text
        .split("[horizon.output.dropped_bytes=")
        .skip(1)
        .map(|part| part.split(']').next().unwrap().parse::<usize>().unwrap())
        .sum();
    assert!(dropped > 0);
    assert_eq!(written + dropped, 256 * 16 * 1024);
    assert!(
        written <= (8 + 4 + 2) * 16 * 1024,
        "queue plus pipe/working buffers exceeded their byte budget"
    );
}
