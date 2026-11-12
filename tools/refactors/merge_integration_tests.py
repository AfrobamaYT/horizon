"""Merge a crate's integration test files into one test binary, `tests/integration.rs`.

Each `tests/<name>.rs` becomes the module `tests/integration/<name>.rs`, so Cargo links one test
binary instead of one per file; every test keeps its function name under its module
(`<name>::<test>`). Files named with --keep stay separate binaries: they hold process-global
state another file's tests would observe (an installed hook, a set-once initializer).

    python3 tools/refactors/merge_integration_tests.py crates/horizon-world --keep native_load_hooks
"""
import argparse
import subprocess
from pathlib import Path

GUARD = '''
/// Every file under `tests/integration/` is declared above, so none is silently left unrun.
#[test]
fn every_integration_file_is_declared() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/integration");
    let declared = include_str!("integration.rs");
    for entry in std::fs::read_dir(root).unwrap() {
        let name = entry.unwrap().path().file_stem().unwrap().to_string_lossy().into_owned();
        assert!(declared.contains(&format!("mod {name};")), "tests/integration/{name}.rs is not declared");
    }
}
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("crate", type=Path)
    parser.add_argument("--keep", action="append", default=[], help="a test file (stem) left as its own binary")
    args = parser.parse_args()
    tests = args.crate / "tests"
    target = tests / "integration"
    target.mkdir()
    names = sorted(path.stem for path in tests.glob("*.rs") if path.stem not in args.keep)
    for name in names:
        subprocess.run(["git", "mv", str(tests / f"{name}.rs"), str(target / f"{name}.rs")], check=True)
    header = ("//! The crate's integration tests in one binary (tools/refactors/merge_integration_tests.py):\n"
              "//! one link instead of one per file. A new file under `integration/` needs its `mod` line.\n\n")
    (tests / "integration.rs").write_text(header + "".join(f"mod {name};\n" for name in names) + GUARD)
    subprocess.run(["git", "add", str(tests / "integration.rs")], check=True)


if __name__ == "__main__":
    main()
