#!/usr/bin/env python3
"""Restore retained API point translations from the option C base."""
import json
from pathlib import Path
import subprocess
source = '\n'.join(p.read_text() for p in Path('launcher/src').rglob('*.rs'))
for path in Path('launcher/lang').glob('*.json'):
    base = json.loads(subprocess.check_output(['git', 'show', f'90db1b460b:{path}'], text=True))
    data = json.loads(path.read_text())
    for key, value in base.items():
        if key not in data and key in source:
            data[key] = value
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2, sort_keys=True) + '\n')
