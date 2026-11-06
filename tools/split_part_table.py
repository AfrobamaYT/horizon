#!/usr/bin/env python3
"""One-shot #1459 move; --verify BASE checks exact items, order and moved lines.

Declared edits: repeated trait/impl/module envelopes, one generated include, the
shared support comment, and rustfmt whitespace. No body or signature substitutions.
"""
import argparse
from collections import Counter, defaultdict
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
SOURCE = Path('crates/horizon-mod-api/src/part_table.rs')
DIRECTORY = SOURCE.with_suffix('')
INCLUDE = 'include!(concat!(env!("OUT_DIR"), "/part_table_generated.rs"));\n'
COMMENT = '//! Shared table state, cross-kind registration and common test fixtures only.\n'
# Original fixture span boundaries: whole syn items, never bodies, move together.
TEST_AREAS = [(2009, 'game_rules'), (2128, 'shared'), (2205, 'blocks'),
              (2218, 'entities'), (2256, 'fluids'), (2321, 'items'),
              (2372, 'blocks'), (2465, 'effects'), (2538, 'block_entities'),
              (2617, 'shared'), (2638, 'block_entities'), (2658, 'shared'),
              (2680, 'structures'), (2714, 'blocks'), (2742, 'block_entities'),
              (2816, 'shared'), (2837, 'blocks'), (2847, 'shared'),
              (2981, 'items'), (3866, 'shared'), (3883, 'capabilities'),
              (3988, 'entities'), (4087, 'capabilities'), (4110, 'shared'),
              (4158, 'blocks'), (4621, 'capabilities'), (4707, 'items'),
              (5598, 'tickets'), (5621, 'key_mappings'), (5640, 'tickets'),
              (5657, 'items'), (5676, 'capabilities'), (5941, 'items'),
              (5981, 'capabilities'), (6042, 'shared'), (6062, 'items'),
              (6088, 'shared'), (6097, 'fluids'), (6503, 'energy'),
              (6676, 'blocks'), (6694, 'block_entities'), (6757, 'capabilities'),
              (6767, 'blocks')]


def run(*args, source=None):
    return subprocess.check_output(args, cwd=ROOT, input=source, text=True)


def formatted(source):
    return run('rustfmt', '--edition', '2024', '--config', 'skip_children=true',
               '--emit', 'stdout', source=source)


def area(text):
    declaration = re.search(r'^(?:(?:pub(?:\([^)]*\))? )?(?:fn|trait|struct|enum|type) |impl[ <])[^\n]+', text, re.M)
    name = re.search(r'\bfn (\w+)', text) if declaration is None or re.match(r'(?:pub(?:\([^)]*\))? )?fn ', declaration[0]) else None
    name = name[1] if name else declaration[0] if declaration else text
    for pattern, kind in [
        (r'game_rule', 'game_rules'), (r'key_mapping', 'key_mappings'),
        (r'ticket|controller', 'tickets'), (r'structure', 'structures'),
        (r'place_provider|PlaceProviderPart', 'items'),
        (r'block_entity|BlockEntity|BlockCapabilityProvider|BlockProvider', 'block_entities'),
        (r'entity|Entity', 'entities'), (r'item|ItemKeyError|ItemCapabilityProvider|ItemProvider', 'items'),
        (r'fluid', 'fluids'), (r'effect', 'effects'), (r'block|BlockProvider', 'blocks'),
        (r'menu', 'menus'),
        (r'capability|Capability|Capabilities|Provider|providers|provided|^next$|^answered$|Answer', 'capabilities')]:
        if re.search(pattern, name):
            return kind
    return 'shared'


