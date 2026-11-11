"""Self-check orchestration contracts; all check execution is mocked, no Cargo."""
import contextlib
import io
import unittest
from unittest.mock import patch

from tools import pr_ready


class RevisionOrchestration(unittest.TestCase):
    def test_starting_identity_survives_concurrent_commit_and_dirty_commit(self):
        for status in ("", " M tracked.rs\n"):
            for ending in (("b" * 40, "", ""), ("a" * 40, status, "changed contents")):
                start = ("a" * 40, status, "")
                calls = []
                def snapshot():
                    calls.append("snapshot")
                    return start if calls.count("snapshot") == 1 else ending
                def selected(_):
                    calls.append("selection")
                    self.assertEqual(calls[0], "snapshot")
                    return set()
                def checks(*_):
                    calls.append("checks")
                with patch.object(pr_ready, "revision_state", side_effect=snapshot), \
                        patch.object(pr_ready.parity, "changed_files", side_effect=selected), \
                        patch.object(pr_ready, "git", return_value="mergebase"), \
                        patch.object(pr_ready, "line_counts", return_value={}), \
                        patch.object(pr_ready, "plan", return_value=[pr_ready.Step("Cargo.lock", where="git")]), \
                        patch.object(pr_ready, "lock_problem", return_value=None), \
                        patch.object(pr_ready, "run_checks", side_effect=checks), \
                        patch.object(pr_ready, "rust_version", return_value="1.99.0"), \
                        patch.object(pr_ready, "old_rust", return_value=None), \
                        patch.object(pr_ready, "suggested_label", return_value="skip-release"), \
                        patch.object(pr_ready.release_train, "exporters", return_value=[]), \
                        patch("sys.argv", ["pr_ready.py", "--quick"]), \
                        contextlib.redirect_stdout(io.StringIO()) as output, \
                        contextlib.redirect_stderr(io.StringIO()):
                    self.assertEqual(pr_ready.main(), 0)
                header = output.getvalue().splitlines()[0]
                self.assertIn("commit `" + "a" * 40 + "`", header)
                self.assertNotIn("b" * 40, header)
                self.assertEqual(" dirty" in header, bool(status))
                self.assertIn("changed during run", header)
                self.assertEqual(calls, ["snapshot", "selection", "checks", "snapshot"])
