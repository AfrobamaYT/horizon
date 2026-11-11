"""Exercise CI apt recovery without installing packages or sleeping."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


class AptRetryTests(unittest.TestCase):
    def run_install(self, failed_command, failures):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            stubs = {
                "sudo": '#!/bin/bash\nexec "$@"\n',
                "sleep": '#!/bin/bash\necho "sleep $*" >> "$LOG"\n',
                "apt-get": '''#!/bin/bash
echo "apt $*" >> "$LOG"
if [[ " $* " == *" update "* ]]; then
  # Simulate an index failure that apt normally reports as success.
  [[ " $* " == *" APT::Update::Error-Mode=any "* ]] || exit 0
  command=update
else
  command=install
fi
n=$(cat "$STATE-$command" 2>/dev/null || echo 0)
echo $((n + 1)) > "$STATE-$command"
if [[ "$command" == "$FAILED_COMMAND" && "$n" -lt "$FAILURES" ]]; then
  echo 'Temporary failure resolving security.ubuntu.com' >&2
  exit 100
fi
''',
            }
            for name, content in stubs.items():
                path = root / name
                path.write_text(content)
                path.chmod(0o755)
            env = dict(os.environ, PATH=f"{root}:{os.environ['PATH']}",
                       LOG=str(root / "log"), STATE=str(root / "state"),
                       FAILED_COMMAND=failed_command, FAILURES=str(failures),
                       DYNA_CI_APT_TRIES="4", DYNA_CI_APT_PAUSE="5")
            result = subprocess.run(
                ["bash", ROOT / "tools/ci/install_launcher_deps.sh", "libx11-xcb-dev", "libudev-dev"],
                env=env, text=True, capture_output=True)
            return result, (root / "log").read_text().splitlines()

    def test_transient_index_and_download_failures_recover(self):
        for command in ("update", "install"):
            with self.subTest(command=command):
                result, calls = self.run_install(command, 2)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual([c for c in calls if c.startswith("sleep")], ["sleep 5", "sleep 10"])
                self.assertEqual(len([c for c in calls if c.startswith("apt")]), 4)
                self.assertIn("apt install -y --no-install-recommends libx11-xcb-dev libudev-dev", calls)

    def test_persistent_errors_fail_closed_at_the_bound(self):
        for command in ("update", "install"):
            with self.subTest(command=command):
                result, calls = self.run_install(command, 99)
                self.assertEqual(result.returncode, 100, result.stderr)
                self.assertEqual([c for c in calls if c.startswith("sleep")],
                                 ["sleep 5", "sleep 10", "sleep 15"])
                updates = [c for c in calls if c.endswith(" update")]
                installs = [c for c in calls if c.startswith("apt install")]
                self.assertEqual(len(updates), 4 if command == "update" else 1)
                self.assertEqual(len(installs), 0 if command == "update" else 4)


if __name__ == "__main__":
    unittest.main()
