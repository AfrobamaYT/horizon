"""Synthetic failure payloads only; no game or prepared-data execution."""
import contextlib
import io
import unittest
from unittest.mock import patch

from tools.ci import ignored_tests as ci


class Budgets(unittest.TestCase):
    def render(self, output):
        with contextlib.redirect_stdout(io.StringIO()) as log:
            ci.failure_diagnostics(output)
        return log.getvalue()

    def test_120_failures_all_have_identity_and_first_safe_context(self):
        output = ''.join(f"test suite::case_{i} ... FAILED\n"
                         f"thread 'suite::case_{i}' panicked at .local/private.rs:1:1:\n"
                         "assertion `left == right` failed: POISON\n left: 96\n right: 93\n"
                         for i in range(120))
        log = self.render(output)
        for i in range(120):
            self.assertIn(f'failed test: suite::case_{i}\n', log)
        self.assertEqual(log.count('assertion equality failed'), 120)
        self.assertEqual(log.count('assertion left: 96'), 120)
        self.assertIn('120 shown; 0 failing tests not shown', log)
        self.assertNotIn('POISON', log)
        self.assertNotIn('.local', log)

    def test_total_budget_counts_occurrences_including_duplicate_names(self):
        with patch.object(ci, 'FAILURE_TOTAL_BYTES', 200):
            log = self.render('test repeated ... FAILED\n' * 120)
        payload, metadata = log.split('failure diagnostics: ')
        shown = payload.count('failed test:')
        self.assertLessEqual(len(payload.encode()), 200)
        self.assertIn(f'{shown} shown; {120 - shown} failing tests not shown', metadata)
        self.assertIn('total byte budget 200', metadata)

    def test_multibyte_identity_and_per_test_budget(self):
        log = self.render('test ' + '界' * 400 + ' ... FAILED\n')
        payload = log.split('failure diagnostics: ')[0]
        self.assertLessEqual(len(payload.encode()), 500)
        self.assertIn('shortened: per-test byte budget', payload)

    def test_missing_and_poisoned_context_are_explicit(self):
        log = self.render("test absent ... FAILED\ntest missing ... FAILED\n"
                          "thread 'missing' panicked at .local/private.rs:1:1:\n"
                          "test poisoned ... FAILED\n"
                          "thread 'poisoned' panicked at .local/private.rs:1:1:\n"
                          "secret token POISON\n")
        self.assertIn('panic context missing', log)
        self.assertIn('panic reason missing', log)
        self.assertIn('custom panic; payload withheld', log)
        self.assertNotIn('POISON', log)
