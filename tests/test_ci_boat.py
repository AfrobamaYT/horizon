"""Boat routing must distinguish runner outages from real test failures."""
import importlib.util
from pathlib import Path
import unittest
import sys
from unittest.mock import patch
import os
import tempfile
from datetime import datetime, timezone

spec = importlib.util.spec_from_file_location("boat_health", Path(__file__).parents[1] / "tools/ci/boat_health.py")
boat = importlib.util.module_from_spec(spec)
spec.loader.exec_module(boat)
sys.modules["boat_health"] = boat
watch_spec = importlib.util.spec_from_file_location("boat_watch", Path(__file__).parents[1] / "tools/ci/boat_watch.py")
watch = importlib.util.module_from_spec(watch_spec)
watch_spec.loader.exec_module(watch)
controller_spec = importlib.util.spec_from_file_location("boat_controller", Path(__file__).parents[1] / "tools/ci/boat_controller.py")
controller = importlib.util.module_from_spec(controller_spec)
controller_spec.loader.exec_module(controller)
NOW = datetime(2026, 11, 10, 12, tzinfo=timezone.utc)


def job(**changes):
    return dict(id=42, labels=["boat-large"], status="queued", conclusion=None,
                created_at="2026-11-10T11:49:59Z", steps=[], **changes)


