"""Ignored-test CI must distinguish completed targets from child fixture output."""
import contextlib
import io
import subprocess
import unittest
from unittest.mock import patch

from tools.ci import ignored_tests


def target(count, results, summary):
    return f"     Running unittests src/lib.rs (target/debug/deps/example)\nrunning {count} tests\n{results}{summary}"


def summary(passed=0, failed=0, ignored=0):
    verdict = "FAILED" if failed else "ok"
    return (f"test result: {verdict}. {passed} passed; {failed} failed; "
            f"{ignored} ignored; 0 measured; 12 filtered out; finished in 0.1s\n")


class IgnoredTests(unittest.TestCase):
    def run_output(self, output, code=0):
        with patch.object(ignored_tests.subprocess, "run", return_value=subprocess.CompletedProcess([], code, output)), contextlib.redirect_stdout(io.StringIO()) as log:
            result = ignored_tests.run("example", [])
        return result, log.getvalue()

    def test_success_and_multiple_targets(self):
        output = target(1, "test first ... ok\n", summary(passed=1))
        output += target(2, "test second ... ok\ntest third ... ok\n", summary(passed=2))
        result, log = self.run_output(output)
        self.assertEqual(result, {"first": True, "second": True, "third": True})
        self.assertIn("target 2 started 2 tests", log)
        self.assertIn("cargo exit 0", log)

    def test_known_failure_is_still_available_to_policy(self):
        result, _ = self.run_output(target(2, "test good ... ok\ntest bad ... FAILED\n", summary(passed=1, failed=1)), 101)
        self.assertEqual(result, {"good": True, "bad": False})
        with patch.object(ignored_tests, "known_failures", return_value={("example", "bad"): None}), patch.object(ignored_tests, "run", return_value=result), contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(ignored_tests.main(["example"]), 0)

    def test_plain_known_failure_that_passes_requires_removal(self):
        result, _ = self.run_output(target(1, "test repaired ... ok\n", summary(passed=1)))
        with patch.object(ignored_tests, "known_failures", return_value={("example", "repaired"): None}), patch.object(ignored_tests, "run", return_value=result), contextlib.redirect_stdout(io.StringIO()) as log:
            self.assertEqual(ignored_tests.main(["example"]), 1)
        self.assertIn("repaired: passes now; take it off", log.getvalue())

    def test_unexplained_nonzero_status(self):
        result, _ = self.run_output(target(1, "test good ... ok\n", summary(passed=1)), 101)
        self.assertIsNone(result)

    def test_missing_summary_even_after_parsed_failure(self):
        result, _ = self.run_output(target(2, "test bad ... FAILED\n", ""), 101)
        self.assertIsNone(result)

    def test_child_summary_cannot_hide_aborted_parent(self):
        output = target(2401, "running 1 test\ntest child ... ok\n", summary(passed=1))
        result, _ = self.run_output(output, 101)
        self.assertIsNone(result)

    def test_small_summary_rejected_even_on_zero_exit(self):
        result, _ = self.run_output(target(2401, "test child ... ok\n", summary(passed=1)))
        self.assertIsNone(result)

    def test_missing_first_summary_not_hidden_by_next_target(self):
        output = target(10, "test bad ... FAILED\n", "")
        output += target(1, "test child ... ok\n", summary(passed=1))
        result, _ = self.run_output(output, 101)
        self.assertIsNone(result)

    def test_target_without_count_and_build_failure(self):
        for output in ["     Running unittests src/lib.rs (target/example)\n", "error: build failed\n"]:
            result, _ = self.run_output(output, 101)
            self.assertIsNone(result)

    def test_signal_diagnostic_does_not_echo_adjacent_output(self):
        output = target(2, "", "") + "error: process failed (signal: 9, private fixture data)\n"
        result, log = self.run_output(output, 101)
        self.assertIsNone(result)
        self.assertIn("(signal: 9)", log)
        self.assertNotIn("private fixture data", log)

    def test_ignored_tests_count_toward_completion(self):
        result, _ = self.run_output(target(2, "test good ... ok\ntest skipped ... ignored\n", summary(passed=1, ignored=1)))
        self.assertEqual(result, {"good": True})


