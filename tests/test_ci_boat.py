"""Boat routing must distinguish runner outages from real test failures."""
import importlib.util
from pathlib import Path
import unittest
import sys
from datetime import datetime, timezone

spec = importlib.util.spec_from_file_location("boat_health", Path(__file__).parents[1] / "tools/ci/boat_health.py")
boat = importlib.util.module_from_spec(spec)
spec.loader.exec_module(boat)
sys.modules["boat_health"] = boat
watch_spec = importlib.util.spec_from_file_location("boat_watch", Path(__file__).parents[1] / "tools/ci/boat_watch.py")
watch = importlib.util.module_from_spec(watch_spec)
watch_spec.loader.exec_module(watch)
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

    def test_allowlist_and_switch(self):
        for enabled, author, actor in [("off", "alice", "alice"), ("on", "bob", "alice"), ("on", "alice", "bob")]:
            self.assertEqual(boat.select_label(enabled, ["alice"], author, actor, [], NOW)[0], "ubuntu-latest")
        self.assertEqual(boat.select_label("on", ["alice"], "alice", "alice", [], NOW)[0], "boat-large")
