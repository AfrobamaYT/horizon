import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("wire_goldens", ROOT / "tools/generate_wire_enum_goldens.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class WireEnumDiscoveryTests(unittest.TestCase):
    def test_payloads_comments_attributes_and_non_wire_enums(self):
        source = '''
        enum Plain { /// ignored punctuation (not a payload)
            #[doc = "{ fields } (tuple)"] First = 7, Second = 9 }
        enum Payload { Unit, Tuple(u32), Record { value: String } }
        enum Other { First }
        impl ::mod_api::wire::Wire for Plain { fn layout() { runtime_before_or_const_after() } }
        impl ::mod_api::wire::Wire for Payload {}
        '''
        self.assertEqual(MODULE.discover(source), {"Plain"})

    def test_duplicate_leaf_identity_refuses_instead_of_dropping(self):
        with self.assertRaisesRegex(ValueError, "ambiguous"):
            MODULE.discover('''
                mod one { enum Same { A } impl ::mod_api::wire::Wire for Same {} }
                mod two { enum Same { B } impl ::mod_api::wire::Wire for Same {} }
            ''')


if __name__ == "__main__":
    unittest.main()
