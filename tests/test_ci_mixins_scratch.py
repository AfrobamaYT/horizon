"""Exercise the workflow's scratch allocation without starting a build."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[1]


@unittest.skipUnless(os.name == "posix" and shutil.which("bash"), "requires POSIX Bash")
class MixinsScratchTests(unittest.TestCase):
    def test_self_hosted_scratch_is_private_unique_and_off_tmpfs(self):
        """Run the actual step twice and record its build-script arguments."""
        workflow = (ROOT / ".github/workflows/mixins-check.yml").read_text()
        block = workflow.split("      - name: Build the mod through the launcher at this commit\n", 1)[1]
        block = block.split("      - name:", 1)[0].split("        run: |\n", 1)[1]
        script = textwrap.dedent(block)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            stub = root / "tools/ci/mixins_check.sh"
            stub.parent.mkdir(parents=True)
            stub.write_text('#!/bin/bash\nset -eu\n'
                            'printf "%s\\n" "$2" >> "$RECEIPT"\n'
                            'if [ "$RUNNER_ENVIRONMENT" = self-hosted ]; then\n'
                            '  test -d "$2" && test -O "$2"\n'
                            '  test "$(stat -c %a "$2")" = 700\n'
                            '  rm -rf "$2" && mkdir -p "$2/sdk-target"\n'
                            '  test "$(stat -c %a "$2")" = 700\n'
                            'fi\n')
            stub.chmod(0o755)
            receipt = root / "receipt"
            env = dict(os.environ, GITHUB_WORKSPACE=str(root), RUNNER_TEMP=str(root / "tmpfs"),
                       RUNNER_ENVIRONMENT="self-hosted", RECEIPT=str(receipt))
            for _ in range(2):
                subprocess.run(["bash", "-e", "-c", script], cwd=root, env=env, check=True)
            paths = [Path(p) for p in receipt.read_text().splitlines()]
            self.assertEqual(len(set(paths)), 2)
            for path in paths:
                self.assertEqual(path.parent, root / "target")
                self.assertFalse(path.exists(), "scratch is cleaned even on a persistent build volume")
            subprocess.run(["bash", "-e", "-c", script], cwd=root,
                           env=dict(env, RUNNER_ENVIRONMENT="github-hosted"), check=True)
            self.assertEqual(receipt.read_text().splitlines()[-1], str(root / "tmpfs/mixins-check"))


if __name__ == "__main__":
    unittest.main()
