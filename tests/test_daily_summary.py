"""The daily summary: the daily-green tag, one issue per failing test, and nothing written outside main."""
import contextlib
import io
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from tools.ci import daily_summary as summary

COMMIT = "c" * 40
LEGS = {leg: {"result": "success"} for leg in summary.LEGS}


def job(name, conclusion="success", failed=()):
    steps = [{"name": "Set up job", "conclusion": "success"}] + [{"name": step, "conclusion": "failure"} for step in failed]
    return {"name": name, "conclusion": conclusion, "steps": steps}


GREEN_JOBS = [job("shards / plan"), job("shards / ignored-tests-shards (horizon-world)"),
              job("wasm-mods / fixture"), job("wasm-mods / Guest stack / ubuntu-latest"),
              job("fn-replace-check / route"), job("fn-replace-check / fn-replace-check"),
              job("linkage / route"), job("linkage / linkage x86_64-unknown-linux-gnu"),
              job("sdk / fresh"), job("sdk / build linux"), job("sdk / publish", "skipped")]


def shard_report(problems=(), passed=(), incomplete=()):
    return {"shards": ["horizon-world"], "problems": list(problems), "passed": list(passed),
            "incomplete": list(incomplete)}


def failure(test, panic="crates/horizon-world/src/lib.rs:7:3", kind="failed", package="horizon-world"):
    return {"package": package, "test": test, "kind": kind, "panic": panic, "policy": None}


class Api:
    """GitHub's REST API as the summary sees it: listings by page, and every write recorded."""

    def __init__(self, jobs, issues=(), tag=False, fail_on=None):
        self.jobs, self.issues, self.tag, self.fail_on, self.writes = jobs, list(issues), tag, fail_on, []

    def __call__(self, method, path, body=None):
        if self.fail_on and self.fail_on in path:
            raise SystemExit("::error::GitHub refused GET: HTTP 403 (rate limit exhausted)")
        if method != "GET":
            self.writes.append((method, path, body))
            return {}
        if "matching-refs" in path:
            return [{"ref": "refs/tags/daily-green"}] if self.tag else []
        page = int(path.rsplit("page=", 1)[1])
        if "/jobs" in path:
            return {"jobs": self.jobs[(page - 1) * 100:page * 100]}
        if path.startswith("issues?"):
            return self.issues[(page - 1) * 100:page * 100]
        raise AssertionError(path)


def git(merged=(), tag=True):
    def run(*args):
        if args[0] == "rev-parse":
            return "d" * 40 if tag else None
        return "\n".join(f"Merge pull request #{n} from arismoko/topic" for n in merged) + "\nMerge branch 'x'"
    return run