def leg_names(count=2):
    """Test names that each leg of `count` owns, found by the same hash the runner uses."""
    owned = {number: [] for number in range(1, count + 1)}
    index = 0
    while any(len(names) < 2 for names in owned.values()):
        name = f"suite::test_{index}"
        for number in owned:
            if ignored_tests.in_leg(name, number, count):
                owned[number].append(name)
        index += 1
    return owned


class Legs(unittest.TestCase):
    def test_shard_names(self):
        self.assertEqual(ignored_tests.parse_shard("horizon-client#2of3"), ("horizon-client", 2, 3))
        self.assertEqual(ignored_tests.parse_shard("horizon-client-lib"), ("horizon-client-lib", None, None))
        with self.assertRaises(SystemExit):
            ignored_tests.parse_shard("horizon-client#3of2")

    def test_every_test_is_in_exactly_one_leg(self):
        for index in range(500):
            name = f"module::test_{index}"
            self.assertEqual(sum(ignored_tests.in_leg(name, number, 3) for number in (1, 2, 3)), 1)
        self.assertTrue(ignored_tests.in_leg("anything", None, None))

    def test_a_leg_skips_every_test_another_leg_owns(self):
        owned = leg_names()
        listing = "".join(f"{name}: test\n" for names in owned.values() for name in names) + "a_bench: benchmark\n"
        output = target(2, "".join(f"test {name} ... ok\n" for name in owned[1]), summary(passed=2))
        calls = []

        def fake(command, **_):
            calls.append(command)
            if "--list" in command:
                return subprocess.CompletedProcess(command, 0, listing)
            return subprocess.CompletedProcess(command, 0, output)

        with patch.object(ignored_tests.subprocess, "run", side_effect=fake), contextlib.redirect_stdout(io.StringIO()):
            result = ignored_tests.run("example", ["marked"], 1, 2)
        self.assertEqual(set(result), set(owned[1]))
        command = calls[1]
        skipped = {command[i + 1] for i, arg in enumerate(command) if arg == "--skip"}
        self.assertEqual(skipped, {"marked", *owned[2]})

    def test_a_leg_that_cannot_list_its_tests_fails(self):
        def fake(command, **_):
            return subprocess.CompletedProcess(command, 101, "")

        with patch.object(ignored_tests.subprocess, "run", side_effect=fake), contextlib.redirect_stdout(io.StringIO()):
            self.assertIsNone(ignored_tests.run("example", [], 1, 2))

    def judge(self, known, results, leg=1):
        with patch.object(ignored_tests, "known_failures", return_value=known), patch.object(ignored_tests, "run", return_value=results), contextlib.redirect_stdout(io.StringIO()) as log:
            code = ignored_tests.main([f"example#{leg}of2"])
        return code, log.getvalue()

    def test_an_entry_in_the_other_leg_is_not_judged_here(self):
        mine, other = leg_names()[1][0], leg_names()[2][0]
        code, log = self.judge({("example", other): None}, {mine: True})
        self.assertEqual(code, 0, log)

    def test_an_entry_in_this_leg_that_passes_or_vanished_still_fails(self):
        mine = leg_names()[1]
        code, log = self.judge({("example", mine[0]): None}, {mine[0]: True})
        self.assertEqual(code, 1)
        self.assertIn("passes now", log)
        code, log = self.judge({("example", mine[1]): None}, {mine[0]: True})
        self.assertEqual(code, 1)
        self.assertIn("no longer runs", log)

    def test_a_new_failure_and_an_incomplete_leg_fail(self):
        mine = leg_names()[1][0]
        code, log = self.judge({}, {mine: False})
        self.assertEqual(code, 1)
        self.assertIn("isn't a known failure", log)
        code, log = self.judge({}, None)
        self.assertEqual(code, 1)
        self.assertIn("example#1of2: the test run did not complete", log)


if __name__ == "__main__":
    unittest.main()
