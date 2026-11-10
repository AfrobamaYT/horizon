#!/usr/bin/env python3
"""Retarget source consumers after split_mod_vocabulary.py (not mod-facing paths)."""
from pathlib import Path

root = Path(__file__).resolve().parents[2]
paths = list((root / 'crates/horizon-mod-api-gen/src').rglob('*.rs'))
paths += list((root / 'tools').glob('*.py'))
paths += [path for path in (root / 'tests').glob('*.py') if path.name != 'test_mod_api_wire.py']
paths += list((root / 'docs').rglob('*.md'))
paths += [root / name for name in (
    'tools/addresses.json', 'tools/api_scaffold.toml', 'tools/api_scaffold.md',
)]
for path in paths:
    source = path.read_text()
    moved = source.replace('crates/horizon-mod-api/src', 'crates/horizon-mod-vocab/src')
    if path.name == 'lib.rs':
        moved = moved.replace('("mod_api::", "horizon-mod-api")', '("mod_api::", "horizon-mod-vocab")')
    if source != moved:
        path.write_text(moved)
