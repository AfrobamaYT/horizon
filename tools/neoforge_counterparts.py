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
    return f"""# NeoForge counterparts

Each hook's `neoforge = ...` key in `crates/horizon-mod-api/src/hooks/` names the NeoForge 26.3 event or method it
matches, since a counterpart's name and contract are the rule for a hook. This page lists the keys as declared.
It is written by `tools/neoforge_counterparts.py` (run it again after changing a key; `--check` tells if the page is
stale). A key names the event class (`PlayerEvent.Clone`) or the extension method (`IPlayerExtension.openMenu`);
notes about cancelling or the event bus belong in the hook's `///` doc, not in the key.

{len(rows)} hooks: {len(mapped)} name a counterpart, {len(none)} say they have none.
The names were checked against the NeoForge 26.3.0.101 sources (`neoforge-26.3.0.101-sources.jar` on
maven.neoforged.net): each class or extension method named below exists there. A `none` says why no event or method
matches; the keys naming FML classes (`FMLClientSetupEvent`, `ModConfigEvent`, `ModContainer`) or a vanilla one
(`MapDecorationType`) are not in that jar and were not checked, and `screen_transparent_background` still carries its
own note.

## Hooks with a counterpart

{table(mapped, "NeoForge")}

## Hooks that declare no counterpart

A hook with no NeoForge counterpart is judged on its merits like any other. The note is the key's own reason.

{table(none, "Reason")}
"""


if __name__ == "__main__":
    text = page()
    if "--check" in sys.argv:
        sys.exit(0 if os.path.exists(PAGE) and open(PAGE, encoding="utf8").read() == text else 1)
    open(PAGE, "w", encoding="utf8", newline="\n").write(text)
