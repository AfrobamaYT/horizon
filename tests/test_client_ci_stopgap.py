"""Exercise the temporary client runner and actual cargo command contract."""
import contextlib
import io
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

from test_ci_concurrency import expression_value, yaml
from tools.ci import ignored_shards, ignored_tests

ROOT = Path(__file__).resolve().parents[1]


class ClientStopgap(unittest.TestCase):
    def test_actual_yaml_routes_only_two_exact_client_selectors(self):
        job = yaml.safe_load((ROOT / '.github/workflows/dyna-ci.yml').read_text())["jobs"]["ignored-tests-shards"]
        expression = job["runs-on"]
        selectors = ignored_shards.expand(ignored_shards.SHARDS) + [
            "horizon-client", "horizon-client#1of1", "horizon-client#1of3",
            "horizon-client#2of3", "horizon-client#3of3", "horizon-client-lib#1of2",
        ]
        for selector in selectors:
            with self.subTest(selector=selector):
                # Reuse the restricted evaluator with its context name mapped to matrix.
                actual = expression_value(expression.removeprefix('${{').removesuffix('}}').replace('matrix.', 'github.'), {"shard": selector}) if expression.startswith('${{') else expression
                expected = "dyna-xlarge" if selector in ("horizon-client#1of2", "horizon-client#2of2") else "dyna-large"
                self.assertEqual(actual, expected)

    def test_wrapper_command_threads_for_canonical_and_direct_selectors(self):
        selectors = ignored_shards.expand(ignored_shards.SHARDS) + [
            "horizon-client", "horizon-client#1of1", "horizon-client#1of3",
            "horizon-client#2of3", "horizon-client#3of3", "horizon-client-lib#1of2",
        ]
        for selector in selectors:
            with self.subTest(selector=selector):
                commands = []

                def execute(command, **kwargs):
                    commands.append(command)
                    return subprocess.CompletedProcess(command, 0, "suite::case: test\n" if '--list' in command else '     Running unittests src/lib.rs (target/example)\nrunning 0 tests\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.1s\n')

                package, number, count = ignored_tests.parse_shard(selector)
                with patch.object(ignored_tests.subprocess, 'run', side_effect=execute), contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(ignored_tests.run(package, ['suite::case'], number, count), {})
                command = commands[-1]
                expected = 8 if selector in ("horizon-client#1of2", "horizon-client#2of2") else 4
                self.assertEqual([arg for arg in command if arg.startswith('--test-threads=')], [f'--test-threads={expected}'])
                self.assertIn('--include-ignored', command)
                self.assertIn('--exact', command)
