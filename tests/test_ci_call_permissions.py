"""A called workflow may not ask for a permission its caller job does not grant.

GitHub checks this when it expands the call, before any job runs, whatever a nested job's `if:` says, so a
mismatch fails the whole run at startup (dry runs included) and cannot be seen before it reaches main."""
import unittest

from tests.test_ci_concurrency import ROOT, yaml

WORKFLOWS = ROOT / ".github/workflows"
LEVELS = {"none": 0, "read": 1, "write": 2}
SCOPES = ("actions", "attestations", "checks", "contents", "deployments", "discussions", "id-token", "issues",
          "models", "packages", "pages", "pull-requests", "repository-projects", "security-events", "statuses")


def permissions(value):
    """A permissions block as {scope: level}; `read-all`/`write-all` expand to every scope."""
    if value in ("read-all", "write-all"):
        return {scope: value.split("-")[0] for scope in SCOPES}
    return dict(value or {})


def granted(workflow, job):
    """What a caller job grants: its own block, else its workflow's (an absent block is every default, unknown
    here, so it grants nothing this test can rely on)."""
    return permissions(job.get("permissions", workflow.get("permissions", {})))


def requests(path, seen=()):
    """Every (job, scope, level) a called workflow's jobs ask for, nested calls included."""
    workflow = yaml.safe_load(path.read_text())
    top = workflow.get("permissions")
    asked = []
    for name, job in workflow["jobs"].items():
        if "uses" in job:
            nested = WORKFLOWS / job["uses"].rsplit("/", 1)[1]
            if nested not in seen:
                asked += [(f"{path.name}:{name} -> {job_name}", scope, level)
                          for job_name, scope, level in requests(nested, seen + (path,))]
            continue
        for scope, level in permissions(job.get("permissions", top)).items():
            asked.append((f"{path.name}:{name}", scope, level))
    return asked


def mismatches(caller_path):
    workflow = yaml.safe_load(caller_path.read_text())
    problems = []
    for name, job in workflow["jobs"].items():
        if not str(job.get("uses", "")).startswith("./.github/workflows/"):
            continue
        grant = granted(workflow, job)
        for where, scope, level in requests(WORKFLOWS / job["uses"].rsplit("/", 1)[1]):
            if LEVELS[level] > LEVELS[grant.get(scope, "none")]:
                problems.append(f"{caller_path.name}:{name} grants {scope}: {grant.get(scope, 'none')}, "
                                f"but {where} asks for {scope}: {level}")
    return problems


class CallPermissionTests(unittest.TestCase):
    def test_every_call_grants_what_its_workflow_asks_for(self):
        callers = [path for path in sorted(WORKFLOWS.glob("*.yml")) if "uses: ./.github/workflows/" in path.read_text()]
        self.assertIn(WORKFLOWS / "daily.yml", callers)
        for path in callers:
            with self.subTest(workflow=path.name):
                self.assertEqual(mismatches(path), [])

    def test_a_write_asked_by_a_never_running_job_is_still_a_mismatch(self):
        import tempfile
        from pathlib import Path
        with tempfile.TemporaryDirectory() as folder:
            called = Path(folder) / "called.yml"
            called.write_text("on: workflow_call\npermissions:\n  contents: read\njobs:\n"
                              "  publish:\n    if: false\n    permissions:\n      contents: write\n"
                              "    runs-on: ubuntu-latest\n    steps: [{run: 'true'}]\n")
            caller = Path(folder) / "caller.yml"
            caller.write_text("on: workflow_dispatch\npermissions:\n  contents: read\njobs:\n"
                              "  leg:\n    uses: ./.github/workflows/called.yml\n")
            global WORKFLOWS
            saved, WORKFLOWS = WORKFLOWS, Path(folder)
            try:
                self.assertEqual(mismatches(caller), ["caller.yml:leg grants contents: read, but called.yml:publish "
                                                      "asks for contents: write"])
            finally:
                WORKFLOWS = saved


if __name__ == "__main__":
    unittest.main()
