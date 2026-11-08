"""Synthetic trace comparisons: independent observations, no game resources required."""
import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
import paper_parity as parity
from check_public_tree import prohibited


class PaperParityTests(unittest.TestCase):
    def test_plugin_is_public_but_not_its_binaries_or_neighboring_sources(self):
        prefix = 'tools/fixtures/paper-trace-plugin/'
        self.assertFalse(prohibited(prefix + 'PaperTrace.java'))
        for name in ('Other.java', 'PaperTrace.class', 'paper.jar'):
            self.assertTrue(prohibited(prefix + name))

    def test_trace_window_and_normalization(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parents[1]) as tmp:
            out = Path(tmp)
            def row(name, fields=None):
                return dict(tick=17, name=name, fields=fields or {}, cancelled=False)
            def write(side, rows):
                (out / f'{side}.jsonl').write_text('\n'.join(map(json.dumps, rows)))
            paper = row('ChunkLoadEvent', dict(world='minecraft:overworld', x=-2, z=4, new=True))
            horizon = row('chunk_load', dict(dimension='minecraft:overworld', chunk=[-2, 4], new=True))
            before = row('ChunkLoadEvent', dict(world='minecraft:overworld', x=99, z=99, new=True))
            warmup = row('chunk_load', dict(dimension='minecraft:overworld', chunk=[99, 99], new=True))
            after = row('ChunkLoadEvent', dict(world='minecraft:overworld', x=100, z=100, new=True))
            write('paper', [before, row('_join'), paper, paper, row('_quit'), after])
            write('horizon', [row('_join'), warmup, horizon, row('_quit')])
            parity.lab.write_json(out / 'run.json', dict(complete=True, scenario={'hooks': ['chunk_load']}))
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(parity.report(out), 0)
                horizon['fields']['new'] = False
                write('horizon', [row('_join'), warmup, horizon, row('_quit')])
                self.assertEqual(parity.report(out), 1)
                write('horizon', [row('_join'), row('_quit')])
                self.assertEqual(parity.report(out), 1)
                write('horizon', [horizon])
                self.assertEqual(parity.report(out), 1)
