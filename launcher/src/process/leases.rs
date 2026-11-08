//! Child-owned kernel claims. Not wired into managed launch until the complete safety stack.
use crate::{
    native_cache::{Error, LockMode, Release},
    run_leases::{Claim, Registry},
};
use cap_std::fs::Dir;
use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
};

/// Independent open descriptions: dropping the creator never explicitly unlocks the child.
pub struct ChildLeases {
    claims: Vec<Claim>,
    data: PathBuf,
}
impl ChildLeases {
    /// Acquire the shared gate, then each held tree. Fails immediately on contention.
    pub fn acquire(data: &Path, trees: &[Dir]) -> io::Result<Self> {
        let registry = Registry::with_release(data, Release::Close).map_err(error)?;
        let claims = trees
            .iter()
            .map(|tree| registry.lock(tree, LockMode::Shared).map_err(error))
            .collect::<io::Result<Vec<_>>>()?;
        Ok(Self {
            claims,
            data: data.canonicalize()?,
        })
    }
    fn files(&self) -> Vec<&File> {
        self.claims.iter().flat_map(Claim::files).collect()
    }
    /// Only the chosen fork clears CLOEXEC. Consume the command, so stale pre_exec fds cannot
    /// be reused after these owners drop. No allocation or lock is taken in the child callback.
    #[cfg(unix)]
    pub fn spawn(self, mut command: std::process::Command) -> io::Result<std::process::Child> {
        use std::os::{fd::AsRawFd, unix::process::CommandExt};
        let mut fds: Vec<_> = self.files().iter().map(|file| file.as_raw_fd()).collect();
        fds.sort_unstable();
        fds.dedup();
        command.env("HORIZON_RUNTIME_DATA_ROOT", &self.data).env(
            "HORIZON_RUNTIME_LEASE_FDS",
            fds.iter().map(i32::to_string).collect::<Vec<_>>().join(","),
        );
        // SAFETY: only async-signal-safe fcntl calls run after fork. self owns every fd until
        // spawn returns, and the launcher/holder copies keep their original CLOEXEC flags.
        unsafe {
            command.pre_exec(move || {
                for &fd in &fds {
                    let flags = libc::fcntl(fd, libc::F_GETFD);
                    if flags < 0 || libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) < 0 {
                        return Err(io::Error::last_os_error());
                    }
                }
                Ok(())
            });
        }
        let child = command.spawn();
        drop(self); // Close-only, including the gate: never unlock the transferred set.
        child
    }
}
#[cfg(test)]
mod tests;

fn error(error: Error) -> io::Error {
    match error {
        Error::Busy => io::ErrorKind::WouldBlock.into(),
        Error::Unsafe(_) => io::ErrorKind::InvalidInput.into(),
        Error::Io(error) => error,
    }
}
