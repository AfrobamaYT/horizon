#!/usr/bin/env python3
"""What a change does to the mod API's WebAssembly interface: the functions it adds, changes or
removes, by interface, as Markdown for the job summary. A removed or changed import refuses every
mod.wasm that uses it; an added one breaks none. Only reports: it never fails a pull request.

    wit_report.py BASE.wit HEAD.wit
"""
import re
import sys
from pathlib import Path


def functions(path):
    """{interface/function: signature} of a generated mod-api.wit."""
    found, interface = {}, None
    for line in Path(path).read_text(encoding="utf-8").splitlines():
        start = re.match(r"interface ([\w-]+) \{", line)
        if start:
            interface = start.group(1)
            continue
        if line.startswith("}"):
            interface = None
            continue
        func = re.match(r"\s+%?([\w-]+): func(.*);", line)
        if interface and func:
            found[f"{interface}/{func.group(1)}"] = func.group(2).strip()
    return found


def main():
    base, head = functions(sys.argv[1]), functions(sys.argv[2])
    added = sorted(set(head) - set(base))
    removed = sorted(set(base) - set(head))
    changed = sorted(n for n in set(base) & set(head) if base[n] != head[n])
    print("### The mod API's wasm interface\n")
    if not (added or removed or changed):
        print("No import or export changes.")
        return
    for title, names, note in [
        ("Removed", removed, "mods that use these are refused"),
        ("Changed", changed, "mods that use these are refused until rebuilt"),
        ("Added", added, "no mod is affected"),
    ]:
        if names:
            print(f"**{title}** ({len(names)}, {note}):\n")
            for name in names:
                print(f"- `{name}`")
            print()


if __name__ == "__main__":
    main()
