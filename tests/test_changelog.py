import copy
from datetime import datetime, timezone
import json
from pathlib import Path
import unittest
from unittest.mock import patch

from tools.changelog import collect, members, merged_prs, render


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

    @patch('tools.changelog.time.sleep')
    @patch('tools.changelog.get')
    def test_pull_collection_is_batched(self, get, sleep):
        node = {'number': 1, 'title': 'Change', 'body': '',
                'mergedAt': '2026-11-13T00:00:00Z', 'headRefName': 'topic',
                'headRefOid': 'a' * 40, 'author': {'login': 'author', '__typename': 'User'},
                'labels': {'nodes': []}}
        get.side_effect = [
            {'data': {'search': {'issueCount': 101, 'nodes': [dict(node, number=n) for n in range(100)],
                                'pageInfo': {'hasNextPage': True, 'endCursor': 'next'}}}},
            {'data': {'search': {'issueCount': 101, 'nodes': [dict(node, number=100)],
                                'pageInfo': {'hasNextPage': False, 'endCursor': 'end'}}}}]
        self.assertEqual(len(merged_prs('2026-11-13')), 101)
        self.assertEqual(get.call_count, 2)
        self.assertEqual(get.call_args.args[1]['variables']['after'], 'next')

    @patch('tools.changelog.search', return_value=[])
    @patch('tools.changelog.merged_prs')
    @patch('tools.changelog.get', return_value=[])
    def test_collect_uses_no_per_pull_rest_requests(self, get, pulls, search):
        pulls.return_value = self.data['prs']
        result = collect(self.now, 1)
        self.assertEqual(len(result['prs']), 2)
        self.assertEqual(get.call_count, 1)  # Releases only; member already batched.
        self.assertIn('/releases?', get.call_args.args[0])

    @patch('tools.changelog.search', return_value=[])
    @patch('tools.changelog.merged_prs', return_value=[])
    @patch('tools.changelog.get')
    def test_release_paging_is_bounded(self, get, pulls, search):
        get.return_value = [{'published_at': '2026-11-13T00:00:00Z'}] * 100
        with self.assertRaisesRegex(RuntimeError, 'five pages'):
            collect(self.now, 1)
        self.assertEqual(get.call_count, 5)

    @patch('tools.changelog.time.sleep')
    @patch('tools.changelog.get')
    def test_daily_search_limit_fails_clearly(self, get, sleep):
        get.return_value = {'data': {'search': {'issueCount': 1001}}}
        with self.assertRaisesRegex(RuntimeError, '1,000 results'):
            merged_prs('2026-11-13')

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
