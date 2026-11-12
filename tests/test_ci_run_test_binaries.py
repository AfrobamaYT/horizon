"""The parallel test-binary runner lists Cargo's test executables and reports every failure."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("run_test_binaries", ROOT / "tools/ci/run_test_binaries.py")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


def artifact(executable, test=True, name="horizon-world", kind="lib"):
    return json.dumps({
        "reason": "compiler-artifact", "package_id": f"path+file:///repo/crates/{name}#0.1.0",
        "manifest_path": f"/repo/crates/{name}/Cargo.toml", "profile": {"test": test},
        "target": {"name": name, "kind": [kind], "src_path": f"/repo/crates/{name}/src/lib.rs"},
        "executable": executable,
    })


class RunTestBinariesTests(unittest.TestCase):
    def test_package_identity_reads_both_id_forms(self):
        self.assertEqual(runner.package_identity("horizon-world 0.1.0 (path+file:///repo/crates/horizon-world)"),
                         ("horizon-world", "0.1.0"))
        self.assertEqual(runner.package_identity("path+file:///repo/crates/horizon-world#0.1.0"),
                         ("horizon-world", "0.1.0"))
        self.assertEqual(runner.package_identity("path+file:///repo/docs/modding/examples#horizon-modding-examples@0.0.0"),
                         ("horizon-modding-examples", "0.0.0"))

    def test_only_test_executables_each_once_with_their_package_environment(self):
        messages = "\n".join([
            artifact("/t/a"), artifact("/t/a"), artifact("/t/b", test=False), artifact(None),
            json.dumps({"reason": "build-finished", "success": True}), "not json",
        ])
        binaries = runner.test_binaries(messages)
        self.assertEqual([b["executable"] for b in binaries], ["/t/a"])
        self.assertEqual(binaries[0]["cwd"], "/repo/crates/horizon-world")
        self.assertEqual(binaries[0]["env"]["CARGO_PKG_NAME"], "horizon-world")
        self.assertEqual(binaries[0]["env"]["CARGO_CRATE_NAME"], "horizon_world")

    @unittest.skipUnless(os.name == "posix" and shutil.which("sh"), "requires a POSIX shell")
    def test_every_binary_runs_and_any_failure_fails_the_run(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / "crates/horizon-world"
            package.mkdir(parents=True)
            passing, failing = root / "passing", root / "failing"
            passing.write_text('#!/bin/sh\necho "cwd=$PWD pkg=$CARGO_PKG_NAME"\n')
            failing.write_text("#!/bin/sh\necho broken\nexit 101\n")
            for binary in (passing, failing):
                binary.chmod(0o755)
            listing = [artifact(str(binary)).replace("/repo", str(root)) for binary in (passing, failing)]
            cargo = root / "cargo"
            cargo.write_text(f"#!/bin/sh\ncat <<'EOF'\n" + "\n".join(listing) + "\nEOF\n")
            cargo.chmod(0o755)
            rustc = root / "rustc"
            rustc.write_text(f'#!/bin/sh\ncase "$1" in --print) echo {root}/sysroot ;; *) echo "host: x86_64-unknown-linux-gnu" ;; esac\n')
            rustc.chmod(0o755)
            with contextlib.redirect_stdout(io.StringIO()) as log:
                code = runner.main(["--jobs", "2", "--", str(cargo), "test"])
            output = log.getvalue()
            self.assertEqual(code, 1)
            self.assertIn(f"cwd={package} pkg=horizon-world", output)
            self.assertIn("broken", output)
            self.assertIn("ran 2 test binaries", output)
            self.assertIn("::error::horizon-world lib.rs (failing): exit 101", output)
            # std's directory beside the toolchain: a proc-macro crate's test binary links it dynamically.
            self.assertIn(f"{root}/sysroot/lib/rustlib/x86_64-unknown-linux-gnu/lib", output)


if __name__ == "__main__":
    unittest.main()
