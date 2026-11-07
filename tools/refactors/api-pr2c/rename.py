#!/usr/bin/env python3
"""Audit the forwarder rename before changing any source.

Refuses existing Game methods, including trait impl methods: adding an inherent
method could change method lookup at callers. No rewrite is allowed on failure.
"""
import collections
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
GAME = ROOT / "crates/horizon-client/src"


def mask(source):
    # Preserve offsets and braces in code, but not comments or string literals.
    pattern = r'''//[^\n]*|/\*[\s\S]*?\*/|r(\#*)"[\s\S]*?"\1|'(?:\\(?:u\{[0-9a-fA-F]+\}|x[0-9a-fA-F]{2}|.)|[^'\\])'|"(?:\\.|[^"\\])*"'''
    return re.sub(pattern, lambda m: " " * len(m[0]), source)


def blocks(source, pattern):
    code = mask(source)
    for match in re.finditer(pattern, code):
        start = match.end()
        depth = 1
        end = start
        while depth:
            if code[end] == "{":
                depth += 1
            elif code[end] == "}":
                depth -= 1
            end += 1
        yield match, source[start:end - 1], start


def audit():
    targets = {}
    classes = collections.Counter()
    for path in sorted((GAME / "game/host").glob("*.rs")):
        if path.stem in {"draw", "services"}:
            continue  # These are explicitly outside PR 2c.
        for method, body, _ in blocks(path.read_text(), r'\bfn\s+(\w+)\s*\([^{}]*?\)[^{}]*\{'):
            call = re.search(r'(self\.|Self::)(native_\w+)\(([^;]*)\)\s*$', body)
            if not call:
                classes["accessors"] += 1
                continue
            name = method[1]
            target = call[2]
            assert target not in targets, target
            targets[target] = name
            classes["static" if call[1] == "Self::" else "mechanical" if target == "native_" + name else "odd_named"] += 1
    definitions = collections.defaultdict(list)
    parsed = subprocess.check_output(
        ["cargo", "run", "--quiet", "-p", "horizon-mod-api-gen", "--example", "audit_game_methods"],
        cwd=ROOT, text=True,
    )
    for row in parsed.splitlines():
        name, path, line, kind = row.split("\t")
        if "host" in Path(path).parts:
            continue  # The replaced forwarders themselves are not clashes.
        definitions[name].append((path, int(line), kind))
    print("Base:", subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip())
    print("Forwarder classes:", dict(classes))
    clashes = []
    for old, new in sorted(targets.items()):
        if len(definitions[old]) != 1:
            clashes.append(f"{old} -> {new}: expected one Game definition, found {definitions[old]}")
        for path, line, header in definitions[new]:
            clashes.append(f"{old} -> {new}: {path}:{line}: {header}")
    if clashes:
        print("REFUSED: rename collisions / unresolved definitions:")
        print("\n".join(clashes))
        return 1
    print("No rename clashes.")
    return 0


if __name__ == "__main__":
    raise SystemExit(audit())
