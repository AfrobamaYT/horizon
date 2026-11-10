"""Packaging comparisons against independent pre-change git object fixtures.

Fixtures are verbatim da81e38705 objects, not outputs of the staging function.
"""
import hashlib
import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('stage', ROOT / 'tools/stage_launcher_packaging.py')
STAGE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(STAGE)
FIXTURES = ROOT / 'tests/fixtures/launcher-upstream'

class PackagingIdentity(unittest.TestCase):
    def test_compiled_namespace_metadata_preserves_upstream_and_isolates_fork(self):
        expected = json.loads((FIXTURES / 'identity.json').read_text())
        with tempfile.TemporaryDirectory() as temporary:
            temporary = Path(temporary)
            # Compile the real identity module, not a Python model of its selection.
            program = '#[allow(dead_code)] #[path = ' + json.dumps(str(ROOT / 'launcher/src/identity.rs')) + '] mod identity;\nfn main() {\n'
            program += ''.join('println!("{}\\t{}", "' + name + '", identity::' + name + ');\n' for name in expected)
            program += '}\n'
            source = temporary / 'metadata.rs'
            source.write_text(program)
            outputs = []
            for flags in ([], ['--cfg', 'horizon_fork_launcher']):
                binary = temporary / 'metadata'
                subprocess.run(['rustc', '--edition=2021', str(source), '-o', str(binary), *flags], check=True)
                output = subprocess.check_output([str(binary)], text=True)
                outputs.append(dict(line.split('\t', 1) for line in output.splitlines()))
            self.assertEqual(outputs[0], expected)
            self.assertTrue(all(outputs[0][name] != outputs[1][name] for name in expected))
            self.assertEqual(outputs[1]['KEYCHAIN_SERVICE'], 'io.github.arismoko.horizonfork.launcher')
            self.assertEqual(outputs[1]['LAUNCHER_FOLDER'], 'horizon-fork-arismoko/launcher')
            self.assertEqual(outputs[1]['GAME_FOLDER'], 'horizon-fork-arismoko/game')

    def test_upstream_metadata_is_byte_identical_to_base_objects(self):
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / 'packaging'
            STAGE.stage(ROOT / 'packaging', destination, STAGE.UPSTREAM)
            for fixture in FIXTURES.glob('packaging_*'):
                relative = fixture.name.removeprefix('packaging_').replace('_', '/')
                self.assertEqual((destination / relative).read_bytes(), fixture.read_bytes(), relative)
            for name, digest in json.loads((FIXTURES / 'icon-sha256.json').read_text()).items():
                self.assertEqual(hashlib.sha256((destination / name.removeprefix('packaging/')).read_bytes()).hexdigest(), digest, name)

    def test_fork_staging_does_not_modify_source_and_has_distinct_bundle(self):
        source = ROOT / 'packaging'
        before = {p.relative_to(source): p.read_bytes() for p in source.rglob('*') if p.is_file()}
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / 'packaging'
            STAGE.stage(source, destination, STAGE.FORK)
            plist = (destination / 'macos/Info.plist').read_text()
            self.assertIn('<string>io.github.arismoko.horizonfork.launcher</string>', plist)
            self.assertIn('<string>horizon-fork-launcher</string>', plist)
            self.assertNotIn('<string>Horizon</string>', plist)
            desktop = (destination / 'linux/horizon-fork-launcher.desktop').read_text()
            self.assertIn('Name=Horizon Fork Launcher (arismoko)\n', desktop)
            self.assertIn('Exec=horizon-fork-launcher\n', desktop)
            self.assertIn('StartupWMClass=io.github.arismoko.horizonfork.launcher\n', desktop)
            for script in ('linux/appimage.sh', 'macos/package.sh'):
                subprocess.run(['bash', '-n', str(destination / script)], check=True)
            self.assertNotEqual((destination / 'icons/horizon.ico').read_bytes(), (source / 'icons/horizon.ico').read_bytes())
        self.assertEqual(before, {p.relative_to(source): p.read_bytes() for p in source.rglob('*') if p.is_file()})

if __name__ == '__main__':
    unittest.main()
