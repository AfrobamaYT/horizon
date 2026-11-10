//! Explicit account-free Play for persisted unsigned checkout lineage.
use crate::{developer_install, i18n::tr};
use std::process::Command;

pub const ENV: &str = "HORIZON_LAUNCHER_DEVELOPER_OFFLINE";
pub const FLAG: &str = "--developer-single-player";
pub const NOTICE: &str = "Offline developer session: single-player only; no account features.";

pub fn enabled(tag: &str) -> bool {
    if std::env::var_os(ENV).as_deref() != Some(std::ffi::OsStr::new("1")) {
        return false;
    }
    let base = crate::native_mods::about(tag)
        .map(|a| a.base)
        .or_else(|| crate::build_import::about(tag).map(|a| a.base))
        .unwrap_or_else(|| tag.to_owned());
    eligible(&base, &crate::backend::version_dir(&base))
}

fn eligible(base: &str, directory: &std::path::Path) -> bool {
    developer_install::is_developer(base)
        && directory.join(".installed").is_file()
        && developer_install::revision_in(directory).is_ok()
}

pub const REBUILD: &str = "This game cannot enforce offline developer single-player. Rebuild and reinstall the developer game and its native mods.";
pub fn require_capability(tag: &str) -> Result<(), String> {
    let output = Command::new(crate::backend::client_program(tag))
        .arg("--launch-api")
        .output()
        .map_err(|_| tr(REBUILD).to_string())?;
    let reply: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|_| tr(REBUILD).to_string())?;
    if output.status.success()
        && reply["options"].as_array().is_some_and(|options| {
            options
                .iter()
                .any(|o| o["name"] == FLAG && o["value"].is_null())
        })
    {
        Ok(())
    } else {
        Err(tr(REBUILD).into())
    }
}

pub fn mark(cmd: &mut Command) {
    cmd.arg(FLAG)
        .env_remove("HORIZON_SESSION_FILE")
        .env_remove("HORIZON_ACCOUNT_SESSIONS")
        .env_remove("HORIZON_LAUNCHER_EXE");
}

/// Validate after instance arguments and wrappers have been applied, before acquiring real leases.
pub fn validate(cmd: &Command) -> Result<(), String> {
    let offline = cmd.get_args().any(|a| a == FLAG);
    if offline
        && cmd.get_args().any(|a| {
            let a = a.to_string_lossy();
            [
                "--server",
                "--quickPlayMultiplayer",
                "--session-file",
                "--session-stdin",
                "--dedicated",
                "--transfers",
            ]
            .iter()
            .any(|flag| a == *flag || a.starts_with(&format!("{flag}=")))
        })
    {
        return Err(tr(NOTICE).into());
    }
    Ok(())
}

pub fn selected() -> bool {
    let instance = crate::instances::selected();
    instance.java.is_none()
        && instance
            .resolve_version(None)
            .is_ok_and(|tag| enabled(&tag))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persisted_provenance_not_just_a_prefix() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path();
        assert!(!eligible("developer-local", dir));
        std::fs::write(dir.join(".installed"), "").unwrap();
        for record in ["{}", "not json", r#"{"revision":"wrong"}"#] {
            std::fs::write(dir.join("developer.json"), record).unwrap();
            assert!(!eligible("developer-local", dir));
        }
        std::fs::write(
            dir.join("developer.json"),
            serde_json::json!({"revision":"a".repeat(40)}).to_string(),
        )
        .unwrap();
        assert!(eligible("developer-local", dir));
        for base in ["v2026.11.10", "nightly-abcdef", "native-signed"] {
            assert!(!eligible(base, dir));
        }
    }
    #[test]
    fn final_network_and_session_intents_are_refused() {
        for flag in [
            "--server",
            "--quickPlayMultiplayer",
            "--session-file",
            "--session-stdin",
            "--dedicated",
            "--transfers",
            "--server=localhost",
        ] {
            let mut command = Command::new("game");
            mark(&mut command);
            command.arg(flag);
            assert!(validate(&command).is_err(), "{flag}");
        }
        let mut command = Command::new("game");
        mark(&mut command);
        command.args(["--world", "DeveloperProof", "--creative"]);
        assert!(validate(&command).is_ok());
        let mut normal = Command::new("game");
        normal.args(["--server", "localhost"]);
        assert!(validate(&normal).is_ok());
    }
}
