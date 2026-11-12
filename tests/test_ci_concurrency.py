"""Pin the actual parsed workflow's main queue and PR replacement behavior."""
import ast
from pathlib import Path
import re
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
# Use the same offline parser as the repository guard, not a runner package.
sys.path.insert(0, str(ROOT / "tools/ci/vendor/pyyaml-6.0.3.zip"))
import yaml

WORKFLOWS = [ROOT / ".github/workflows/dyna-ci.yml"]
# Main pushes and dispatch only, one run per commit (owner, 2026-11-12).
PER_COMMIT = [ROOT / ".github/workflows" / name for name in ("wasm-mods.yml", "fn-replace-check.yml")]
MAIN_ONLY = [ROOT / ".github/workflows/fn-replace-check.yml"]


def expression_value(expression, github):
    """Evaluate this workflow's context/equality/boolean subset; reject other syntax.

    These contexts use nonzero PR numbers, lowercase event/ref strings and nonempty
    SHAs, whose truthiness/equality matches Actions. Boolean operators return their
    operands, so the group's fallback chain is exercised rather than pinned as text.
    """
    def evaluate(node):
        if isinstance(node, ast.Constant):
            return node.value
        if isinstance(node, ast.Name):
            return {"github": github, "true": True, "false": False}[node.id]
        if isinstance(node, ast.Attribute):
            return evaluate(node.value).get(node.attr, "")
        if isinstance(node, ast.Compare) and len(node.ops) == 1:
            equal = evaluate(node.left) == evaluate(node.comparators[0])
            if isinstance(node.ops[0], ast.Eq):
                return equal
            if isinstance(node.ops[0], ast.NotEq):
                return not equal
        if isinstance(node, ast.BoolOp):
            for value in node.values:
                result = evaluate(value)
                if isinstance(node.op, ast.Or) and result:
                    return result
                if isinstance(node.op, ast.And) and not result:
                    return result
            return result
        raise AssertionError(f"unsupported Actions expression syntax: {ast.dump(node)}")

    return evaluate(ast.parse(expression.replace("&&", " and ").replace("||", " or ").strip(), mode="eval").body)


def concurrency_for(path, event, sha, number=None):
    workflow = yaml.safe_load(path.read_text())
    policy = workflow["concurrency"]
    github = {"event_name": event, "sha": sha, "ref": "refs/heads/main",
              "event": {"pull_request": {"number": number or ""}}}
    if number is not None:
        github["ref"] = f"refs/pull/{number}/merge"
    group = re.sub(r"\$\{\{(.*?)\}\}",
                   lambda match: str(expression_value(match.group(1), github)), policy["group"])
    cancel = policy["cancel-in-progress"]
    if isinstance(cancel, str):
        match = re.fullmatch(r"\$\{\{(.*?)\}\}", cancel)
        if match is None:
            raise AssertionError("cancellation must be a boolean or an Actions expression")
        cancel = expression_value(match.group(1), github)
    if not isinstance(cancel, bool):
        raise AssertionError("cancellation did not evaluate to a boolean")
    return group, cancel


class ConcurrencyTests(unittest.TestCase):
    def test_main_pushes_share_one_non_cancelling_group(self):
        for path in WORKFLOWS:
            with self.subTest(workflow=path.name):
                first = concurrency_for(path, "push", "a" * 40)
                next_push = concurrency_for(path, "push", "b" * 40)
                self.assertEqual(first[0], next_push[0], "main pushes must queue behind the running group")
                self.assertIs(first[1], False, "a main push must not cancel the running run")
                self.assertIs(next_push[1], False)

    def test_pr_updates_cancel_only_the_same_pr_group(self):
        for path in WORKFLOWS:
            with self.subTest(workflow=path.name):
                first = concurrency_for(path, "pull_request", "a" * 40, 101)
                update = concurrency_for(path, "pull_request", "b" * 40, 101)
                other_pr = concurrency_for(path, "pull_request", "c" * 40, 102)
                main = concurrency_for(path, "push", "d" * 40)
                self.assertEqual(first[0], update[0], "updates must replace checks for the same PR")
                self.assertNotEqual(first[0], other_pr[0], "one PR must not cancel another PR")
                self.assertNotEqual(first[0], main[0], "a PR must not cancel main")
                for _, cancel in (first, update, other_pr):
                    self.assertIs(cancel, True, "PR events must still cancel their previous checks")


    def test_main_only_checks_never_run_on_pull_requests(self):
        for path in MAIN_ONLY:
            with self.subTest(workflow=path.name):
                triggers = yaml.load(path.read_text(), Loader=yaml.BaseLoader)["on"]
                self.assertEqual(set(triggers), {"push", "workflow_dispatch"})
                self.assertEqual(triggers["push"], {"branches": ["main"]})

    def test_wasm_stack_prs_run_only_the_hosted_matrix(self):
        workflow = yaml.load((ROOT / ".github/workflows/wasm-mods.yml").read_text(), Loader=yaml.BaseLoader)
        paths = set(workflow["on"]["pull_request"]["paths"])
        self.assertTrue({"crates/horizon-threads/**", "crates/horizon-guest-stack/**",
                         "crates/horizon-client/**", "packaging/**"} <= paths)
        fixture = workflow["jobs"]["fixture"]
        for event, expected in [("pull_request", False), ("push", True), ("workflow_dispatch", True)]:
            self.assertIs(expression_value(fixture["if"], {"event_name": event}), expected)
        guest = workflow["jobs"]["guest-stack"]
        self.assertTrue(guest["name"])
        self.assertEqual(set(guest["strategy"]["matrix"]["os"]),
                         {"ubuntu-latest", "windows-latest", "macos-latest"})
        self.assertEqual(guest["runs-on"], "${{ matrix.os }}")

    def test_main_only_checks_run_once_per_commit(self):
        for path in PER_COMMIT:
            with self.subTest(workflow=path.name):
                first = concurrency_for(path, "push", "a" * 40)
                next_push = concurrency_for(path, "push", "b" * 40)
                dispatch = concurrency_for(path, "workflow_dispatch", "c" * 40)
                self.assertNotEqual(first[0], next_push[0], "a newer main push must not replace a queued run")
                self.assertEqual(first[0], concurrency_for(path, "workflow_dispatch", "a" * 40)[0],
                                 "a dispatch of the same commit queues behind its run")
                for _, cancel in (first, next_push, dispatch):
                    self.assertIs(cancel, False, "no run of these checks is cancelled")


if __name__ == "__main__":
    unittest.main()
