#!/usr/bin/env python3
"""Move the mod vocabulary source files verbatim (#2534).

Run from the repository root. --verify compares moved files with the source
commit recorded below, independently of Git's rename similarity heuristic.
Boundary adaptations belong in later commits, not in this script.
"""
import argparse
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[2]
OLD = pathlib.Path('crates/horizon-mod-api/src')
NEW = pathlib.Path('crates/horizon-mod-vocab/src')
BASE = 'e96c76c0b9'
STAY = {
    'wire/wire_guest.rs', 'wire/wire_resources.rs',
    'world/phase.rs', 'world/reload.rs', 'world/server_reload.rs',
    'client/client_reload.rs', 'content/registry_keys.rs',
    'testing.rs', 'parts/shared.rs',
}


def files():
    names = subprocess.check_output(
        ['git', 'ls-tree', '-r', '--name-only', BASE, str(OLD)], cwd=ROOT,
        text=True,
    ).splitlines()
    return [pathlib.Path(name).relative_to(OLD) for name in names
            if pathlib.Path(name).relative_to(OLD).as_posix() not in STAY
            and not pathlib.Path(name).relative_to(OLD).as_posix().startswith('testing/')]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--verify', action='store_true')
    args = parser.parse_args()
    paths = files()
    for path in paths:
        destination = ROOT / NEW / path
        if args.verify:
            expected = subprocess.check_output(['git', 'show', f'{BASE}:{OLD / path}'], cwd=ROOT)
            if destination.read_bytes() != expected:
                raise SystemExit(f'non-verbatim move: {path}')
        else:
            destination.parent.mkdir(parents=True, exist_ok=True)
            subprocess.run(['git', 'mv', str(OLD / path), str(NEW / path)], cwd=ROOT, check=True)
    print(f'{len(paths)} verbatim moves {"verified" if args.verify else "performed"}')


if __name__ == '__main__':
    main()