def split(source, layout):
    lines = source.splitlines(keepends=True)
    outputs = defaultdict(str)
    first = next(i['start'] - 1 for i in layout if i['start'] > 25)
    residual = ''.join(lines[:first]) + '\n' + INCLUDE
    cursor = first
    for item in layout:
        if item['end'] <= first:
            continue
        start, end = item['start'] - 1, item['end']
        prefix = ''.join(lines[cursor:start])
        cursor = end
        text = ''.join(lines[start:end])
        if 'children' in item and 'pub mod tests {' in text:
            members = item['children']
            groups = defaultdict(str)
            previous = next(n for n in range(start, end) if lines[n].endswith('pub mod tests {\n')) + 1
            header = ''.join(lines[start:previous])
            for member in members:
                declaration = next((n + 1 for n in range(member['start'] - 1, member['end'])
                                    if re.match(r'    (?:pub )?(?:fn|struct|impl|enum|trait|mod) ', lines[n])), member['start'])
                kind = next(k for line, k in reversed(TEST_AREAS) if declaration >= line) if declaration >= 2009 else 'shared'
                groups[kind] += ''.join(lines[previous:member['end']])
                previous = member['end']
            groups['shared'] += ''.join(lines[previous:end - 1])
            for kind, body in groups.items():
                outputs[kind] += prefix + header + body + '}\n'
        elif ('members' in item and re.search(r'^(?:pub trait (?:Registrar|CapabilityProviders)\b|impl (?:dyn Registrar|Registrar for PartRegistrar|PartRegistrar|HookParts|Parts|CapabilityProviders for InstalledProviders)\b)', text, re.M)):
            members = item['members']
            previous = members[0]['start'] - 1
            while not lines[previous - 1].rstrip().endswith('{'):
                previous -= 1
            header = ''.join(lines[start:previous])
            groups = defaultdict(str)
            for member in members:
                kind = area(member['text'])
                groups[kind] += ''.join(lines[previous:member['end']])
                previous = member['end']
            groups['shared'] += ''.join(lines[previous:end - 1])
            for kind, body in groups.items():
                if body.strip():
                    outputs[kind] += prefix + header + body + '}\n'
        else:
            # Cross-kind types and helpers retain the original shared table scope.
            kind = area(text)
            if re.search(r'^(?:pub(?:\([^)]*\))? )?(?:struct|enum|fn|mod) (?:Installed|Parts|HookParts|PartRegistrar|register_all|install_registrar|resolve_parts|unknown_keys|add_tagged|attached)\b', text, re.M):
                kind = 'shared'
            outputs[kind] += prefix + text
    outputs['shared'] = COMMENT + outputs['shared']
    return formatted(residual), {kind + '.rs': formatted(text) for kind, text in outputs.items()}


def payload(source, layout):
    lines = source.splitlines()
    excluded = set()
    def walk(items):
        for item in items:
            members = item.get('members', item.get('children'))
            if members:
                first = members[0]['start'] - 1
                while first > item['start'] and not lines[first - 1].rstrip().endswith('{'):
                    first -= 1
                excluded.update(range(item['start'] - 1, first))
                excluded.add(item['end'] - 1)
                if 'children' in item:
                    walk(item['children'])
    walk(layout)
    return Counter(line.strip() for n, line in enumerate(lines) if n not in excluded
                   and line.strip() and line != COMMENT.rstrip())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--verify', metavar='BASE')
    parser.add_argument('--base', metavar='BASE')
    args = parser.parse_args()
    run('cargo', 'build', '-q', '-p', 'horizon-mod-api-gen', '--bin', 'part-table-layout')
    metadata = json.loads(run('cargo', 'metadata', '--no-deps', '--format-version', '1'))
    binary = Path(metadata['target_directory']) / 'debug' / 'part-table-layout'
    layout = lambda source: json.loads(run(str(binary), source=source))
    base = args.verify or args.base
    source = run('git', 'show', f'{base}:{SOURCE}') if base else (ROOT / SOURCE).read_text()
    residual, files = split(source, layout(source))
    if args.verify:
        actual = {p.name: p.read_text() for p in (ROOT / DIRECTORY).glob('*.rs')}
        assert actual == files, 'item contents, envelopes or within-kind order differ'
        assert (ROOT / SOURCE).read_text() == residual, 'root drift'
        before = payload(source, layout(source))
        after = payload(residual, layout(residual))
        after.subtract(Counter([INCLUDE.strip()]))
        for text in files.values():
            after.update(payload(text, layout(text)))
        assert +before == +after, f'moved lines differ: removed {before - after}; added {after - before}'
        print(f'Moved payload line multisets equal: {sum(before.values())} nonblank lines')
        print('Exact formatted items, attributes, signatures, bodies and within-kind order verified')
    else:
        (ROOT / DIRECTORY).mkdir(exist_ok=True)
        (ROOT / SOURCE).write_text(residual)
        for name, text in files.items():
            (ROOT / DIRECTORY / name).write_text(text)
    for name, text in sorted(files.items()):
        fixtures = sum(i['end'] - i['start'] + 1 for i in layout(text)
                       if 'children' in i and 'pub mod tests {' in '\n'.join(text.splitlines()[i['start'] - 1:i['end']]))
        print(f'{name}: {len(text.splitlines())} lines; {fixtures} fixture-module lines')


if __name__ == '__main__':
    main()
