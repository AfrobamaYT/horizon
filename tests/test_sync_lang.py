import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('merge_lang', Path(__file__).resolve().parents[1] / 'tools/sync/merge_lang.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class TranslationMergeTests(unittest.TestCase):
    def test_upstream_change(self):
        self.assertEqual(module.merge({'a': 'old'}, {'a': 'old'}, {'a': 'new'}), ({'a': 'new'}, []))

    def test_fork_change_and_addition(self):
        self.assertEqual(module.merge({'a': 'old'}, {'a': 'fork', 'b': 'added'}, {'a': 'old'}), ({'a': 'fork', 'b': 'added'}, []))

    def test_deletion(self):
        self.assertEqual(module.merge({'a': 'old'}, {}, {'a': 'old'}), ({}, []))

    def test_conflict(self):
        self.assertEqual(module.merge({'a': 'old'}, {'a': 'fork'}, {'a': 'new'}), ({'a': 'fork'}, ['a']))

    def test_order(self):
        self.assertEqual(list(module.merge({}, {'z': 'last', 'a': 'first'}, {})[0]), ['a', 'z'])
