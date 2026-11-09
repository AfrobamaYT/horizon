#!/usr/bin/env python3
"""Apply audited path policies only to unmerged paths; print remaining areas."""
import fnmatch
from pathlib import Path
import subprocess
import sys
from collections import Counter


def git(*args):
    return subprocess.check_output(['git', *args]).decode()


def patterns(name):
    return [line for line in Path('tools/sync/' + name).read_text().splitlines()
            if line and not line.startswith('#')]


def area(path):
    if '/lang/' in path:
        return 'translations'
    if path.startswith(('launcher/', 'crates/horizon-client-render/')):
        return 'renderlauncher'
    if path.startswith('crates/horizon-client/src/game/'):
        return 'game'
    if path.startswith('crates/horizon-client/'):
        return 'clientroot'
    if path.startswith('crates/'):
        return 'crates'
    return 'toolsdocs'


def main(target):
    removed = patterns('fork-removed.txt')
    upstream = patterns('upstream-owned.txt')
    generated = [line.split('\t')[0] for line in patterns('generated.txt')]
    for path in git('diff', '--name-only', '--diff-filter=U').splitlines():
        if any(fnmatch.fnmatchcase(path, p) for p in removed):
            # Only remove when absent in our pre-merge tree, never suppress edits.
            exists = subprocess.run(['git', 'cat-file', '-e', 'HEAD:' + path], capture_output=True).returncode == 0
            if not exists:
                subprocess.run(['git', 'rm', '-f', '--', path], check=True)
                print('Kept fork removal:', path)
        elif path in generated or any(fnmatch.fnmatchcase(path, p) for p in upstream):
            subprocess.run(['git', 'checkout', target, '--', path], check=True)
            print('Took upstream (regenerate if listed):', path)
    remaining = git('diff', '--name-only', '--diff-filter=U').splitlines()
    for group, count in sorted(Counter(map(area, remaining)).items()):
        print(f'{group}: {count}')
    print(f'Remaining REAL conflicts: {len(remaining)}')


if __name__ == '__main__':
    main(sys.argv[1])
