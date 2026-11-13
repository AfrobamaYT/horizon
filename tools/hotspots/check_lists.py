#!/usr/bin/env python3
"""A `pub use` under crates/horizon-mod-vocab/src/exports/ names at most 12 items in braces.

rustfmt re-wraps a brace list on every added name, so two PRs that each add a name to one long list always
conflict (tools/hotspots/README.md). Export a module's items with a glob, or put the new names in a fragment of
their own. A list's length is the number of names in the whole `pub use` (any visibility), nested braces included.

The lists that are longer today are in LONG_LISTS with their current length; an entry may only shrink. The check
fails when a listed file's longest list is longer than its entry, and when it is shorter, so the entry is lowered
in the same PR (or removed once the file is at 12 or under). The files in SWITCHED are replaced as a whole by a
planned PR and are not checked.

    python3 tools/hotspots/check_lists.py
"""
import argparse
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
EXPORTS = "crates/horizon-mod-vocab/src/exports"
LIMIT = 12

# file (under EXPORTS): (longest list today, why it stays)
LONG_LISTS = {
    "built.rs": (17, "no planned PR removes it"),
    "game.rs": (24, "no planned PR removes it"),
    "payload.rs": (17, "no planned PR removes it"),
    "payload_codec.rs": (13, "no planned PR removes it"),
    "phase.rs": (15, "no planned PR removes it"),
    "registry_keys.rs": (15, "rename list"),
    "structure_type.rs": (14, "no planned PR removes it"),
}

# Files a planned switch replaces as a whole (design of the conflict hotspots, PR plan).
SWITCHED = {
    "domains.rs": "PR F turns its type lists into one glob per domain",
    "part_table.rs": "PR G splits it into one fragment per parts file",
}

USE = re.compile(r"\bpub(?:\s*\([^)]*\))?\s+use\s+([^;]*);")
COMMENT = re.compile(r"//[^\n]*|/\*.*?\*/", re.S)


def longest_list(source):
    """Names in the longest braced `pub use` of a Rust source (0 when none has braces)."""
    best = 0
    for m in USE.finditer(COMMENT.sub("", source)):
        tree = m.group(1)
        if "{" not in tree:
            continue
        names = [x for x in re.split(r"[{},]", tree) if x.strip() and not x.strip().endswith("::")]
        best = max(best, len(names))
    return best


def problems(root):
    found = []
    folder = Path(root) / EXPORTS
    for path in sorted(folder.rglob("*.rs")):
        name = path.relative_to(folder).as_posix()
        if name in SWITCHED:
            continue
        n = longest_list(path.read_text(encoding="utf-8"))
        entry = LONG_LISTS.get(name)
        shown = f"{EXPORTS}/{name}"
        if entry is None:
            if n > LIMIT:
                found.append(f"{shown}: a `pub use` names {n} items in braces, more than {LIMIT}. Two PRs that each "
                             "add a name to a long list conflict when rustfmt re-wraps it: export the module with a "
                             "glob, or put the new names in a `pub use` of their own.")
        elif n > entry[0]:
            found.append(f"{shown}: its longest `pub use` list grew to {n} names (at most {entry[0]}, and it may only "
                         "shrink). Put the new names in a `pub use` of their own.")
        elif n < entry[0]:
            what = (f"remove its entry from LONG_LISTS" if n <= LIMIT else f"lower its entry in LONG_LISTS to {n}")
            found.append(f"{shown}: its longest `pub use` list is down to {n} names: {what} in tools/hotspots/check_lists.py.")
    for name in sorted(LONG_LISTS):
        if not (folder / name).exists():
            found.append(f"{EXPORTS}/{name} is gone: remove its entry from LONG_LISTS in tools/hotspots/check_lists.py.")
    return found


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    ap.add_argument("--root", default=str(ROOT))
    args = ap.parse_args(argv)
    found = problems(args.root)
    for line in found:
        print(f"error: {line}", file=sys.stderr)
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
