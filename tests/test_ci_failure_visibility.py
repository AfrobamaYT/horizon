"""Synthetic diagnostics only: never use prepared resource output as a fixture."""
import contextlib
import io
import subprocess
import unittest
from unittest.mock import patch

from tests.test_ci_concurrency import yaml, ROOT
from tests.test_ignored_test_completion import IgnoredTests, target, summary
from tools.ci import ignored_tests


class Visibility(unittest.TestCase):
    def test_panic_category_and_identity_without_operands(self):
        output = target(1, "test suite::broken ... FAILED\n"
                        "thread 'suite::broken' panicked at src/lib.rs:12:3:\n"
                        "assertion `left == right` failed: PRIVATE_RESOURCE\n"
                        " left: PRIVATE_RESOURCE\n", summary(failed=1))
        result, log = IgnoredTests().run_output(output, 101)
        self.assertEqual(result, {"suite::broken": False})
        self.assertIn("failed test: suite::broken", log)
        self.assertIn("assertion equality failed", log)
        self.assertNotIn("PRIVATE_RESOURCE", log)

    def test_numeric_boolean_and_private_operands(self):
        for left, right, expected in (("96", "93", "assertion left: 96"),
                                      ("true", "false", "assertion right: false"),
                                      ('"PRIVATE"', '[PRIVATE]', "operand withheld")):
            output = target(1, "test broken ... FAILED\n"
                            "thread 'broken' panicked at src/lib.rs:12:3:\n"
                            f"assertion `left == right` failed\n left: {left}\n right: {right}\n",
                            summary(failed=1))
            _, log = IgnoredTests().run_output(output, 101)
            self.assertIn(expected, log)
            self.assertNotIn("PRIVATE", log)

    def test_list_failure_preserves_compiler_headings_not_source(self):
        process = subprocess.CompletedProcess([], 101, "", "error[E0308]: mismatched types\n"
                    " --> src/lib.rs:12:3\nerror: expected `SECRET`, found `OTHER`\n"
                    " PRIVATE_RESOURCE\n")
        with patch.object(ignored_tests.subprocess, "run", return_value=process) as run, contextlib.redirect_stdout(io.StringIO()) as log:
            self.assertIsNone(ignored_tests.run("example", [], 1, 2))
        self.assertEqual(run.call_count, 1)
        self.assertIn("mismatched types", log.getvalue())
        self.assertIn("src/lib.rs:12:3", log.getvalue())
        self.assertNotIn("SECRET", log.getvalue())
        self.assertNotIn("PRIVATE_RESOURCE", log.getvalue())

    def test_bounded_and_unrecognized_diagnostics(self):
        with contextlib.redirect_stdout(io.StringIO()) as log:
            ignored_tests.compile_diagnostics("\n".join(["error: " + "x" * 400] * 30))
            ignored_tests.compile_diagnostics("private unexpected payload")
        self.assertIn("truncated", log.getvalue())
        self.assertIn("no recognized compiler diagnostic", log.getvalue())
        self.assertLess(len(log.getvalue()), 7500)
        self.assertNotIn("private unexpected payload", log.getvalue())

    def test_formatting_failure_does_not_skip_later_builds_but_is_propagated(self):
        steps = yaml.safe_load((ROOT / '.github/workflows/dyna-ci.yml').read_text())["jobs"]["lint"]["steps"]
        fmt = next(s for s in steps if s.get("id") == "formatting")
        native = next(s for s in steps if s.get("id") == "native-lint")
        final = next(s for s in steps if s.get("name") == "Propagate formatting failure")
        self.assertTrue(fmt["continue-on-error"])
        self.assertNotIn("clippy", fmt["run"])
        self.assertIn("clippy", native["run"])
        self.assertNotIn("continue-on-error", native)
        for name in ("Lint native targets", "Lint the server build without sound or controllers", "The machine sample builds against this API", "The mod API builds for wasm32"):
            step = next(s for s in steps if s.get("name") == name)
            self.assertLess(steps.index(fmt), steps.index(step))
            self.assertLess(steps.index(step), steps.index(final))
            self.assertNotIn("formatting", step.get("if", ""))
        self.assertIn("always()", final["if"])
        self.assertIn("steps.formatting.outcome == 'failure'", final["if"])
        self.assertIn("exit 1", final["run"])
        # Actions treats continue-on-error failure as a successful conclusion for
        # subsequent default success() steps, but retains the original outcome.
        for format_outcome in ("success", "failure"):
            conclusion = "success" if fmt["continue-on-error"] else format_outcome
            later_builds_run = conclusion == "success"
            final_runs = format_outcome == "failure"  # parsed always()/outcome predicate above
            self.assertTrue(later_builds_run)
            self.assertEqual(final_runs, format_outcome == "failure")
