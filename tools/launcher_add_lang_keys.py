#!/usr/bin/env python3
"""Append source-English values for new keys to every launcher language catalog."""

import argparse
import json
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("keys", nargs="+", help="New source-English strings used by tr/trf")
args = parser.parse_args()

for path in sorted((Path(__file__).resolve().parents[1] / "launcher/lang").glob("*.json")):
    source = path.read_text(encoding="utf-8")
    table = json.loads(source)
    for key in args.keys:
        table.setdefault(key, key)
    updated = json.dumps(table, ensure_ascii=False, indent=2) + "\n"
    if updated != source:
        path.write_text(updated, encoding="utf-8")
