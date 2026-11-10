#!/usr/bin/env python3
"""Require explicit ownership reasons for vocabulary process-global declarations.

The allowlist is a reviewed inventory, not a discovery-generated permission list.
Comments, literals and unconditional unit-test items do not declare production
state. A feature-gated declaration is checked even when its feature is off.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re

from layering.rust_text import blank, item_end

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "crates/horizon-mod-vocab/src"
INVENTORY = ROOT / "tools/layering/mod_vocab_state.json"
STATE = re.compile(r"(?<![\w'])static\s+(?:(?:mut|ref)\s+)?(?P<name>[A-Za-z_]\w*)\s*:(?!:)\s*(?P<type>[^=;]+)=")
TEST = re.compile(r"#\[\s*(?:test\s*|cfg\s*\(\s*test\s*\)\s*)\]")


def declarations(source: str) -> list[tuple[str, str, int]]:
    """Return name, normalized type and line for each non-test static/TLS slot."""
    code = blank(source)
    prefix = re.match(r"\s*(?:#!\[[^\]]*\]\s*)*", code).group()
    if re.search(r"#!\[\s*cfg\s*\(\s*test\s*\)\s*\]", prefix):
        return []
    regions = [(m.start(), item_end(code, m.end())[0]) for m in TEST.finditer(code)]
    functions = [(m.start(), item_end(code, m.start())[0], m[1])
                 for m in re.finditer(r"\bfn\s+(\w+)", code)]
    found = []
    for match in STATE.finditer(code):
        if any(start <= match.start() < stop for start, stop in regions):
            continue
        owners = [name for start, stop, name in functions if start <= match.start() < stop]
        name = '::'.join([*owners, match['name']])
        found.append((name, re.sub(r"\s+", "", match['type']), code.count('\n', 0, match.start()) + 1))
    return found


def violations(files: dict[str, str], allowed: dict[str, dict]) -> list[str]:
    errors = []
    seen = set()
    for path, source in sorted(files.items()):
        for name, kind, line in declarations(source):
            key = f"{path}::{name}"
            entry = allowed.get(key)
            if key in seen:
                errors.append(f"{path}:{line}: duplicate inventory identity {name}; use distinct state names")
            seen.add(key)
            if entry is None:
                errors.append(f"{path}:{line}: unlisted process state {name}: {kind}")
            elif kind != entry['type']:
                errors.append(f"{path}:{line}: {name} changed type: {entry['type']} -> {kind}")
    for key in sorted(allowed.keys() - seen):
        errors.append(f"stale state inventory entry: {key}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=SOURCE)
    parser.add_argument('--inventory', type=Path, default=INVENTORY)
    args = parser.parse_args()
    allowed = json.loads(args.inventory.read_text())
    for name, entry in allowed.items():
        if not entry.get('reason') or not entry.get('category'):
            parser.error(f"{name} needs an ownership category and reason")
    files = {p.relative_to(args.source).as_posix(): p.read_text() for p in args.source.rglob('*.rs')}
    errors = violations(files, allowed)
    for error in errors:
        print(error)
    return bool(errors)


if __name__ == '__main__':
    raise SystemExit(main())
