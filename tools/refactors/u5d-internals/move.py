#!/usr/bin/env python3
"""Replay the atomic internals-owner move and verify its exact declared edits.

The JSON operation list is the reviewable transformation: each replacement has
an exact old/new value and occurrence count. No source is discovered or guessed.
Unchanged relocated lines cancel in the multiset; only the declared edits may
change its balance. New focused tests are listed separately and counted as
hand-written work, never hidden inside the equivalence exemption.
"""
import argparse
from collections import Counter
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent


def source(base, path):
    return subprocess.check_output(
        ["git", "show", f"{base}:{path}"], cwd=ROOT, text=True
    )


def lines(text):
    return Counter(line for line in text.splitlines() if line.strip())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apply", action="store_true")
    args = parser.parse_args()
    spec = json.loads((HERE / "edits.json").read_text())
    removed, added = Counter(), Counter()
    expected_removed, expected_added = Counter(), Counter()
    for path, operations in spec["files"].items():
        before = source(spec["base"], path)
        expected = before
        for operation in operations:
            old, new, count = (operation[key] for key in ("old", "new", "count"))
            actual_count = expected.count(old)
            if actual_count != count:
                raise SystemExit(f"{path}: expected {count} occurrences, found {actual_count}: {old!r}")
            expected = expected.replace(old, new)
        if args.apply:
            (ROOT / path).write_text(expected)
        actual = (ROOT / path).read_text()
        if actual != expected:
            raise SystemExit(f"{path}: differs from the scripted transformation")
        # Whole-file line counters also account for surrounding-line joins in
        # the explicitly declared replacements; pure relocation cancels out.
        expected_removed.update(lines(before) - lines(expected))
        expected_added.update(lines(expected) - lines(before))
        removed.update(lines(before) - lines(actual))
        added.update(lines(actual) - lines(before))
    if removed + expected_added != added + expected_removed:
        raise SystemExit("removed/added line multisets differ beyond declared edits")
    print(f"Verified {len(spec['files'])} files against {spec['base']}")
    print("Removed lines + declared additions = added lines + declared removals")


if __name__ == "__main__":
    main()
