"""Snapshots (#4022): release.yml kind snapshot builds a commit of main with the gate green and publishes it under its
own tag, never as the Nightly and never moving `daily-green`."""
import unittest

from tests.test_ci_concurrency import ROOT, yaml

WORKFLOWS = ROOT / ".github/workflows"


def load(name):
    return yaml.load((WORKFLOWS / name).read_text(), Loader=yaml.BaseLoader)


class ReleaseSnapshotTests(unittest.TestCase):
    def setUp(self):
        self.workflow = load("release.yml")
        self.jobs = self.workflow["jobs"]
        self.snapshot = {name: job for name, job in self.jobs.items() if name.startswith("snapshot")}

    def test_only_main_without_dry_run_publishes(self):
        mode = self.jobs["decide"]["steps"][1]
        self.assertEqual(mode["run"],
                         "echo \"dry_run=${{ inputs.dry_run || github.ref != 'refs/heads/main' }}\" >> \"$GITHUB_OUTPUT\"")
        self.assertEqual(self.workflow["on"]["workflow_dispatch"]["inputs"]["dry_run"]["default"], "true")
        publish = self.snapshot["snapshot-sdk-publish"]
        self.assertIn("needs.decide.outputs.dry_run != 'true'", publish["if"])
        self.assertEqual(publish["uses"], "./.github/workflows/sdk-publish.yml")
        # The game is built by `nightly build`, which publishes only when its own dry_run is false: it gets ours.
        run = self.snapshot["snapshot"]["steps"][0]["run"]
        self.assertIn('-f dry_run="$DRY_RUN"', run)
        self.assertEqual(self.snapshot["snapshot"]["steps"][0]["env"]["DRY_RUN"], "${{ needs.decide.outputs.dry_run }}")

    def test_a_snapshot_never_moves_daily_green_or_writes_the_nightly(self):
        text = yaml.safe_dump(self.snapshot)
        self.assertNotIn("daily-green", text)
        self.assertNotIn("git tag", text)
        self.assertNotIn("gh release", text)
        run = self.snapshot["snapshot"]["steps"][0]["run"]
        self.assertIn("gh workflow run nightly-build.yml --repo \"$GITHUB_REPOSITORY\" --ref main -f kind=snapshot", run)
        for name, job in self.snapshot.items():
            self.assertIn("inputs.kind == 'snapshot'", job["if"], name)
        self.assertIn("inputs.kind == 'nightly'", self.jobs["nightly"]["if"])

    def test_the_sdk_is_built_and_published_in_this_run_before_the_game(self):
        self.assertEqual(self.snapshot["snapshot-sdk"]["uses"], "./.github/workflows/prebuilt-game-build.yml")
        self.assertEqual(self.snapshot["snapshot-sdk"]["with"], {"commit": "${{ needs.decide.outputs.head }}"})
        self.assertEqual(self.snapshot["snapshot-sdk-publish"]["with"],
                         {"commit": "${{ needs.decide.outputs.head }}", "tag": "${{ needs.snapshot-sdk.outputs.tag }}"})
        game = self.snapshot["snapshot"]
        self.assertEqual(game["needs"], ["decide", "snapshot-sdk", "snapshot-sdk-publish"])
        self.assertIn("needs.snapshot-sdk.result == 'success'", game["if"])
        self.assertIn("needs.snapshot-sdk-publish.result == 'success'", game["if"])

    def test_snapshots_never_queue_with_nightlies(self):
        self.assertIn("inputs.kind == 'snapshot' && '-snapshot'", self.workflow["concurrency"]["group"])


if __name__ == "__main__":
    unittest.main()
