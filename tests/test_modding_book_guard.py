"""Exercise the workflow's shell guard without building the game or book."""
import os
from pathlib import Path
import subprocess
import tempfile
import textwrap
import unittest


WORKFLOW = Path(__file__).resolve().parents[1] / '.github/workflows/modding-book.yml'


def guard_script():
    step = WORKFLOW.read_text().split('      - name: Reject private paths in published docs\n', 1)[1]
    script = step.split('        run: |\n', 1)[1].split('      - ', 1)[0]
    return textwrap.dedent(script)


class BookGuardTests(unittest.TestCase):
    def run_guard(self, event, *, token=None, missing=None):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            paths = ['docs/modding/src', 'target/modding-book']
            if event != 'pull_request':
                paths.append('target/doc/mod_api')
            for path in paths:
                if path != missing:
                    folder = root / path
                    folder.mkdir(parents=True)
                    (folder / 'page').write_text('__private' if path == token else 'public docs')
            result = subprocess.run(
                ['bash', '-e', '-o', 'pipefail', '-c', guard_script()],
                cwd=root, env={**os.environ, 'BOOK_EVENT': event},
                capture_output=True, text=True,
            )
            return result.returncode

    def test_pr_scans_source_and_html_without_rustdoc(self):
        self.assertEqual(self.run_guard('pull_request'), 0)
        for path in ['docs/modding/src', 'target/modding-book']:
            with self.subTest(path=path):
                self.assertNotEqual(self.run_guard('pull_request', token=path), 0)
                self.assertNotEqual(self.run_guard('pull_request', missing=path), 0)

    def test_non_pr_events_require_and_scan_all_outputs(self):
        for event in ['push', 'workflow_dispatch', 'schedule']:
            self.assertEqual(self.run_guard(event), 0)
            for path in ['docs/modding/src', 'target/modding-book', 'target/doc/mod_api']:
                with self.subTest(event=event, path=path):
                    self.assertNotEqual(self.run_guard(event, token=path), 0)
                    self.assertNotEqual(self.run_guard(event, missing=path), 0)

    def test_grep_errors_do_not_pass(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for path in ['docs/modding/src', 'target/modding-book']:
                (root / path).mkdir(parents=True)
            # Simulate grep's documented error status, independently of user permissions.
            bin_dir = root / 'bin'
            bin_dir.mkdir()
            grep = bin_dir / 'grep'
            grep.write_text('#!/bin/sh\nexit 2\n')
            grep.chmod(0o755)
            result = subprocess.run(
                ['bash', '-e', '-o', 'pipefail', '-c', guard_script()],
                cwd=root, env={**os.environ, 'BOOK_EVENT': 'pull_request',
                               'PATH': str(bin_dir) + os.pathsep + os.environ['PATH']},
                capture_output=True, text=True,
            )
            self.assertNotEqual(result.returncode, 0)
