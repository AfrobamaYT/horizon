"""Dyna's switch routes only supported, same-repository events to its pool."""
import hashlib
import os
from pathlib import Path
import subprocess
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[1]


class DynaRoutingTests(unittest.TestCase):
    def route(self, event, same_repo="true", enabled="true", lease=None):
        env = dict(os.environ, EVENT=event, SAME_REPO=same_repo, ENABLED=enabled,
                   LEASE=str(int(time.time()) + 3600) if lease is None else lease)
        return subprocess.check_output(["bash", ROOT / "tools/ci/dyna_ci_runner.sh"], env=env, text=True).strip()

    def test_only_allowed_events_and_same_repository(self):
        for event in ("push", "workflow_dispatch", "pull_request"):
            self.assertEqual(self.route(event), "self_hosted=true", event)
        for event in ("pull_request_target", "workflow_run", "issue_comment", "schedule", ""):
            self.assertEqual(self.route(event), "self_hosted=false", event)
        self.assertEqual(self.route("pull_request", same_repo="false"), "self_hosted=false")

    def test_switch_and_lease_fail_closed(self):
        self.assertEqual(self.route("push", enabled="false"), "self_hosted=false")
        for lease in ("", "bad", "0"):
            self.assertEqual(self.route("push", lease=lease), "self_hosted=false")

    def test_cached_dependencies_export_persistent_paths(self):
        script = ROOT / "tools/ci/dyna_ci_deps.sh"
        with tempfile.TemporaryDirectory() as directory:
            cache = Path(directory)
            prefix = cache / "apt-prefix"
            prefix.mkdir()
            (prefix / ".dyna-hash").write_text(hashlib.sha256(script.read_bytes()).hexdigest() + "\n")
            envfile, pathfile = cache / "env", cache / "path"
            env = dict(os.environ, DYNA_CI_BUILD=directory, GITHUB_ENV=str(envfile), GITHUB_PATH=str(pathfile))
            subprocess.run(["bash", script], env=env, check=True)
            exports = envfile.read_text()
            for key, value in (("CARGO_TARGET_DIR", "target"), ("CARGO_HOME", "cargo"), ("RUSTUP_HOME", "rustup")):
                self.assertIn(f"{key}={cache / value}\n", exports)
            self.assertNotIn("CARGO_INCREMENTAL", exports)
            self.assertIn(str(cache / "cargo/bin"), pathfile.read_text())

    def test_workflow_wires_route_and_keeps_privileges_off_pool(self):
        from test_ci_runners import jobs
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        blocks = dict(jobs(workflow))
        route = blocks["changes"]
        for item in ("vars.DYNA_CI_SELF_HOSTED", "vars.DYNA_CI_POOL_UNTIL", "github.event_name",
                     "github.event.pull_request.head.repo.full_name == github.repository", "dyna_ci_runner.sh"):
            self.assertIn(item, route)
        for name in ("verify", "launcher"):
            block = blocks[name]
            self.assertIn("needs: changes", block)
            self.assertIn("needs.changes.outputs.self_hosted == 'true' && 'dyna-medium'", block)
            self.assertNotIn("secrets.", block)
            self.assertNotIn("permissions:", block)
        self.assertIn("runs-on: ubuntu-latest", blocks["pool-watch"])
        self.assertIn("permissions:\n  contents: read", workflow)


if __name__ == "__main__":
    unittest.main()
