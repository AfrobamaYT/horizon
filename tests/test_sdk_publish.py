"""The prebuilt SDK is published in one file, sdk-publish.yml, which every publisher calls (#4022)."""
import unittest

from tests.test_ci_concurrency import ROOT, yaml

WORKFLOWS = ROOT / ".github/workflows"


def load(name):
    return yaml.load((WORKFLOWS / name).read_text(), Loader=yaml.BaseLoader)


class SdkPublishTests(unittest.TestCase):
    def test_only_sdk_publish_releases_an_sdk(self):
        publishers = sorted(path.name for path in WORKFLOWS.glob("*.yml") if "Release prebuilt-<sha>" in path.read_text())
        self.assertEqual(publishers, ["sdk-publish.yml"])
        callers = sorted(path.name for path in WORKFLOWS.glob("*.yml")
                         if "uses: ./.github/workflows/sdk-publish.yml" in path.read_text())
        self.assertEqual(callers, ["daily.yml", "prebuilt-game.yml", "release.yml"])

    def test_one_step_refuses_an_incomplete_set_and_never_replaces_a_release(self):
        workflow = load("sdk-publish.yml")
        self.assertEqual(workflow["permissions"], {"contents": "write"})
        self.assertEqual(set(workflow["on"]["workflow_call"]["inputs"]), {"commit", "tag"})
        publish = workflow["jobs"]["publish"]
        self.assertEqual(publish["env"]["TAG"], "${{ inputs.tag }}", "the tag comes only from the input")
        steps = {step.get("name") or step.get("uses"): step for step in publish["steps"]}
        self.assertEqual(steps["Is prebuilt-<sha> published"]["id"], "published")
        download = steps["actions/download-artifact@v4"]
        self.assertNotIn("continue-on-error", download, "a failed download stops the publish")
        release = steps["Release prebuilt-<sha>"]
        for step in (download, release):
            self.assertEqual(step["if"], "steps.published.outputs.published == 'false'")
        self.assertEqual([step.get("name") for step in publish["steps"]][-1], "Release prebuilt-<sha>")
        self.assertLess(release["run"].index("python3 tools/ci/nightly.py sdk-archives"),
                        release["run"].index("gh release create"), "refuses unless exactly the four archives are there")
        self.assertEqual(release["run"].count("gh release"), 1, "creates; never deletes or edits an SDK")

    def test_the_hand_dispatch_publishes_from_main_only(self):
        publish = load("prebuilt-game.yml")["jobs"]["publish"]
        self.assertEqual(publish["if"], "github.ref == 'refs/heads/main' && needs.build.outputs.needed == 'true'")
        self.assertEqual(publish["with"], {"commit": "${{ github.sha }}", "tag": "${{ needs.build.outputs.tag }}"})


if __name__ == "__main__":
    unittest.main()
