import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from tools.ci import upstream_sync as sync


def git(folder, *args):
    return subprocess.run(["git", "-C", str(folder), *args], check=True, text=True,
                          capture_output=True).stdout.strip()


class SyncTests(unittest.TestCase):
    """Real scratch remotes and the production shell merge; only GitHub is mocked."""

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="upstream-sync-")
        self.addCleanup(self.temp.cleanup)
        root = Path(self.temp.name)
        self.origin, self.upstream, self.work = (root / name for name in ("origin", "upstream", "work"))
        seed = root / "seed"
        for bare in (self.origin, self.upstream):
            bare.mkdir()
            git(bare, "init", "--bare")
        seed.mkdir()
        git(seed, "init", "-b", "main")
        git(seed, "config", "user.name", "Test")
        git(seed, "config", "user.email", "test@example.invalid")
        (seed / "shared.txt").write_text("base\n")
        git(seed, "add", ".")
        git(seed, "commit", "-m", "base")
        git(seed, "push", str(self.origin), "main")
        subprocess.run(["git", "clone", "--branch", "main", str(self.origin), str(self.work)],
                       check=True, capture_output=True)
        git(self.work, "config", "user.name", "Test")
        git(self.work, "config", "user.email", "test@example.invalid")
        self.before = git(self.work, "rev-parse", "HEAD")
        (seed / "shared.txt").write_text("upstream\n")
        git(seed, "commit", "-am", "upstream")
        git(seed, "push", str(self.upstream), "main")
        self.upstream_sha = git(seed, "rev-parse", "HEAD")
        self.calls, self.open_issues = [], []
        original = sync.command

        def command(*args):
            self.calls.append(args)
            if args[0] == "gh":
                return json.dumps(self.open_issues) if args[1:3] == ("issue", "list") else ""
            if args[:2] == ("git", "fetch"):
                args = tuple(str(self.upstream) if arg == "https://github.com/horizon-rust/horizon.git" else arg
                             for arg in args)
            return original(*args)

        self.enterContext(patch.object(sync, "command", side_effect=command))
        self.enterContext(patch.dict(os.environ, GITHUB_REPOSITORY="arismoko/horizon", GITHUB_RUN_ID="123",
                                    GITHUB_RUN_ATTEMPT="1", GITHUB_SERVER_URL="https://github.com"))
        cwd = Path.cwd()
        os.chdir(self.work)
        self.addCleanup(os.chdir, cwd)

    def commit_fork(self, filename):
        (self.work / filename).write_text("fork\n")
        git(self.work, "add", ".")
        git(self.work, "commit", "-m", "fork")
        git(self.work, "push", "origin", "main")
        self.before = git(self.work, "rev-parse", "HEAD")

    def test_clean_merge_verified_before_main_push_and_then_noop(self):
        self.commit_fork("fork.txt")
        self.open_issues = [{"number": 9, "title": sync.TITLE}]

        def verify(repo, head, request):
            self.assertEqual(git(self.origin, "rev-parse", "main"), self.before)
            self.assertEqual(git(self.origin, "rev-parse", "sync/upstream"), head)
            return "https://github.com/checks/1"

        with patch.object(sync, "verify", side_effect=verify) as check:
            self.assertEqual(sync.sync(), 0)
            check.assert_called_once()
            head = git(self.work, "rev-parse", "HEAD")
            self.assertEqual(git(self.origin, "rev-parse", "main"), head)
            self.assertEqual(git(self.work, "show", "-s", "--format=%P"), f"{self.before} {self.upstream_sha}")
            identity = "arismoko <19542195+arismoko@users.noreply.github.com>"
            self.assertEqual(git(self.work, "show", "-s", "--format=%an <%ae>%n%cn <%ce>"), f"{identity}\n{identity}")
            self.assertEqual(git(self.work, "show", "-s", "--format=%B"),
                             f"Merge upstream main {git(self.work, 'rev-parse', '--short', 'upstream/main')}")
            self.assertTrue(any(call[1:3] == ("issue", "close") for call in self.calls))
            self.open_issues = []
            self.assertEqual(sync.sync(), 0)
            check.assert_called_once()
        print("CLEAN: two-parent merge; exact identity/message; verified staging SHA promoted; issue closed; NOOP skips checks")

    def test_conflict_pushes_nothing_and_updates_one_issue(self):
        self.commit_fork("shared.txt")
        self.open_issues = [{"number": 9, "title": sync.TITLE}]
        with patch.object(sync, "verify") as check:
            self.assertEqual(sync.sync(), 1)
            check.assert_not_called()
        self.assertEqual(git(self.origin, "rev-parse", "main"), self.before)
        self.assertEqual(git(self.origin, "for-each-ref", sync.STAGING), "")
        self.assertEqual(git(self.work, "status", "--porcelain"), "")
        comment = next(call[-1] for call in self.calls if call[1:3] == ("issue", "comment"))
        self.assertIn(self.upstream_sha, comment)
        self.assertIn("shared.txt", comment)
        self.assertFalse(any(call[1:3] == ("issue", "create") for call in self.calls))
        print("CONFLICT: shared.txt reported with upstream SHA; merge aborted; neither main nor staging pushed; issue commented")

    def test_workflow_changes_require_agent_without_any_push(self):
        seed = self.work.parent / "seed"
        workflow = seed / ".github/workflows/checks.yml"
        workflow.parent.mkdir(parents=True)
        workflow.write_text("name: checks\n")
        git(seed, "add", ".")
        git(seed, "commit", "-m", "change workflow")
        git(seed, "push", str(self.upstream), "main")
        self.upstream_sha = git(seed, "rev-parse", "HEAD")
        with patch.object(sync, "verify") as check:
            self.assertEqual(sync.sync(), 1)
            check.assert_not_called()
        self.assertEqual(git(self.origin, "rev-parse", "main"), self.before)
        self.assertEqual(git(self.origin, "for-each-ref", sync.STAGING), "")
        issue = next(call[-1] for call in self.calls if call[1:3] == ("issue", "create"))
        self.assertIn("Upstream changed workflow files; an agent must push this one", issue)
        self.assertIn(".github/workflows/checks.yml", issue)
        self.assertIn(self.upstream_sha, issue)
        print("WORKFLOWS: token limit detected before any push; paths/SHA reported for agent intervention")

    def test_concurrent_main_advance_is_not_overwritten(self):
        competitor = self.work.parent / "competitor"
        subprocess.run(["git", "clone", "--branch", "main", str(self.origin), str(competitor)],
                       check=True, capture_output=True)
        git(competitor, "config", "user.name", "Test")
        git(competitor, "config", "user.email", "test@example.invalid")
        (competitor / "other.txt").write_text("other work\n")
        git(competitor, "add", ".")
        git(competitor, "commit", "-m", "concurrent work")
        other = git(competitor, "rev-parse", "HEAD")

        def verify(*args):
            git(competitor, "push", "origin", "main")
            return "https://github.com/checks/1"

        with patch.object(sync, "verify", side_effect=verify):
            self.assertEqual(sync.sync(), 1)
        self.assertEqual(git(self.origin, "rev-parse", "main"), other)
        print("RACE: non-fast-forward main push refused; concurrent work preserved; issue opened")

    def test_red_checks_leave_main_unchanged_and_open_new_issue(self):
        with patch.object(sync, "verify", side_effect=RuntimeError("verify failed")):
            self.assertEqual(sync.sync(), 1)
        self.assertEqual(git(self.origin, "rev-parse", "main"), self.before)
        self.assertNotEqual(git(self.origin, "rev-parse", "sync/upstream"), self.before)
        issue = next(call[-1] for call in self.calls if call[1:3] == ("issue", "create"))
        self.assertIn("verify failed", issue)
        self.assertIn(self.upstream_sha, issue)
        self.assertFalse(any(call[1:3] == ("issue", "reopen") for call in self.calls))
        print("RED: candidate staged, main unchanged; new labelled issue instead of reopening a closed one")


