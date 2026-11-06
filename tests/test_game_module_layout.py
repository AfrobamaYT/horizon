"""Machine-consumed per-file game module metadata and layout invariants."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
GAME = ROOT / 'crates/horizon-client/src/game'


class GameModuleLayoutTests(unittest.TestCase):
    def test_module_roots_and_local_metadata(self):
        for path in GAME.glob('*.rs'):
            self.assertEqual(path.name, 'native_hooks.rs')
        public = {'cushions': 'pub(crate)', 'mods': 'pub(crate)',
                  'native_block_entity_draw': 'pub', 'native_client_services': 'pub'}
        roots = {}
        for marker in GAME.glob('*/.game-area'):
            files = [(p.stem, p) for p in marker.parent.glob('*.rs')]
            files += [(p.parent.name, p) for p in marker.parent.glob('*/mod.rs')]
            for name, path in files:
                self.assertNotIn(name, roots)
                roots[name] = path
        for name, visibility in public.items():
            self.assertIn(f'// game-module-visibility: {visibility}', roots[name].read_text().splitlines())
        for name, path in roots.items():
            lines = path.read_text().splitlines()
            metadata = [line for line in lines if line.startswith('// game-module-')]
            allowed = {'// game-module-visibility: pub', '// game-module-visibility: pub(crate)',
                       '// game-module-cfg: always'}
            for line in metadata:
                self.assertIn(line, allowed, path)
            keys = [line.split(':')[0] for line in metadata]
            self.assertEqual(len(keys), len(set(keys)), path)
            if name == 'game_tests':
                self.assertIn('// game-module-cfg: always', metadata)
        declarations = re.findall(r'^(?:pub(?:\([^)]*\))? )?mod (\w+);',
                                  GAME.with_suffix('.rs').read_text(), re.M)
        self.assertEqual(declarations, ['native_hooks'])
        self.assertIn('include!(concat!(env!("OUT_DIR"), "/game_modules.rs"));',
                      GAME.with_suffix('.rs').read_text())
