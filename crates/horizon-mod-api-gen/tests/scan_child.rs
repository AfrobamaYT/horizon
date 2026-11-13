//! The uses scan's child process, run as `horizon-stage` starts it: a parse that overflows its
//! stack ends the child, which the caller reads as "cannot judge", and the caller lives on.
use mod_surface::child::{CHILD_VAR, Contained, parse_in_child};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

fn program() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_horizon-stage"))
}

/// A chain of qualified paths after a comparison, which the parser reads by recursion.
fn qualified_paths(depth: usize) -> String {
    format!(
        "fn f() {{ let x = 1 < {}T{}; }}",
        "<".repeat(depth),
        " as A>::B".repeat(depth)
    )
}

#[test]
#[ignore = "daily"]
fn a_parse_that_overflows_ends_the_child_and_names_the_file() {
    let fine = "fn f() { let x = (1, [2]); }";
    let deep = qualified_paths(50_000);
    let wait = Duration::from_secs(60);
    assert_eq!(
        parse_in_child(program(), &[fine, &deep], 256 << 10, wait),
        Err(Contained::Died(Some(1)))
    );
    assert_eq!(
        parse_in_child(program(), &[fine, fine], 256 << 10, wait),
        Ok(())
    );
    // A child given no time is stopped, not waited for.
    assert!(matches!(
        parse_in_child(program(), &[&deep], 64 << 20, Duration::ZERO),
        Err(Contained::TimedOut(_))
    ));
    // A program that is not there starts no child.
    assert_eq!(
        parse_in_child(
            Path::new("/nonexistent/horizon-stage"),
            &[fine],
            256 << 10,
            wait
        ),
        Err(Contained::NoChild)
    );
}

#[test]
#[ignore = "daily"]
fn the_child_refuses_input_that_is_not_a_scans() {
    let mut child = Command::new(program())
        .env(CHILD_VAR, "1048576")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"uses src\n")
        .unwrap();
    assert_eq!(child.wait().unwrap().code(), Some(3));
}
