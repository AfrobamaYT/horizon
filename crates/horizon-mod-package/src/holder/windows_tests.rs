use super::tests::actor_command;
use std::{
    fs, mem,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
};
use windows_sys::Win32::System::JobObjects::*;
#[test]
fn raw_cmd_syntax_preserves_metacharacters_and_quoted_output_paths() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().canonicalize().unwrap();
    let command = super::Program {
        executable: std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32/cmd.exe")
            .into_os_string(),
        arguments: vec![],
        environment: vec![],
        directory: directory.clone(),
        raw_windows: Some(r#"/D /S /C "echo alpha^&beta>"raw output.txt"""#.into()),
    };
    assert!(command.command().unwrap().status().unwrap().success());
    assert_eq!(
        fs::read(directory.join("raw output.txt")).unwrap(),
        b"alpha&beta\r\n"
    );
}
#[test]
fn real_no_breakaway_kill_on_close_job_retries_and_preserves_game_record() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap();
    let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    assert!(!handle.is_null());
    let job = unsafe { OwnedHandle::from_raw_handle(handle) };
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    assert_ne!(
        unsafe {
            SetInformationJobObject(
                job.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                mem::size_of_val(&limits) as u32,
            )
        },
        0
    );
    let mut child = actor_command()
        .current_dir(&path)
        .env("HORIZON_HOLDER_TEST_MODE", "job")
        .env("HORIZON_HOLDER_TEST_JOB_ROOT", &path)
        .spawn()
        .unwrap();
    // The standalone actor waits for this event before attempting its first holder spawn.
    assert_ne!(
        unsafe { AssignProcessToJobObject(job.as_raw_handle(), child.as_raw_handle()) },
        0
    );
    fs::write(path.join("assigned"), b"assigned").unwrap();
    assert!(child.wait().unwrap().success());
    assert_eq!(
        fs::read(path.join("proof")).unwrap(),
        b"degraded-held-released"
    );
    drop(job);
}
