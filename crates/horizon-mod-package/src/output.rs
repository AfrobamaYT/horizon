//! Game-owned, bounded raw stdout/stderr capture. Supervisors may tail files, never own readers.
use cap_fs_ext::{FollowSymlinks, MetadataExt, OpenOptionsFollowExt};
use cap_std::fs::{Dir, File, OpenOptions};
use std::io::{self, IsTerminal, Read, Seek, SeekFrom, Write};
use std::sync::mpsc::{self, Receiver};

mod stdio;
const LIMIT: u64 = 16 * 1024 * 1024;

struct Rotating {
    directory: Dir,
    name: &'static str,
    previous: String,
    file: File,
    written: u64,
}
impl Rotating {
    fn open(directory: Dir, name: &'static str) -> io::Result<Self> {
        let previous = format!("{name}.1");
        match directory.symlink_metadata(&previous) {
            Ok(meta) if !meta.is_file() || meta.len() > LIMIT => {
                return Err(io::ErrorKind::InvalidData.into());
            }
            Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
            _ => {}
        }
        let file = Self::append(&directory, name)?;
        let written = file.metadata()?.len();
        if written > LIMIT {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok(Self {
            directory,
            name,
            previous,
            file,
            written,
        })
    }
    fn append(directory: &Dir, name: &str) -> io::Result<File> {
        let mut options = OpenOptions::new();
        options.append(true).create(true).follow(FollowSymlinks::No);
        let file = directory.open_with(name, &options)?;
        if !file.metadata()?.is_file() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        Ok(file)
    }
}
impl Write for Rotating {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.written == LIMIT {
            match self.directory.remove_file(&self.previous) {
                Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
                _ => {}
            }
            self.directory
                .rename(self.name, &self.directory, &self.previous)?;
            self.file = Self::append(&self.directory, self.name)?;
            self.written = 0;
        }
        let count = bytes.len().min((LIMIT - self.written) as usize);
        let written = self.file.write(&bytes[..count])?;
        self.written += written as u64;
        Ok(written)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

struct Stream {
    original: Option<stdio::Restore>,
    done: Receiver<()>,
}
impl Drop for Stream {
    fn drop(&mut self) {
        drop(self.original.take());
        // A descendant can retain stdout: normal exit must not wait for it forever.
        let _ = self.done.recv_timeout(std::time::Duration::from_secs(1));
    }
}

/// Keep alive for the game run. Dropping restores streams and drains pending bytes with a deadline.
/// Install once at entry before worker/mod threads start; the caller owns the launch directory.
pub struct Capture(Vec<Stream>);
impl Capture {
    /// Leave attached terminal streams inherited; redirect other streams to in-game readers.
    pub fn install(directory: &Dir) -> io::Result<Self> {
        let mut capture = Self(Vec::new());
        for (fd, name, tty) in [
            (1, "stdout.log", io::stdout().is_terminal()),
            (2, "stderr.log", io::stderr().is_terminal()),
        ] {
            if tty {
                continue;
            }
            let mut sink = Rotating::open(directory.try_clone()?, name)?;
            let (mut reader, writer) = os_pipe::pipe()?;
            let (done, completed) = mpsc::sync_channel(1);
            std::thread::Builder::new()
                .name(name.into())
                .spawn(move || {
                    let mut buffer = [0u8; 16 * 1024];
                    loop {
                        match reader.read(&mut buffer) {
                            Ok(0) => break,
                            Ok(n) => {
                                let _ = sink.write_all(&buffer[..n]);
                            }
                            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                            Err(_) => break,
                        }
                    }
                    let _ = done.send(());
                })?;
            let original = stdio::redirect(fd, &writer)?;
            capture.0.push(Stream {
                original: Some(original),
                done: completed,
            });
        }
        Ok(capture)
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        let _ = io::stdout().flush();
        let _ = io::stderr().flush();
        // SAFETY: flush C streams before restoring their descriptors; readers are still alive.
        unsafe {
            libc::fflush(std::ptr::null_mut());
        }
    }
}

/// Optional bounded reader under a held launch directory; the writer continues when dropped.
pub struct Tail {
    directory: Dir,
    name: String,
    previous: String,
    position: Option<((u64, u64), u64)>,
}
impl Tail {
    pub fn new(directory: Dir, name: &str) -> Self {
        Self {
            directory,
            name: name.into(),
            previous: format!("{name}.1"),
            position: None,
        }
    }
    /// Drain a retained previous segment before switching. Never pin an unlinked segment
    /// between calls: a stalled UI cannot hold deleted history beyond the writer's cap.
    pub fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let Some((file, key)) = self.open_segment(&self.name)? else {
            return Ok(0);
        };
        if let Some((old, _)) = self.position.filter(|(old, _)| *old != key) {
            if let Some((previous, id)) = self.open_segment(&self.previous)? {
                if id == old {
                    let count = self.read_at(previous, old, buffer)?;
                    if count != 0 {
                        return Ok(count);
                    }
                }
            }
        }
        self.read_at(file, key, buffer)
    }
    fn open_segment(&self, name: &str) -> io::Result<Option<(File, (u64, u64))>> {
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        let file = match self.directory.open_with(name, &options) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        Ok(Some((file, (metadata.dev(), metadata.ino()))))
    }
    fn read_at(&mut self, mut file: File, key: (u64, u64), buffer: &mut [u8]) -> io::Result<usize> {
        let offset = self
            .position
            .filter(|(id, _)| *id == key)
            .map_or(0, |(_, offset)| offset);
        file.seek(SeekFrom::Start(offset))?;
        let count = file.read(buffer)?;
        self.position = Some((key, offset + count as u64));
        Ok(count)
    }
}
