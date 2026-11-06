#!/usr/bin/env python3
"""Move the legacy source-header history once; --verify reproduces it from Git."""

import argparse
from collections import Counter
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
SOURCE = Path("crates/horizon-client/src/game/native_hooks/mod.rs")
ARCHIVE = Path("docs/modding/src/reference/native-api-history.md")
OVERVIEW = """//! Native hook implementations, split by catalog area.
//!
//! Hook documentation and contract versions live in the catalog, not a shared changelog.
//! The numbered history is frozen in the modding book's legacy API history reference.
//!
"""
ARCHIVE_INTRO = """# Legacy native hook API history

This is a frozen archive of the old numbered source-header changelog, preserved
when native hook implementations were split by catalog area. It is not extended
per hook, and its numbers are not compatibility or minimum-game-version promises.
Some historical names are no longer part of the current API.

Current hook documentation and per-hook contract versions belong to the catalog
and the [mod API reference](api.md). Native mods build against an exact game revision.

"""


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--base", help="move the history from this Git revision")
    mode.add_argument("--verify", help="check the exact move from this Git revision")
    args = parser.parse_args()
    base = args.verify or args.base
    source = subprocess.check_output(
        ["git", "show", f"{base}:{SOURCE.as_posix()}"], cwd=ROOT, text=True
    )
    start = re.search(r"^//! API \d+ ", source, re.MULTILINE)
    if start is None:
        parser.error(f"{base}:{SOURCE}: no legacy API history")
    end = source.index("//! Native mods are built against an exact game revision,")
    history = source[start.start():end]
    payload = "\n".join(
        line.removeprefix("//!").removeprefix(" ") for line in history.splitlines()
    ).rstrip() + "\n"
    expected = {
        SOURCE: OVERVIEW + source[end:],
        ARCHIVE: ARCHIVE_INTRO + payload,
    }
    for path, content in expected.items():
        path = ROOT / path
        if args.verify:
            if path.read_text(encoding="utf-8") != content:
                parser.error(f"{path}: differs from the scripted move")
        else:
            path.write_text(content, encoding="utf-8")
    archived = (ROOT / ARCHIVE).read_text(encoding="utf-8").removeprefix(ARCHIVE_INTRO)
    old_lines = [line for line in payload.splitlines() if line.strip()]
    new_lines = [line for line in archived.splitlines() if line.strip()]
    assert Counter(old_lines) == Counter(new_lines)
    assert old_lines == new_lines
    assert (
        (ROOT / SOURCE).read_text(encoding="utf-8").split("\n\n", 1)[1]
        == source.split("\n\n", 1)[1]
    )
    entries = len(re.findall(r"^//! API [0-9]+ ", history, re.MULTILINE))
    print(f"Archived history: {entries} entries")
    print(f"Moved text line multisets and order: equal ({len(old_lines)} nonblank lines)")
    print("Declared edits: strip //! prefixes; archive heading/note; module overview")
    print("Executable Rust source after the module docs: byte-identical")
    print("Exact archive and source output: identical to the scripted move")


if __name__ == "__main__":
    main()
