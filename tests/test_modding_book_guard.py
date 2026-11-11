"""Exercise the workflow's shell guard without building the game or book."""
import os
from pathlib import Path
import subprocess
import tempfile
import sys
import unittest


ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools/ci/vendor/pyyaml-6.0.3.zip'))
import yaml

WORKFLOW = ROOT / '.github/workflows/modding-book.yml'


def guard_script():
    """Read the executable guard from the parsed workflow, not text delimiters."""
    steps = yaml.safe_load(WORKFLOW.read_text())['jobs']['book']['steps']
    return next(step['run'] for step in steps
                if step.get('name') == 'Reject private paths in published docs')


class BookGuardTests(unittest.TestCase):
    def test_parsed_workflow_keeps_scan_on_prs_and_builds_off_prs(self):
        """Pin the event gates themselves so shell tests cannot hide a skipped scan."""
        steps = yaml.safe_load(WORKFLOW.read_text())['jobs']['book']['steps']
        scan = next(step for step in steps
                    if step.get('name') == 'Reject private paths in published docs')
        present = "steps.book.outputs.present == 'true'"
        self.assertEqual(scan['if'], present)
        self.assertEqual(scan['env']['BOOK_EVENT'], '${{ github.event_name }}')
        for command in ['RUSTDOCFLAGS="-D warnings" cargo doc -p horizon-mod-api --no-deps',
                        'cargo test -p horizon-modding-examples']:
            step = next(step for step in steps if step.get('run') == command)
            self.assertEqual(step['if'], present + " && github.event_name != 'pull_request'")

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

    @unittest.skipUnless(os.name == 'posix', 'requires Unix file permissions')
    def test_unreadable_file_fails_with_real_unprivileged_grep(self):
        """Exercise a real permission error, dropping privileges when run as root."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            root.chmod(0o755)
            for path in ['docs/modding/src', 'target/modding-book']:
                (root / path).mkdir(parents=True)
            unreadable = root / 'docs/modding/src/unreadable.md'
            unreadable.write_text('public docs')
            unreadable.chmod(0)
            # A normal test runner is already unprivileged; root must drop to nobody.
            credentials = {'user': 65534, 'group': 65534, 'extra_groups': []} if os.geteuid() == 0 else {}
            try:
                result = subprocess.run(
                    ['bash', '-e', '-o', 'pipefail', '-c', guard_script()],
                    cwd=root, env={**os.environ, 'BOOK_EVENT': 'pull_request', 'LC_ALL': 'C'},
                    capture_output=True, text=True, **credentials,
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('Permission denied', result.stderr)
                self.assertIn('unreadable.md', result.stderr)
            finally:
                unreadable.chmod(0o600)

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
