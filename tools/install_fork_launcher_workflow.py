#!/usr/bin/env python3
"""Replace the disabled launcher workflow with the reviewed dry-run-only template."""
from pathlib import Path
root = Path(__file__).resolve().parents[1]
(root / '.github/workflows/launcher.yml').write_bytes(
    (root / 'packaging/fork-launcher-workflow.yml').read_bytes())
