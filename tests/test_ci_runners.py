"""Which jobs may run on our own runners (docs/ci-runners.md).

- horizon-light (playhorizon's and other small runners, which refuse a fork's pull request): main's own scripts only.
- horizon-checks (isolated runners that take any pull request, forks included): no secret and no token that writes.
- Neither takes a job that holds a secret or whose token can push, publish or start workflows: playhorizon is a
  partner's server, and a checks runner keeps what a fork's job left behind.
"""
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORKFLOWS = ROOT / ".github/workflows"
LIGHT = ("horizon-light", "playhorizon-light", "playhorizon-ci")
CHECKS = ("horizon-checks",)
FORK_GUARD = "github.event.pull_request.head.repo.full_name == github.repository"
TRUSTED_WRITES = re.compile(r"^\s*(contents|actions|packages|id-token|deployments|attestations): write", re.M)
# actions: write can start workflows too; cancel-on-close needs it to cancel runs and runs no checkout at all.
CANCELS_RUNS = {"cancel-on-close.yml cancel"}


def jobs(text):
    """(name, block) for each job of a workflow, by indentation (two spaces under `jobs:`)."""
    body = text.split("\njobs:\n", 1)[1]
    parts = re.split(r"^  ([A-Za-z0-9_-]+):\n", body, flags=re.M)
    return list(zip(parts[1::2], parts[2::2]))


def triggers(text):
    head = text.split("\njobs:\n", 1)[0]
    on = re.search(r"^on:\n((?:[ #].*\n|\n)*)", head, re.M)
    return set(re.findall(r"^  ([a-z_]+):", on.group(1), re.M)) if on else set()


def runs_on(block):
    """The job's runs-on value, a folded (`>-`) one included."""
    lines = block.splitlines()
    for i, line in enumerate(lines):
        if line.strip().startswith("runs-on:"):
            value = line.split("runs-on:", 1)[1].strip()
            if value in (">-", ">", "|"):
                indent = len(line) - len(line.lstrip()) + 2
                # The folded lines right under it.
                folded = []
                for l in lines[i + 1:]:
                    if not l.startswith(" " * indent):
                        break
                    folded.append(l.strip())
                value = " ".join(folded)
            return value
    return ""


def routed():
    """(where, triggers, workflow text, job block, runs-on, pools) for every job that can run on our runners."""
    for path in sorted(WORKFLOWS.glob("*.yml")):
        text = path.read_text()
        on = triggers(text)
        for name, block in jobs(text):
            value = runs_on(block)
            pools = {pool for pool, labels in (("light", LIGHT), ("checks", CHECKS))
                     if any(f'"{label}"' in value for label in labels)}
            if pools:
                yield f"{path.name} {name}", on, text, block, value, pools


class OwnRunnerTests(unittest.TestCase):
    def test_wasm_fixture_routes_by_repository_identity(self):
        workflow = (WORKFLOWS / "wasm-mods.yml").read_text()
        expression = runs_on(dict(jobs(workflow))["fixture"])
        self.assertTrue(expression.startswith("${{ "))
        self.assertTrue(expression.endswith(" }}"))
        expression = expression[4:-3]
        # Evaluate the actual workflow expression, not a duplicate routing rule.
        # These operators have the same precedence and truthy-string behavior
        # in Python and Actions. Missing PR properties evaluate to empty strings.
        for event, head, expected in (
            ("pull_request", "arismoko/horizon", "dyna-large"),
            ("pull_request", "contributor/horizon", "ubuntu-latest"),
            ("push", "", "dyna-large"),
            ("workflow_dispatch", "", "dyna-large"),
        ):
            with self.subTest(event=event, head=head):
                values = {
                    "github.event_name": event,
                    "github.event.pull_request.head.repo.full_name": head,
                    "github.repository": "arismoko/horizon",
                }
                evaluated = re.sub(
                    r"github\.[a-z_.]+", lambda match: repr(values[match[0]]), expression
                ).replace("&&", " and ").replace("||", " or ")
                self.assertEqual(eval(evaluated, {"__builtins__": {}}), expected)

    def test_workflows_do_not_classify_pull_requests_by_fork_flag(self):
        for path in sorted(WORKFLOWS.glob("*.y*ml")):
            with self.subTest(workflow=path.name):
                self.assertNotIn("head.repo.fork", path.read_text())

    def test_every_job_on_our_runners_can_go_back_to_github(self):
        count = 0
        for where, _, _, _, value, pools in routed():
            count += 1
            # Both Ubuntu labels are GitHub-hosted fallbacks.
            self.assertRegex(value, r"\|\| 'ubuntu-(?:latest|24\.04)' \}\}$", where)
