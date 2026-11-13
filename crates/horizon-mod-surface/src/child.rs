//! Parsing a mod's source in a child process, so that a parser that runs out of stack ends the
//! child and not the program that asked.
//!
//! The parser reads nesting by recursion. [`crate::syntax::too_deep`] refuses deep source before
//! it is parsed, but a count of tokens can be fooled, and a stack overflow cannot be caught: it
//! aborts the process. So [`crate::uses::uses_contained`] first parses every file once in a child,
//! the program itself started again ([`std::env::current_exe`]) with [`CHILD_VAR`] set, its
//! sources on stdin and never on the command line. A child that dies, by a signal, an overflow or
//! its memory limit, or that outlives [`CHILD_WAIT`], leaves the source "cannot judge". Only once
//! the child parsed every file does the scan parse them in the program itself, on a stack of the
//! same size, where the same parse of the same text takes the same depth.
//!
//! Every program that runs the scan calls [`child_entry`] first in `main`.
use std::io::{Read, Write};
use std::time::{Duration, Instant};

/// Marks a process as the scan's child; its value is the parser's stack size in bytes.
pub const CHILD_VAR: &str = "HORIZON_MOD_SCAN_CHILD";

/// How long a child may parse a mod's source: about ten times the 1.5 s a release build takes
/// on 200 000 lines in 1749 files.
pub const CHILD_WAIT: Duration = Duration::from_secs(15);

/// The address space a child may use, where the system can limit it (Linux and macOS).
#[cfg(unix)]
const CHILD_MEMORY: u64 = 4 << 30;

/// The first bytes of a child's input.
const MAGIC: &[u8] = b"horizon-mod-scan 1\n";

/// Why the child did not parse the sources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Contained {
    /// It stopped, by a signal, an overflow or its memory limit, while parsing this source (its
    /// index among those given), or before it started.
    Died(Option<usize>),
    /// It was still parsing this source when [`CHILD_WAIT`] ran out.
    TimedOut(Option<usize>),
    /// It could not be started: the program cannot run itself here (always on wasm).
    NoChild,
}

/// In the scan's child, parses the sources on stdin and exits; anywhere else, returns at once.
/// A process started with [`CHILD_VAR`] whose stdin is not a scan's input exits with an error,
/// so the entry never runs outside a scan.
pub fn child_entry() {
    let Some(stack) = std::env::var_os(CHILD_VAR) else {
        return;
    };
    let Some(stack) = stack.to_str().and_then(|s| s.parse::<usize>().ok()) else {
        std::process::exit(3)
    };
    limit_memory();
    let Some(texts) = read_input(&mut std::io::stdin().lock()) else {
        std::process::exit(3)
    };
    let mut out = std::io::stdout().lock();
    for (index, text) in texts.iter().enumerate() {
        // Which source is being parsed, so the parent can name the one the child died on.
        let _ = writeln!(out, "{index}");
        let _ = out.flush();
        let parsed = std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(stack)
                .spawn_scoped(scope, || {
                    let _ = crate::syntax::parse(text);
                })
                .map(|thread| thread.join().is_ok())
        });
        if !matches!(parsed, Ok(true)) {
            std::process::exit(4)
        }
    }
    let _ = writeln!(out, "done");
    let _ = out.flush();
    std::process::exit(0)
}

/// Caps the child's address space where the system allows it.
fn limit_memory() {
    #[cfg(unix)]
    {
        let limit = libc::rlimit {
            rlim_cur: CHILD_MEMORY as libc::rlim_t,
            rlim_max: CHILD_MEMORY as libc::rlim_t,
        };
        // SAFETY: `setrlimit` reads the struct it is given and touches nothing else.
        unsafe {
            libc::setrlimit(libc::RLIMIT_AS, &limit);
        }
    }
}

fn read_input(input: &mut impl Read) -> Option<Vec<String>> {
    let mut magic = vec![0; MAGIC.len()];
    input.read_exact(&mut magic).ok()?;
    if magic != MAGIC {
        return None;
    }
    let mut count = [0; 8];
    input.read_exact(&mut count).ok()?;
    let count = u64::from_le_bytes(count);
    let mut texts = Vec::new();
    for _ in 0..count {
        let mut length = [0; 8];
        input.read_exact(&mut length).ok()?;
        let mut bytes = Vec::new();
        input
            .take(u64::from_le_bytes(length))
            .read_to_end(&mut bytes)
            .ok()?;
        texts.push(String::from_utf8(bytes).ok()?);
    }
    Some(texts)
}

fn write_input(texts: &[&str]) -> Vec<u8> {
    let mut input = MAGIC.to_vec();
    input.extend((texts.len() as u64).to_le_bytes());
    for text in texts {
        input.extend((text.len() as u64).to_le_bytes());
        input.extend(text.as_bytes());
    }
    input
}

/// Parses `texts` in a child started from `program`, the parser on a stack of `stack` bytes,
/// waiting at most `wait`.
pub fn parse_in_child(
    program: &std::path::Path,
    texts: &[&str],
    stack: usize,
    wait: Duration,
) -> Result<(), Contained> {
    use std::process::{Command, Stdio};
    let mut child = Command::new(program)
        .env(CHILD_VAR, stack.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| Contained::NoChild)?;
    let input = write_input(texts);
    let mut stdin = child.stdin.take().ok_or(Contained::NoChild)?;
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&input);
    });
    let mut stdout = child.stdout.take().ok_or(Contained::NoChild)?;
    let reader = std::thread::spawn(move || {
        let mut said = String::new();
        let _ = stdout.read_to_string(&mut said);
        said
    });
    let deadline = Instant::now() + wait;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let _ = writer.join();
    let said = reader.join().unwrap_or_default();
    let last = said.lines().next_back();
    let at = said
        .lines()
        .rev()
        .find_map(|line| line.parse::<usize>().ok());
    match status {
        Some(status) if status.success() && last == Some("done") => Ok(()),
        Some(_) => Err(Contained::Died(at)),
        None => Err(Contained::TimedOut(at)),
    }
}
