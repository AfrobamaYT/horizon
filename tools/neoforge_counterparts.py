#!/usr/bin/env python3
"""Writes docs/modding/src/reference/neoforge-counterparts.md from the `neoforge = ...` keys of the
hook declarations in crates/horizon-mod-api/src/hooks/. Run it from the repository root after a hook's
counterpart changes; `--check` exits 1 if the page is out of date."""
import glob
import os
import re
import sys

HOOKS = "crates/horizon-mod-api/src/hooks/*.rs"
PAGE = "docs/modding/src/reference/neoforge-counterparts.md"


def hooks():
    rows = []
    for path in sorted(glob.glob(HOOKS)):
        if os.path.basename(path) == "mod.rs":
            continue
        text = open(path, encoding="utf8").read()
        for m in re.finditer(r"#\[hook\((.*?)\)\]\s*(?:fn\s+(\w+))", text, re.S):
            attr, name = m.group(1), m.group(2)

            def key(k):
                v = re.search(k + r'\s*=\s*"([^"]*)"', attr)
                return v.group(1) if v else None

            neo = key("neoforge")
            if neo is None:
                lst = re.search(r"neoforge\s*=\s*\[(.*?)\]", attr, re.S)
                if lst:
                    neo = ", ".join(re.findall(r'"([^"]*)"', lst.group(1)))
            rows.append((name, key("side") or "", neo))
    return sorted(rows)


def table(rows, head):
    out = [f"| Hook | Side | {head} |", "| --- | --- | --- |"]
    out += [f"| `{n}` | {s} | {c} |" for n, s, c in rows]
    return "\n".join(out)


def page():
    rows = hooks()
    mapped = [(n, s, f"`{c}`") for n, s, c in rows if c and not c.lower().startswith("none")]
    none = [
        (n, s, re.sub(r"^none\s*\(?(.*?)\)?$", r"\1", c, flags=re.S).strip() or "none")
        for n, s, c in rows
        if c and c.lower().startswith("none")
    ]
    unset = [(n, s, "not set") for n, s, c in rows if not c]
    return f"""# NeoForge counterparts

Each hook's `neoforge = ...` key in `crates/horizon-mod-api/src/hooks/` names the NeoForge 26.3 event or method it
matches, since a counterpart's name and contract are the rule for a hook. This page lists the keys as declared.
It is written by `tools/neoforge_counterparts.py` (run it again after changing a key; `--check` tells if the page is
stale). A key names the event class (`PlayerEvent.Clone`) or the extension method (`IPlayerExtension.openMenu`);
notes about cancelling or the event bus belong in the hook's `///` doc, not in the key.

{len(rows)} hooks: {len(mapped)} name a counterpart, {len(none)} say they have none, {len(unset)} have no key yet.
The names were checked for form, not yet against the NeoForge 26.3 Javadoc; the third table is the part of that
check still to do ([#1029](https://github.com/arismoko/horizon/issues/1029)).

## Hooks with a counterpart

{table(mapped, "NeoForge")}

## Hooks that declare no counterpart

A hook with no NeoForge counterpart is judged on its merits like any other. The note is the key's own reason.

{table(none, "Reason")}

## Hooks with no key yet

These declare no `neoforge` key. Each needs either its NeoForge name or a `none` with the reason.

{table(unset, "NeoForge")}
"""


if __name__ == "__main__":
    text = page()
    if "--check" in sys.argv:
        sys.exit(0 if os.path.exists(PAGE) and open(PAGE, encoding="utf8").read() == text else 1)
    open(PAGE, "w", encoding="utf8", newline="\n").write(text)