class VerifyTests(unittest.TestCase):
    def run_checks(self, *, sha="candidate", conclusion="success", verify="success"):
        run = {"id": 7, "display_title": "Upstream sync 123-1", "html_url": "https://github.com/checks/7",
               "head_sha": sha, "status": "completed", "conclusion": conclusion}
        job = {"name": "verify", "head_sha": sha, "conclusion": verify, "check_run_url": "https://api.github.com/checks/7"}
        with patch.object(sync, "command"), patch.object(sync, "api", side_effect=[
                {"workflow_runs": [dict(run, display_title="unrelated"), run]}, {"jobs": [job]}]):
            return sync.verify("arismoko/horizon", "candidate", "123-1")

    def test_exact_request_and_sha_with_named_green_verify(self):
        self.assertEqual(self.run_checks(), "https://github.com/checks/7")

    def test_waits_for_own_dispatch_to_appear_and_complete(self):
        run = {"id": 7, "display_title": "Upstream sync 123-1", "html_url": "https://github.com/checks/7",
               "head_sha": "candidate", "status": "queued", "conclusion": None}
        job = {"name": "verify", "head_sha": "candidate", "conclusion": "success", "check_run_url": "check/7"}
        answers = [{"workflow_runs": []}, {"workflow_runs": [run]}, dict(run, status="in_progress"),
                   dict(run, status="completed", conclusion="success"), {"jobs": [job]}]
        with patch.object(sync, "api", side_effect=answers), patch.object(sync, "command") as command, \
                patch.object(sync.time, "monotonic", return_value=0), patch.object(sync.time, "sleep") as sleep:
            self.assertEqual(sync.verify("arismoko/horizon", "candidate", "123-1"), run["html_url"])
            command.assert_called_once_with("gh", "workflow", "run", "ci.yml", "--repo", "arismoko/horizon",
                                            "--ref", "sync/upstream", "--field", "upstream_sync=123-1")
            self.assertEqual([call.args[0] for call in sleep.call_args_list], [15, 30, 30])

    def test_stalled_checks_are_cancelled_and_fail_closed(self):
        run = {"id": 7, "display_title": "Upstream sync 123-1", "html_url": "https://github.com/checks/7",
               "head_sha": "candidate", "status": "queued"}
        with patch.object(sync, "api", return_value={"workflow_runs": [run]}), \
                patch.object(sync, "command") as command, \
                patch.object(sync.time, "monotonic", side_effect=[0, 0, 6301]), \
                self.assertRaisesRegex(RuntimeError, "exceeded 105 minutes"):
            sync.verify("arismoko/horizon", "candidate", "123-1")
        self.assertEqual(command.call_args.args, ("gh", "run", "cancel", "7", "--repo", "arismoko/horizon"))

    def test_missing_dispatch_fails_closed(self):
        with patch.object(sync, "api", return_value={"workflow_runs": []}), patch.object(sync, "command"), \
                patch.object(sync.time, "monotonic", side_effect=[0, 301]), \
                self.assertRaisesRegex(RuntimeError, "did not appear"):
            sync.verify("arismoko/horizon", "candidate", "123-1")

    def test_wrong_sha_red_workflow_and_skipped_verify_fail_closed(self):
        for options in ({"sha": "wrong"}, {"conclusion": "failure"}, {"verify": "skipped"}):
            with self.subTest(options=options), self.assertRaises(RuntimeError):
                self.run_checks(**options)
