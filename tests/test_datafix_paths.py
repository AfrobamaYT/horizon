"""No Mojang data, network, JVM or harness execution is needed for root safety."""
import hashlib
import importlib
import io
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from tools import datafix_paths, pr_ready

# Script consumers import their sibling module directly.
sys.modules['datafix_paths'] = datafix_paths


class DatafixPaths(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.checkout = self.base / 'checkout'
        self.checkout.mkdir()
        self.shared = self.base / 'fixtures'
        self.shared.mkdir()
        self.stack = mock.patch.multiple(datafix_paths, ROOT=self.checkout, READ_ONLY=self.shared)
        self.stack.start()
        self.addCleanup(self.stack.stop)
        env = mock.patch.dict(os.environ, {}, clear=True)
        env.start()
        self.addCleanup(env.stop)

    def test_minecraft_input_does_not_select_output(self):
        (self.checkout / '.local').mkdir()
        (self.checkout / '.local/minecraft').symlink_to(self.shared, target_is_directory=True)
        self.assertEqual(datafix_paths.writable_root(), self.checkout / '.local/datafix')

    def test_symlinked_ancestor_refused(self):
        (self.checkout / '.local').symlink_to(self.shared, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'HORIZON_DATAFIX_ROOT'):
            datafix_paths.writable_root()
        self.assertEqual(list(self.shared.iterdir()), [])

    def test_datafix_symlink_refused(self):
        (self.checkout / '.local').mkdir()
        (self.checkout / '.local/datafix').symlink_to(self.shared, target_is_directory=True)
        with self.assertRaises(ValueError):
            datafix_paths.writable_root()

    def test_explicit_safe_root(self):
        os.environ['HORIZON_DATAFIX_ROOT'] = str(self.base / 'private')
        self.assertEqual(datafix_paths.writable_root(), self.base / 'private')

    def test_preflight_starts_no_subprocess_on_refusal(self):
        os.environ['HORIZON_DATAFIX_ROOT'] = str(self.shared / 'downloads')
        with mock.patch.object(pr_ready.subprocess, 'Popen') as runner:
            with self.assertRaisesRegex(ValueError, 'HORIZON_DATAFIX_ROOT'):
                pr_ready.run_script([pr_ready.Step('Parity', 'echo unexpected')], 'local', False)
            runner.assert_not_called()

    def test_download_and_lock_destinations_are_private(self):
        # Import the actual consumers with the owning boundary selecting a temp root.
        for name in ('datafix_worlds', 'test_datafix_parity'):
            sys.modules.pop(name, None)
            self.addCleanup(sys.modules.pop, name, None)
        worlds = importlib.import_module('datafix_worlds')
        parity = importlib.import_module('test_datafix_parity')
        payload = io.BytesIO()
        with zipfile.ZipFile(payload, 'w') as jar:
            jar.writestr('dummy', 'synthetic jar')
        body = payload.getvalue()
        metadata = {'downloads': {'server': {'url': 'https://invalid.test/jar', 'sha1': hashlib.sha1(body).hexdigest()}}}
        with mock.patch.object(worlds, 'version_meta', return_value=metadata), mock.patch.object(worlds.urllib.request, 'urlopen', return_value=io.BytesIO(body)):
            path, _, _ = worlds.server_jar('1.8.9')
        self.assertEqual(path, self.checkout / '.local/datafix/servers/1.8.9/server.jar')
        self.assertEqual(path.read_bytes(), body)
        self.assertEqual(parity.WORLDS.parent / 'generate.lock', self.checkout / '.local/datafix/generate.lock')
        self.assertEqual(worlds.worlds_dir('1.18.2'), parity.WORLDS)
        self.assertEqual(list(self.shared.iterdir()), [])
