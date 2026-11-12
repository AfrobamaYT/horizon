import copy
from datetime import datetime, timezone
import json
from pathlib import Path
import unittest

from tools.changelog import members, render


class ChangelogTests(unittest.TestCase):
    def setUp(self):
        self.data = json.loads((Path(__file__).parent / 'fixtures/changelog.json').read_text())
        self.now = datetime(2026, 11, 13, 12, tzinfo=timezone.utc)

    def test_rollup_expands_deduplicates_and_groups_at_landing(self):
        self.assertEqual(members(self.data['prs'][1]), [1])
        output = render(self.data, self.now)
        self.assertEqual(output.count('(#1)'), 1)
        self.assertNotIn('(#2)', output)
        self.assertIn('## 2026-11-13', output)
        self.assertNotIn('## 2026-11-12', output)
        self.assertIn('### Fixed bugs', output)
        self.assertIn('&#60;script&#62;', output)
        self.assertNotIn('@all', output)

    def test_window_and_cap(self):
        self.data['prs'] = []
        issue = self.data['issues'][0]
        self.data['issues'] += [dict(copy.deepcopy(issue), number=4)]
        output = render(self.data, self.now, cap=1)
        self.assertIn('and 1 more', output)
        self.now = datetime(2026, 12, 14, tzinfo=timezone.utc)
        self.assertIn('No matching activity.', render(self.data, self.now))

    def test_bot_filter_and_label_override(self):
        self.data['prs'] = [self.data['prs'][0]]
        pr = self.data['prs'][0]
        pr['user']['type'] = 'Bot'
        self.assertNotIn('(#1)', render(self.data, self.now))
        pr['labels'] = [{'name': 'bug'}]
        self.assertIn('(#1)', render(self.data, self.now))
        pr['labels'] = [{'name': 'skip-release'}]
        self.assertNotIn('(#1)', render(self.data, self.now))


if __name__ == '__main__':
    unittest.main()
