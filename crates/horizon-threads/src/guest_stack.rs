//! Stack policy shared by guest callers and executable link settings.

#[path = "guest_stack_constants.rs"]
mod constants;
pub use constants::{GUEST_STACK_BYTES, HOST_HEADROOM};

/// Spawn guarantees the reservation even on platforms without a stack-size query.
pub fn builder() -> std::thread::Builder {
    std::thread::Builder::new().stack_size(GUEST_STACK_BYTES + HOST_HEADROOM)
}

/// Must run before compiler-child handling or any game initialization.
pub fn prepare_main() {
    if let Err(error) = prepare() {
        eprintln!("GuestStackUnavailable: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "linux"))]
fn prepare() -> std::io::Result<()> {
    // Executable link settings reserve the same stack on Windows and macOS.
    inspect_current().map_err(std::io::Error::other)
}

#[cfg(target_os = "linux")]
fn prepare() -> std::io::Result<()> {
    use std::os::unix::process::CommandExt;
    let required = (GUEST_STACK_BYTES + HOST_HEADROOM) as libc::rlim_t;
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: limit is writable and getrlimit initializes it on success.
    if unsafe { libc::getrlimit(libc::RLIMIT_STACK, &mut limit) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    if limit.rlim_cur >= required && inspect_current().is_ok() {
        return Ok(());
    }
    if limit.rlim_max < required || std::env::var_os("HORIZON_STACK_REEXEC").is_some() {
        return Err(std::io::Error::other(format!(
            "raise the stack hard limit (ulimit -Hs) to at least {} KiB",
            required.saturating_add(HOST_HEADROOM as libc::rlim_t) / 1024
        )));
    }
    // pthread_getattr_np excludes the initial argument/environment pages from
    // its reported stack extent. Reserve additional host headroom for those
    // pages when the hard limit permits it; guest entry still verifies the extent.
    limit.rlim_cur = required
        .saturating_add(HOST_HEADROOM as libc::rlim_t)
        .min(limit.rlim_max);
    // SAFETY: limit is initialized; this changes only this process's resource limit.
    if unsafe { libc::setrlimit(libc::RLIMIT_STACK, &limit) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    // Linux chooses its mmap base at exec using RLIMIT_STACK. Re-exec keeps the
    // enlarged stack's address range clear before libraries and workers start.
    Err(std::process::Command::new(std::env::current_exe()?)
        .arg0(std::env::args_os().next().unwrap_or_default())
        .args(std::env::args_os().skip(1))
        .env("HORIZON_STACK_REEXEC", "1")
        .exec())
}

/// Refuse guest entry when the calling thread's stack reservation is too small.
/// Cache the platform query once per thread; the hot path reads only thread-local state.
pub fn check_current() -> Result<(), &'static str> {
    thread_local! {
        static CHECK: std::cell::OnceCell<Result<(), &'static str>> = const { std::cell::OnceCell::new() };
    }
    CHECK.with(|check| *check.get_or_init(inspect_current))
}

fn inspect_current() -> Result<(), &'static str> {
    #[cfg(target_os = "linux")]
    {
        let mut attr = std::mem::MaybeUninit::<libc::pthread_attr_t>::uninit();
        // SAFETY: pthread_self is live and attr is writable. Destroy only on success.
        unsafe {
            if libc::pthread_getattr_np(libc::pthread_self(), attr.as_mut_ptr()) != 0 {
                return Err("GuestStackUnavailable: cannot inspect the calling thread stack");
            }
            let mut attr = attr.assume_init();
            let mut size = 0;
            let result = libc::pthread_attr_getstacksize(&attr, &mut size);
            libc::pthread_attr_destroy(&mut attr);
            if result != 0 || size < GUEST_STACK_BYTES + HOST_HEADROOM {
                return Err("GuestStackUnavailable: calling thread needs 136 MiB of stack");
            }
        }
    }
    #[cfg(target_vendor = "apple")]
    {
        // SAFETY: pthread_self names the live calling thread.
        let size = unsafe { libc::pthread_get_stacksize_np(libc::pthread_self()) };
        if size < GUEST_STACK_BYTES + HOST_HEADROOM {
            return Err("GuestStackUnavailable: calling thread needs 136 MiB of stack");
        }
    }
    #[cfg(windows)]
    {
        let mut low = 0;
        let mut high = 0;
        // SAFETY: both pointers refer to writable stack bounds.
        unsafe {
            windows_sys::Win32::System::Threading::GetCurrentThreadStackLimits(&mut low, &mut high)
        };
        if high - low < GUEST_STACK_BYTES + HOST_HEADROOM {
            return Err("GuestStackUnavailable: calling thread needs 136 MiB of stack");
        }
    }
    #[cfg(not(any(target_os = "linux", target_vendor = "apple", windows)))]
    return Err("GuestStackUnavailable: stack inspection is unsupported on this platform");
    #[cfg(any(target_os = "linux", target_vendor = "apple", windows))]
    Ok(())
}

#[cfg(test)]
#[test]
fn audited_guest_workers_use_the_stack_policy() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for file in [
        "crates/horizon-threads/src/jobs.rs",
        "crates/horizon-client/src/vanillagen.rs",
        "crates/horizon-client/src/vanillagen/lod.rs",
        "crates/horizon-client/src/game/runtime/resource_load.rs",
        "crates/horizon-client/src/game/interface/startup_menu.rs",
        "crates/horizon-client/src/game/world/world_loading.rs",
        "crates/horizon-client/src/game/world/world_entry.rs",
    ] {
        let source = std::fs::read_to_string(root.join(file)).unwrap();
        assert!(
            !source.contains("thread::Builder::new()"),
            "{file}: route guest workers through guest_stack::builder"
        );
    }
    let server =
        std::fs::read_to_string(root.join("crates/horizon-client/src/game/world/integrated.rs"))
            .unwrap();
    assert_eq!(
        server.matches(".stack_size(server_thread_stack())").count(),
        2
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn small_worker_refuses_guest_entry() {
        std::thread::Builder::new()
            .stack_size(1024 * 1024)
            .spawn(|| {
                assert!(
                    super::check_current()
                        .unwrap_err()
                        .contains("GuestStackUnavailable")
                )
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn guest_worker_reserves_the_required_stack() {
        super::builder()
            .spawn(|| super::check_current().unwrap())
            .unwrap()
            .join()
            .unwrap();
    }
}
