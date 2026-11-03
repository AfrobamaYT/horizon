//! Exercise both startup tokenization and Options through the public help contract.
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("launch-switches-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn run(&self, args: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_horizon-client"));
        command.current_dir(&self.0).args(args);
        for (name, folder) in [
            ("HORIZON_HOME", "home"),
            ("HORIZON_LAUNCHER_HOME", "launcher"),
            ("XDG_DATA_HOME", "data"),
            ("XDG_CONFIG_HOME", "config"),
            ("HORIZON_LAUNCHER_TRASH", "trash"),
            ("HORIZON_LAUNCHER_CLIPBOARD", "clipboard"),
            ("HORIZON_LAUNCHER_DESKTOP", "desktop"),
        ] {
            command.env(name, self.0.join(folder));
        }
        command.env("HORIZON_LAUNCHER_NO_SELF_UPDATE", "1").env("HORIZON_NO_AUDIO", "1");
        command.output().unwrap()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn documented_switches_do_not_consume_the_following_option() {
    let scratch = Scratch::new();
    let output = scratch.run(&["--help"]);
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    let switches: Vec<_> = help.lines().filter_map(|line| {
        let (usage, _) = line.trim_start().split_once("  ")?;
        (usage.starts_with("--") && !usage.contains(char::is_whitespace)).then_some(usage)
    }).collect();
    assert!(!switches.is_empty(), "help must describe startup switches");
    // Separate processes also isolate --pregen's global command toggle.
    for switch in switches {
        let output = scratch.run(&[switch, "--help"]);
        assert!(output.status.success(), "{switch}: {}", String::from_utf8_lossy(&output.stderr));
        if switch == "--launch-api" {
            let reply: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert!(reply["launch_api"].is_string());
        } else {
            assert_eq!(String::from_utf8(output.stdout).unwrap(), help, "{switch}");
        }
    }
}