class DailySummaryTests(unittest.TestCase):
    def run_summary(self, api, reports=(), results=LEGS, ref="refs/heads/main", dry_run="false", merged=(), tag=True):
        with tempfile.TemporaryDirectory() as folder:
            for index, report in enumerate(reports):
                path = Path(folder, "reports", f"daily-report-{index}")
                path.mkdir(parents=True)
                (path / "daily-report.json").write_text(json.dumps(report))
            env = {"GITHUB_REPOSITORY": "arismoko/horizon", "GITHUB_RUN_ID": "77", "GITHUB_REF": ref,
                   "GITHUB_SERVER_URL": "https://github.com", "COMMIT": COMMIT, "DRY_RUN": dry_run,
                   "RESULTS": json.dumps(results), "REPORTS": str(Path(folder, "reports")),
                   "GITHUB_STEP_SUMMARY": str(Path(folder, "summary")), "GITHUB_OUTPUT": str(Path(folder, "out"))}
            with mock.patch.dict(os.environ, env, clear=True), contextlib.redirect_stdout(io.StringIO()):
                summary.main(api, git(merged, tag))
            return Path(folder, "summary").read_text(), Path(folder, "out").read_text()

    def test_all_green_moves_the_tag_and_closes_issues_that_went_green(self):
        api = Api(GREEN_JOBS, issues=[{"number": 5, "title": "Daily: horizon-world a::b"},
                                      {"number": 6, "title": "Daily: fn-replace-check / fn-replace-check did not complete"},
                                      {"number": 7, "title": "Daily: horizon-world still::unseen"},
                                      {"number": 8, "title": "Something else"}], tag=True)
        text, out = self.run_summary(api, [shard_report(passed=[{"package": "horizon-world", "test": "a::b"}])])
        self.assertIn("green=true", out)
        self.assertIn(("PATCH", "git/refs/tags/daily-green", {"sha": COMMIT, "force": True}), api.writes)
        closed = [path for method, path, body in api.writes if method == "PATCH" and path.startswith("issues/")]
        self.assertCountEqual(closed, ["issues/5", "issues/6"])
        self.assertFalse(any(path.startswith("issues/7") for _, path, _ in api.writes))
        self.assertIn("Move `daily-green`", text)

    def test_a_skipped_sdk_build_or_publish_is_fine_but_a_skipped_shard_is_not(self):
        jobs = [job for job in GREEN_JOBS if not job["name"].startswith("sdk")] + [
            job("sdk / fresh"), job("sdk / build linux", "skipped"), job("sdk / publish", "skipped")]
        _, out = self.run_summary(Api(jobs), [shard_report()])
        self.assertIn("green=true", out, "prebuilt-<sha> already exists, and a dry run publishes nothing")
        jobs = [job("shards / ignored-tests-shards (horizon-world)", "skipped")] + GREEN_JOBS[2:]
        api = Api(jobs)
        _, out = self.run_summary(api, [])
        self.assertIn("green=false", out, "a skipped shard is missing coverage")

    def test_a_failed_sdk_build_is_red(self):
        jobs = [job for job in GREEN_JOBS if not job["name"].startswith("sdk")] + [
            job("sdk / fresh"), job("sdk / build windows", "failure", ["Build and collect"])]
        api = Api(jobs)
        _, out = self.run_summary(api, [shard_report()], results=dict(LEGS, sdk={"result": "failure"}))
        self.assertIn("green=false", out)
        self.assertEqual([body["title"] for _, path, body in api.writes if path == "issues"],
                         ["Daily: sdk / build windows did not complete"])

    def test_the_tag_is_created_the_first_time(self):
        api = Api(GREEN_JOBS, tag=False)
        self.run_summary(api, [shard_report()])
        self.assertIn(("POST", "git/refs", {"ref": "refs/tags/daily-green", "sha": COMMIT}), api.writes)

    def test_a_failing_test_gets_one_issue_with_location_run_and_suspects(self):
        jobs = GREEN_JOBS[:1] + [job("shards / ignored-tests-shards (horizon-world)", "failure",
                                     ["Run the ignored tests that need prepared data"])] + GREEN_JOBS[2:]
        api = Api(jobs)
        report = shard_report([failure("game::x::tests::broken"), failure("game::x::tests::broken")])
        _, out = self.run_summary(api, [report], results=dict(LEGS, shards={"result": "failure"}), merged=[12, 15])
        self.assertIn("green=false", out)
        created = [body for method, path, body in api.writes if (method, path) == ("POST", "issues")]
        self.assertEqual(len(created), 1, "one issue per failing test")
        issue = created[0]
        self.assertEqual(issue["title"], "Daily: horizon-world game::x::tests::broken")
        self.assertEqual(issue["labels"], ["bug"])
        self.assertTrue(issue["body"].startswith(summary.DISCLOSURE + "\n"))
        for part in ("`crates/horizon-world/src/lib.rs:7:3`", "https://github.com/arismoko/horizon/actions/runs/77",
                     "#12, #15"):
            self.assertIn(part, issue["body"])
        self.assertFalse(any("tags" in path for _, path, _ in api.writes), "a red run never moves the tag")

    def test_an_open_issue_gets_a_comment_not_a_duplicate(self):
        jobs = [job("shards / ignored-tests-shards (horizon-world)", "failure", ["Run"])] + GREEN_JOBS[2:]
        issues = [{"number": n, "title": f"filler {n}"} for n in range(150)]
        issues.append({"number": 900, "title": "Daily: horizon-world game::x::tests::broken"})
        api = Api(jobs, issues=issues)
        self.run_summary(api, [shard_report([failure("game::x::tests::broken")])],
                         results=dict(LEGS, shards={"result": "failure"}))
        self.assertEqual([(m, p) for m, p, _ in api.writes], [("POST", "issues/900/comments")],
                         "found past the first page of open issues")
        self.assertTrue(api.writes[0][2]["body"].startswith("🤖 Agent: "))

    def test_a_leg_that_failed_without_test_names_gets_one_issue_per_leg(self):
        jobs = [job("shards / ignored-tests-shards (horizon-world)", "cancelled", ["Prepare checkout-keyed vanilla data"]),
                job("wasm-mods / fixture", "failure", ["Build tool and shared member"]),
                job("fn-replace-check / fn-replace-check", "failure", ["Refusals by name"])] + GREEN_JOBS[6:]
        api = Api(jobs)
        self.run_summary(api, [], results=dict(LEGS, shards={"result": "cancelled"}, linkage={"result": "skipped"}),
                         tag=False)
        titles = sorted(body["title"] for method, path, body in api.writes if path == "issues")
        self.assertEqual(titles, ["Daily: fn-replace-check / fn-replace-check did not complete",
                                  "Daily: shards / ignored-tests-shards (horizon-world) did not complete",
                                  "Daily: wasm-mods / fixture did not complete"])
        bodies = " ".join(body["body"] for _, path, body in api.writes if path == "issues")
        self.assertIn("`Refusals by name`", bodies)
        self.assertIn("no daily run has been green yet", bodies)

    def test_a_skipped_leg_with_no_jobs_is_not_green(self):
        api = Api([job for job in GREEN_JOBS if not job["name"].startswith("shards")])
        _, out = self.run_summary(api, [], results=dict(LEGS, shards={"result": "skipped"}))
        self.assertIn("green=false", out)
        self.assertIn("Daily: shards did not complete", [body["title"] for _, path, body in api.writes if path == "issues"])

    def test_fixture_test_failures_are_filed_by_test_name(self):
        jobs = GREEN_JOBS[:2] + [job("wasm-mods / fixture", "failure", [summary.FIXTURE_TESTS_STEP])] + GREEN_JOBS[3:]
        report = {"needs": "HORIZON_WASM_FIXTURE", "incomplete": [], "passed": [],
                  "problems": [failure("game::wasm::wasm_interaction_shared_matrix", None, package="horizon-client")]}
        api = Api(jobs)
        self.run_summary(api, [report], results=dict(LEGS, **{"wasm-mods": {"result": "failure"}}))
        titles = [body["title"] for _, path, body in api.writes if path == "issues"]
        self.assertEqual(titles, ["Daily: horizon-client game::wasm::wasm_interaction_shared_matrix"])

    def test_nothing_is_written_off_main_or_in_a_dry_run(self):
        jobs = [job("shards / ignored-tests-shards (horizon-world)", "failure", ["Run"])] + GREEN_JOBS[2:]
        for ref, dry_run in (("refs/heads/hotspots-daily", "false"), ("refs/heads/main", "true")):
            with self.subTest(ref=ref, dry_run=dry_run):
                api = Api(jobs)
                text, _ = self.run_summary(api, [shard_report([failure("a::b")])], ref=ref, dry_run=dry_run,
                                           results=dict(LEGS, shards={"result": "failure"}))
                self.assertEqual(api.writes, [])
                self.assertIn("(dry run: nothing written)", text)
                self.assertIn("File **Daily: horizon-world a::b**", text)

    def test_a_failed_lookup_writes_nothing(self):
        jobs = [job("shards / ignored-tests-shards (horizon-world)", "failure", ["Run"])] + GREEN_JOBS[2:]
        api = Api(jobs, fail_on="issues?")
        with self.assertRaises(SystemExit):
            self.run_summary(api, [shard_report([failure("a::b")])], results=dict(LEGS, shards={"result": "failure"}))
        self.assertEqual(api.writes, [])


