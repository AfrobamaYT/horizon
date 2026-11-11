"""Observable preparation diagnostics without executing Cargo, Java or the preparer."""
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "tools/ci/preparation_step.py"


class PreparationStepTests(unittest.TestCase):
    def invoke(self, root, code, step="resource-export"):
        return subprocess.run([sys.executable, str(SCRIPT), step, str(root / "private.log"),
                               sys.executable, "-c", code], capture_output=True, text=True)

    def test_failure_is_private_and_status_survives(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            for marker, reason in [("No space left on device", "disk-full"),
                                   ("Permission denied", "permission-denied"),
                                   ("Read-only file system", "read-only-filesystem"),
                                   ("OutOfMemoryError", "java-out-of-memory"),
                                   ("Could not resolve host", "network-dns"),
                                   ("Connection timed out", "network-timeout"),
                                   ("No such file or directory", "missing-file"),
                                   ("unknown failure", "unclassified")]:
                with self.subTest(reason=reason):
                    payload = "token=SECRET Mojang contents ::error::injected " + marker
                    result = self.invoke(root, f"import sys; print({payload!r}); sys.exit(23)")
                    self.assertEqual(result.returncode, 23)
                    self.assertEqual(result.stdout, f"::error::preparation step=resource-export exit=23 reason_hint={reason}\n")
                    self.assertEqual(result.stderr, "")
                    self.assertIn(payload, (root / "private.log").read_text())

    def test_success_never_prints_private_output(self):
        with tempfile.TemporaryDirectory() as folder:
            result = self.invoke(Path(folder), "print('SECRET No space left on device')")
            self.assertEqual((result.returncode, result.stdout, result.stderr), (0, "", ""))

    def test_missing_log_parent_has_safe_failure(self):
        with tempfile.TemporaryDirectory() as folder:
            result = self.invoke(Path(folder) / "absent", "raise AssertionError('must not run')")
            self.assertEqual(result.returncode, 1)
            self.assertEqual(result.stdout, "::error::preparation step=resource-export exit=1 reason_hint=missing-file\n")
            self.assertEqual(result.stderr, "")

    def test_signal_and_step_identity(self):
        with tempfile.TemporaryDirectory() as folder:
            result = self.invoke(Path(folder), "import os, signal; os.kill(os.getpid(), signal.SIGTERM)", "preparer-build")
            self.assertEqual(result.returncode, 143)
            self.assertEqual(result.stdout, "::error::preparation step=preparer-build exit=-15 reason_hint=unclassified\n")

    def test_shell_cleanup_preserves_failure(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "tools/ci").mkdir(parents=True)
            for name in ("vanilla-data.sh", "preparation_step.py"):
                shutil.copy(SCRIPT.parent / name, root / "tools/ci" / name)
            (root / "research-lock.json").write_text('{"version": "26.3"}\n')
            (root / "source/minecraft/26.3").mkdir(parents=True)
            (root / "source/minecraft/26.3/prepared.json").write_text('{}')
            (root / "bin").mkdir()
            cargo = root / "bin/cargo"
            cargo.write_text('#!/bin/sh\nexit 0\n')
            cargo.chmod(0o755)
            (root / "target/release").mkdir(parents=True)
            prepare = root / "target/release/horizon-prepare"
            prepare.write_text('#!/bin/sh\nmkdir .local/scratch\necho "SECRET No space left on device"\nexit 23\n')
            prepare.chmod(0o755)
            env = dict(os.environ, HORIZON_DYNA_CACHE="true", HORIZON_PREPARED_CACHE=str(root / "cache"),
                       HORIZON_SHARED_SOURCE=str(root / "source"), HORIZON_PREPARE_KEY="test",
                       RUNNER_TEMP=str(root), CARGO_TARGET_DIR=str(root / "target"),
                       PATH=str(root / "bin") + os.pathsep + os.environ["PATH"])
            result = subprocess.run(["bash", "tools/ci/vanilla-data.sh"], cwd=root, env=env,
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 23, result.stderr)
            self.assertIn('step=resource-export exit=23 reason_hint=disk-full', result.stdout)
            self.assertNotIn('SECRET', result.stdout + result.stderr)
            self.assertFalse((root / '.local/scratch').exists())
            self.assertFalse((root / 'cache/prepared/test').exists())
            self.assertTrue((root / 'source/minecraft/26.3/prepared.json').exists())

    def test_bounded_log_does_not_hide_exit(self):
        with tempfile.TemporaryDirectory() as folder:
            result = self.invoke(Path(folder), "import sys; print('x' * 1100000); print('No space left on device SECRET'); sys.exit(7)", "seed-extras")
            self.assertEqual(result.returncode, 7)
            self.assertEqual(result.stdout, "::error::preparation step=seed-extras exit=7 reason_hint=unclassified\n")


if __name__ == "__main__":
    unittest.main()