class BoatHealthTests(unittest.TestCase):
    def test_stale_queue_falls_back(self):
        label, reason = boat.select_label("on", ["alice"], "alice", "alice", [job()], NOW)
        self.assertEqual(label, "ubuntu-latest")
        self.assertIn("42", reason)

    def test_threshold_is_strict(self):
        sample = job()
        sample["created_at"] = "2026-11-10T11:50:00Z"
        self.assertEqual(boat.health_reason([sample], NOW), "")

    def test_setup_failure(self):
        sample = job()
        sample.update(status="completed", conclusion="failure", steps=[
            dict(name="Set up job", started_at="2026-11-10T11:51:00Z", conclusion="failure"),
            dict(name="Complete job", started_at="2026-11-10T11:51:01Z")])
        self.assertIn("first step", boat.health_reason([sample], NOW))

    def test_real_failure_is_not_retried(self):
        sample = job()
        sample.update(status="completed", conclusion="failure", steps=[
            dict(name="Run tests", started_at="2026-11-10T11:51:00Z", conclusion="failure"),
            dict(name="Complete job", started_at="2026-11-10T11:51:01Z")])
        self.assertEqual(boat.health_reason([sample], NOW), "")

    def test_other_runners_do_not_poison_boat(self):
        sample = job()
        sample["labels"] = ["dyna-large"]
        self.assertEqual(boat.health_reason([sample], NOW), "")

    def test_mixed_real_failure_does_not_replace_run(self):
        queued = job()
        queued["name"] = "verify"
        failed = job()
        failed.update(name="lint", status="completed", conclusion="failure", steps=[
            dict(name="Run tests", started_at="2026-11-10T11:51:00Z")])
        self.assertEqual(watch.recovery_reason([queued, failed], NOW), "")
        failed["name"] = "launcher"
        self.assertEqual(watch.recovery_reason([queued, failed], NOW), "")
        self.assertIn("queued", watch.recovery_reason([queued], NOW))

    def test_previous_cancelled_queue_retains_outage(self):
        sample = job()
        sample.update(name="lint", status="completed", conclusion="cancelled", completed_at="2026-11-10T12:00:01Z")
        self.assertIn("previous attempt", boat.previous_attempt_reason([sample], NOW))
        sample["completed_at"] = "2026-11-10T11:50:00Z"
        self.assertEqual(boat.previous_attempt_reason([sample], NOW), "")
        sample["completed_at"] = "2026-11-10T12:00:01Z"
        sample["steps"] = [dict(name="Run tests", started_at="2026-11-10T11:51:00Z")]
        self.assertEqual(boat.previous_attempt_reason([sample], NOW), "")

    def test_controller_limits_to_first_fork_attempt(self):
        run = dict(workflow_id=123, event="pull_request", run_attempt=1, status="queued", head_repository=dict(full_name="other/horizon"))
        self.assertTrue(controller.eligible(run, "arismoko/horizon", 123))
        for key, value in [("run_attempt", 2), ("event", "push"), ("workflow_id", 999), ("head_repository", dict(full_name="arismoko/horizon"))]:
            self.assertFalse(controller.eligible(dict(run, **{key: value}), "arismoko/horizon", 123))

    def test_controller_cancel_then_rerun(self):
        sample = job()
        sample["name"] = "lint"
        cancelled = dict(sample, status="completed", conclusion="cancelled", completed_at="2026-11-10T12:00:01Z")
        with patch.object(controller, "api", side_effect=[{"jobs": [sample]}, {"status": "completed"}, {"jobs": [cancelled]}]), patch.object(controller, "post") as post, patch.object(controller, "log"), patch.object(controller, "datetime") as clock:
            clock.now.return_value = NOW
            controller.recover("arismoko/horizon", dict(id=123, status="in_progress"))
        self.assertEqual([call.args[0] for call in post.call_args_list], [
            "repos/arismoko/horizon/actions/runs/123/cancel", "repos/arismoko/horizon/actions/runs/123/rerun"])

    def test_controller_does_not_rerun_late_real_failure(self):
        sample = job()
        sample["name"] = "lint"
        failed = dict(sample, status="completed", conclusion="failure", steps=[dict(name="Run tests", started_at="2026-11-10T11:51:00Z")])
        with patch.object(controller, "api", side_effect=[{"jobs": [sample]}, {"status": "completed"}, {"jobs": [failed]}]), patch.object(controller, "post") as post, patch.object(controller, "log"), patch.object(controller, "datetime") as clock:
            clock.now.return_value = NOW
            controller.recover("arismoko/horizon", dict(id=123, status="in_progress"))
        self.assertEqual(len(post.call_args_list), 1)
        self.assertTrue(post.call_args.args[0].endswith("/cancel"))

    def test_controller_completed_setup_failure_needs_no_cancel(self):
        sample = job()
        sample.update(name="verify", status="completed", conclusion="failure")
        with patch.object(controller, "api", return_value={"jobs": [sample]}), patch.object(controller, "post") as post, patch.object(controller, "log"), patch.object(controller, "datetime") as clock:
            clock.now.return_value = NOW
            controller.recover("arismoko/horizon", dict(id=123, status="completed"))
        post.assert_called_once_with("repos/arismoko/horizon/actions/runs/123/rerun")

    def test_controller_real_failure_never_calls_api_writes(self):
        sample = job()
        sample.update(name="lint", status="completed", conclusion="failure", steps=[dict(name="Run tests", started_at="2026-11-10T11:51:00Z")])
        with patch.object(controller, "api", return_value={"jobs": [sample]}), patch.object(controller, "post") as post, patch.object(controller, "log"), patch.object(controller, "datetime") as clock:
            clock.now.return_value = NOW
            controller.recover("arismoko/horizon", dict(id=123, status="completed"))
        post.assert_not_called()

    def test_rerun_gate_outputs_forced_github_and_summary(self):
        sample = job()
        sample.update(name="lint", status="completed", conclusion="cancelled", completed_at="2026-11-10T12:00:01Z")
        with tempfile.TemporaryDirectory() as directory:
            output, summary = Path(directory) / "output", Path(directory) / "summary"
            with patch.dict(os.environ, dict(GITHUB_REPOSITORY="arismoko/horizon", GITHUB_RUN_ID="123", GITHUB_RUN_ATTEMPT="2", BOAT_CI="off", AUTHOR="alice", ACTOR="alice", GITHUB_OUTPUT=str(output), GITHUB_STEP_SUMMARY=str(summary))), patch.object(boat, "api", return_value={"jobs": [sample]}):
                boat.main()
            self.assertIn("label=ubuntu-latest", output.read_text())
            self.assertIn("forced_github=true", output.read_text())
            self.assertIn("previous attempt", summary.read_text())

    def test_allowlist_and_switch(self):
        for enabled, author, actor in [("off", "alice", "alice"), ("on", "bob", "alice"), ("on", "alice", "bob")]:
            self.assertEqual(boat.select_label(enabled, ["alice"], author, actor, [], NOW)[0], "ubuntu-latest")
        self.assertEqual(boat.select_label("on", ["alice"], "alice", "alice", [], NOW)[0], "boat-large")