class DailyWorkflowTests(unittest.TestCase):
    def setUp(self):
        from tests.test_ci_concurrency import ROOT, yaml
        self.workflow = yaml.load((ROOT / ".github/workflows/daily.yml").read_text(), Loader=yaml.BaseLoader)

    def test_the_schedule_only_dispatches_a_real_run_on_main(self):
        jobs = self.workflow["jobs"]
        self.assertEqual(self.workflow["on"]["schedule"], [{"cron": "0 9 * * *"}])
        self.assertIn("github.event_name == 'schedule'", jobs["schedule-dispatch"]["if"])
        self.assertEqual(jobs["schedule-dispatch"]["runs-on"], "ubuntu-latest")
        self.assertIn("--ref main -f dry_run=false", jobs["schedule-dispatch"]["steps"][0]["run"])
        self.assertEqual(jobs["resolve"]["if"], "github.event_name == 'workflow_dispatch'")
        self.assertEqual(self.workflow["on"]["workflow_dispatch"]["inputs"]["dry_run"]["default"], "true")

    def test_only_main_without_dry_run_writes(self):
        jobs = self.workflow["jobs"]
        step = jobs["resolve"]["steps"][-1]
        self.assertEqual(step["env"]["DRY_RUN"], "${{ inputs.dry_run || github.ref != 'refs/heads/main' }}")
        self.assertEqual(jobs["summary"]["steps"][-1]["env"]["DRY_RUN"], "${{ needs.resolve.outputs.dry_run }}")
        self.assertEqual(jobs["nightly"]["if"],
                         "needs.summary.outputs.green == 'true' && needs.resolve.outputs.dry_run == 'false'")
        self.assertEqual(self.workflow["concurrency"]["cancel-in-progress"], "false")
        self.assertEqual(jobs["sdk"]["with"]["publish"], "${{ needs.resolve.outputs.dry_run == 'false' }}")

    def test_every_leg_is_a_called_workflow_with_the_commit(self):
        jobs = self.workflow["jobs"]
        for leg in summary.LEGS:
            with self.subTest(leg=leg):
                self.assertTrue(jobs[leg]["uses"].startswith("./.github/workflows/"))
                self.assertEqual(jobs[leg]["with"]["commit"], "${{ needs.resolve.outputs.commit }}")
                self.assertIn(leg, jobs["summary"]["needs"])


if __name__ == "__main__":
    unittest.main()
