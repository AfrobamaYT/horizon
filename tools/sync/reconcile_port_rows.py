#!/usr/bin/env python3
"""Merge sync 27 ledger cells, retaining fork relocations on overlapping edits."""
import pathlib
import subprocess
import re

symbols = {}
for file in pathlib.Path('crates').glob('**/*.rs'):
    if 'target' in file.parts:
        continue
    for name in re.findall(r'\bfn\s+(\w+)', file.read_text()):
        symbols.setdefault(name, set()).add(file)

BASE = '7b42678bcd'
FORK = '600782248b'
UPSTREAM = '501446443c'

def rows(ref, path):
    data = subprocess.check_output(['git', 'show', f'{ref}:{path}'], text=True)
    return {line.split('\t')[0]: line.split('\t') for line in data.splitlines()}

for path in pathlib.Path('tools/port_map').glob('*.tsv'):
    base, fork, upstream = (rows(ref, path) for ref in (BASE, FORK, UPSTREAM))
    current = {line.split('\t')[0]: line.split('\t') for line in path.read_text().splitlines()}
    for key, incoming in upstream.items():
        old = base.get(key)
        if incoming == old:
            continue
        own = fork.get(key)
        if own is None or own == old:
            current[key] = incoming
        elif old is not None and len(old) == len(own) == len(incoming):
            current[key] = [u if f == b else f for b, f, u in zip(old, own, incoming)]
    # Resolve moved source files by unique basename, never invent a target.
    for row in current.values():
        if len(row) < 3 or not row[2]:
            continue
        source, sep, symbol = row[2].partition('::')
        if source.endswith('.rs') and not pathlib.Path(source).exists():
            matches = list(pathlib.Path('crates').glob('**/' + pathlib.Path(source).name))
            if len(matches) == 1:
                row[2] = str(matches[0]) + sep + symbol
            else:
                candidates = symbols.get(symbol.split('::')[-1], set())
                if len(candidates) == 1:
                    row[2] = str(next(iter(candidates))) + sep + symbol
                elif row[0] in upstream and upstream[row[0]] != base.get(row[0]):
                    print(f'Unresolved incoming path: {path}: {row[0]}: {row[2]}')
    path.write_text('\n'.join('\t'.join(row).rstrip() for row in current.values()) + '\n')
