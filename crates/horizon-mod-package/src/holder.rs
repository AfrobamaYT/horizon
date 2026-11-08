//! Selected-game holder bootstrap; production IO/signals are wired by the next layer.
use crate::{
    local_control::{Channel, Endpoint, Marker, connect},
    locks::{LockMode, Release},
    runtime_leases::{Claim, Registry, open_directory},
};
use cap_fs_ext::{FollowSymlinks, MetadataExt, OpenOptionsFollowExt};
use cap_std::fs::{Dir, File, OpenOptions};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus},
    time::{Duration, Instant},
};
pub const ENV: &str = "HORIZON_RUNTIME_HOLDER";
pub const ENTRY: &str = "HORIZON_RUNTIME_ENTRY";
pub const CAPABILITY: [u8; 26] = *b"horizon-private-holder-v1;";
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Tree {
    #[serde(
        serialize_with = "crate::local_control::path_out",
        deserialize_with = "crate::local_control::path_in"
    )]
    path: PathBuf,
    identity: [u64; 2],
    complete: Option<[u64; 2]>,
}
/// Retained descriptions prevent inode reuse between GUI snapshot and holder acknowledgment.
struct Snapshot {
    tree: Tree,
    directory: Dir,
    _complete: Option<File>,
}
fn identity(directory: &Dir) -> io::Result<[u64; 2]> {
    let meta = directory.dir_metadata()?;
    Ok([meta.dev(), meta.ino()])
}
fn completion(directory: &Dir) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let file = directory.open_with(".installed", &options)?;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() != 0 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(file)
}
fn token(file: &File) -> io::Result<[u64; 2]> {
    let meta = file.metadata()?;
    Ok([meta.dev(), meta.ino()])
}
impl Snapshot {
    fn capture(path: &Path, complete: bool) -> io::Result<Self> {
        let directory = open_directory(path).map_err(lease_error)?;
        let file = complete.then(|| completion(&directory)).transpose()?;
        let tree = Tree {
            path: path.to_owned(),
            identity: identity(&directory)?,
            complete: file.as_ref().map(token).transpose()?,
        };
        Ok(Self {
            tree,
            directory,
            _complete: file,
        })
    }
}
impl Tree {
    fn snapshot(&self) -> io::Result<Snapshot> {
        let snapshot = Snapshot::capture(&self.path, self.complete.is_some())?;
        if snapshot.tree.identity != self.identity || snapshot.tree.complete != self.complete {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok(snapshot)
    }
    fn recheck(&self) -> io::Result<Dir> {
        self.snapshot().map(|snapshot| snapshot.directory)
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Program {
    pub executable: OsString,
    pub arguments: Vec<OsString>,
    pub environment: Vec<(OsString, OsString)>,
    #[serde(
        serialize_with = "crate::local_control::path_out",
        deserialize_with = "crate::local_control::path_in"
    )]
    pub directory: PathBuf,
    /// Lossless cmd.exe syntax from its owning wrapper planner, never recovered from args.
    pub raw_windows: Option<OsString>,
}
impl Program {
    fn command(&self) -> io::Result<Command> {
        let mut command = Command::new(&self.executable);
        if let Some(raw) = &self.raw_windows {
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.raw_arg(raw);
            }
            #[cfg(not(windows))]
            {
                let _ = raw;
                return Err(io::ErrorKind::InvalidInput.into());
            }
        } else {
            command.args(&self.arguments);
        }
        command
            .current_dir(&self.directory)
            .env_clear()
            .envs(self.environment.iter().cloned());
        command.env_remove(ENV).env_remove(ENTRY);
        Ok(command)
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    #[serde(deserialize_with = "crate::local_control::version")]
    version: u8,
    #[serde(
        serialize_with = "crate::local_control::path_out",
        deserialize_with = "crate::local_control::path_in"
    )]
    data: PathBuf,
    trees: Vec<Tree>,
}
/// Independently opened tree and completion objects prevent identity reuse for this process.
pub struct Acquired {
    _claims: Vec<Claim>,
    _snapshots: Vec<Snapshot>,
}
impl Entry {
    fn validate(&self, maximum: usize) -> io::Result<()> {
        if self.trees.is_empty()
            || self.trees.len() > maximum
            || !self.trees.iter().any(|tree| tree.complete.is_some())
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        Ok(())
    }
    /// Strictly noncreating, gate before trees; used by holder and independent game entry.
    pub fn acquire(&self) -> io::Result<Acquired> {
        self.validate(16)?;
        let registry = Registry::existing(&self.data).map_err(lease_error)?;
        let snapshots = self
            .trees
            .iter()
            .map(Tree::snapshot)
            .collect::<io::Result<Vec<_>>>()?;
        let claims = snapshots
            .iter()
            .map(|snapshot| {
                registry
                    .lock(&snapshot.directory, LockMode::Shared)
                    .map_err(lease_error)
            })
            .collect::<io::Result<_>>()?;
        self.check_image(&std::env::current_exe()?)?;
        Ok(Acquired {
            _claims: claims,
            _snapshots: snapshots,
        })
    }
    fn check_image(&self, executable: &Path) -> io::Result<()> {
        let directory = open_directory(executable.parent().ok_or(io::ErrorKind::InvalidInput)?)
            .map_err(lease_error)?;
        let expected = identity(&directory)?;
        if !self.trees.iter().any(|tree| tree.identity == expected) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok(())
    }
    fn recheck(&self) -> io::Result<()> {
        self.trees
            .iter()
            .try_for_each(|tree| tree.recheck().map(drop))
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    entry: Entry,
    program: Program,
}
pub struct Prepared {
    request: Request,
    _snapshots: Vec<Snapshot>,
    _claims: Vec<Claim>,
}
impl Prepared {
    pub fn new(data: PathBuf, trees: Vec<(PathBuf, bool)>, program: Program) -> io::Result<Self> {
        if !data.is_absolute()
            || !program.directory.is_absolute()
            || trees.is_empty()
            || trees.len() > 15
            || !trees.iter().any(|(_, complete)| *complete)
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        // The namespace gate must precede every selected-tree open, not just its eventual lock.
        let registry = Registry::with_release(&data, Release::Close).map_err(lease_error)?;
        let mut snapshots = trees
            .into_iter()
            .map(|(path, complete)| Snapshot::capture(&path, complete))
            .collect::<io::Result<Vec<_>>>()?;
        let mut entry = Entry {
            version: crate::local_control::VERSION,
            data,
            trees: snapshots
                .iter()
                .map(|snapshot| snapshot.tree.clone())
                .collect(),
        };
        let record = record(&entry.data)?;
        entry.trees.push(record.tree.clone());
        snapshots.push(record);
        let claims = snapshots
            .iter()
            .map(|snapshot| {
                registry
                    .lock(&snapshot.directory, LockMode::Shared)
                    .map_err(lease_error)
            })
            .collect::<io::Result<_>>()?;
        entry.recheck()?;
        use std::io::Write;
        let directory = &snapshots.last().unwrap().directory;
        let mut options = OpenOptions::new();
        options
            .write(true)
            .create_new(true)
            .follow(FollowSymlinks::No);
        directory
            .open_with("launch.json", &options)?
            .write_all(&serde_json::to_vec(&entry).map_err(|_| io::ErrorKind::InvalidData)?)?;
        Ok(Self {
            request: Request { entry, program },
            _snapshots: snapshots,
            _claims: claims,
        })
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum Reply {
    Acquired,
    Commit,
}
/// Capability bytes probe format support, never executable authenticity.
pub fn capable(path: &Path) -> io::Result<bool> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = [0; 8192];
    let mut retained = 0;
    loop {
        let read = file.read(&mut buffer[retained..])?;
        if read == 0 {
            return Ok(false);
        }
        let length = retained + read;
        if buffer[..length]
            .windows(CAPABILITY.len())
            .any(|bytes| bytes == CAPABILITY)
        {
            return Ok(true);
        }
        retained = length.min(CAPABILITY.len() - 1);
        buffer.copy_within(length - retained..length, 0);
    }
}
/// Legacy images return without spawning or receiving bootstrap secrets.
pub enum Started {
    Legacy(Command),
    Holder { child: Child, degraded: bool },
}
pub fn launch(
    mut holder: Command,
    marker: &Marker,
    prepared: &Prepared,
    deadline: Instant,
) -> io::Result<Started> {
    if !capable(Path::new(holder.get_program()))? {
        return Ok(Started::Legacy(prepared.request.program.command()?));
    }
    prepared
        .request
        .entry
        .check_image(Path::new(holder.get_program()))?;
    let mut endpoint = Endpoint::bind(marker)?;
    holder.env(
        ENV,
        serde_json::to_string(marker).map_err(|_| io::ErrorKind::InvalidData)?,
    );
    let (mut child, degraded) = spawn(&mut holder)?;
    let request = &prepared.request;
    let handshake = (|| {
        let stream = endpoint.accept(deadline)?;
        let channel = Channel::authenticate(stream, marker, true, deadline)?;
        let channel = channel.send(request)?;
        let (channel, reply) = channel.receive::<Reply>()?;
        if !matches!(reply, Reply::Acquired) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        request.entry.recheck()?;
        channel.send(&Reply::Commit).map(drop)
    })();
    if let Err(error) = handshake {
        let _ = child.kill();
        child.wait()?;
        return Err(error);
    }
    Ok(Started::Holder { child, degraded })
}
fn spawn(command: &mut Command) -> io::Result<(Child, bool)> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use windows_sys::Win32::{
            Foundation::ERROR_ACCESS_DENIED, System::Threading::CREATE_BREAKAWAY_FROM_JOB,
        };
        command.creation_flags(CREATE_BREAKAWAY_FROM_JOB);
        match command.spawn() {
            Ok(child) => return Ok((child, false)),
            Err(error) if error.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32) && in_job()? => {
                command.creation_flags(0);
                return command.spawn().map(|child| (child, true));
            }
            Err(error) => return Err(error),
        }
    }
    #[cfg(not(windows))]
    command.spawn().map(|child| (child, false))
}
#[cfg(windows)]
fn in_job() -> io::Result<bool> {
    use windows_sys::Win32::System::{JobObjects::IsProcessInJob, Threading::GetCurrentProcess};
    let mut assigned = 0;
    if unsafe { IsProcessInJob(GetCurrentProcess(), std::ptr::null_mut(), &mut assigned) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(assigned != 0)
}
fn run(encoded: OsString, claims: &mut Option<Acquired>) -> io::Result<ExitStatus> {
    let marker: Marker = serde_json::from_str(encoded.to_str().ok_or(io::ErrorKind::InvalidData)?)
        .map_err(|_| io::ErrorKind::InvalidData)?;
    // SAFETY: earliest-main private mode precedes threads and never resumes normal startup.
    unsafe {
        std::env::remove_var(ENV);
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    let stream = connect(&marker, deadline)?;
    let channel = Channel::authenticate(stream, &marker, false, deadline)?;
    let (channel, request) = channel.receive::<Request>()?;
    *claims = Some(request.entry.acquire()?);
    request.entry.recheck()?;
    let channel = channel.send(&Reply::Acquired)?;
    let (channel, reply) = channel.receive::<Reply>()?;
    if !matches!(reply, Reply::Commit) {
        return Err(io::ErrorKind::InvalidData.into());
    }
    request.entry.recheck()?;
    drop(channel);
    let mut command = request.program.command()?;
    command.env(
        ENTRY,
        serde_json::to_string(&request.entry).map_err(|_| io::ErrorKind::InvalidData)?,
    );
    command.status()
}
/// Persistent per-launch kernel-owned record; cleanup treats held records as live, never PIDs.
fn record(data: &Path) -> io::Result<Snapshot> {
    use cap_fs_ext::DirExt;
    use sha2::Digest;
    let directory = open_directory(data).map_err(lease_error)?;
    match directory.create_dir("run-records") {
        Err(error) if error.kind() != io::ErrorKind::AlreadyExists => return Err(error),
        _ => {}
    }
    let records = directory.open_dir_nofollow("run-records")?;
    let mut nonce = [0; 16];
    getrandom::fill(&mut nonce).map_err(|_| io::ErrorKind::Other)?;
    let name = format!("{:x}", sha2::Sha256::digest(nonce));
    records.create_dir(&name)?;
    Snapshot::capture(&data.join("run-records").join(name), false)
}
/// Does nothing for ordinary startup. Private startup can never reach the normal game path.
pub fn enter() {
    let Some(encoded) = std::env::var_os(ENV) else {
        return;
    };
    let mut claims = None;
    let result = run(encoded, &mut claims);
    // These claims are deliberately still owned when ExitProcess/exit closes the process.
    std::process::exit(result.map_or(78, |status| status.code().unwrap_or(1)));
}
fn lease_error(error: crate::runtime_leases::Error) -> io::Error {
    match error {
        crate::runtime_leases::Error::Busy => io::ErrorKind::WouldBlock.into(),
        crate::runtime_leases::Error::Io(error) => error,
        _ => io::ErrorKind::InvalidInput.into(),
    }
}
#[cfg(test)]
mod tests;
#[cfg(all(test, windows))]
mod windows_tests;
