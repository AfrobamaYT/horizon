#!/usr/bin/env python3
"""Retarget Cargo executable builds without changing engine library selections."""
import re
from pathlib import Path
root = Path(__file__).resolve().parents[2]
# Root documentation also contains developer build commands.
for p in [root / 'AGENTS.md']:
    s = p.read_text()
    p.write_text(re.sub(r'-p horizon-client(?![\w-])', '-p horizon-game', s))
for base in ['tools', '.github', 'docs', 'packaging', 'xtask']:
    for p in (root / base).rglob('*'):
        if not p.is_file() or 'refactors' in p.parts:
            continue
        try:
            s = p.read_text()
        except UnicodeError:
            continue
        # Python argument lists containing an explicit binary selection.
        def rewrite(m):
            text = m.group()
            if re.search(r"['\"]build['\"]", text) and '--example' not in text and '--lib' not in text:
                text = re.sub(r"(['\"]-p['\"]\s*,\s*['\"])horizon-client(['\"])", r'\1horizon-game\2', text)
            return text
        new = re.sub(r'\[[^\[\]]*\]', rewrite, s)
        # Shell build lines; retain library checks/tests.
        new = re.sub(r'(?m)^(  -p )horizon-client(?![\w-])', r'\1horizon-game', new)
        new = '\n'.join(re.sub(r'-p horizon-client(?![\w-])', '-p horizon-game', line) if 'cargo build' in line and '--example' not in line and '--lib' not in line else line for line in new.split('\n'))
        if new != s:
            p.write_text(new)
