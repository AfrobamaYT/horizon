#!/usr/bin/env python3
"""Log the mechanical API-owned state/key symbol cut after handwritten prerequisites.

Dry-run by default. Public symbols change only inside mod_api. Two Game-only imports
move off the deleted alias without changing numeric engine storage. Raw API constructor
fixtures are a handwritten prerequisite rather than guessed ordinal mappings.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--root", type=Path, default=Path.cwd())
parser.add_argument("--report", type=Path, required=True)
parser.add_argument("--apply", action="store_true")
args = parser.parse_args()
root = args.root.resolve()
api = root / "crates/horizon-mod-api/src"
if not (root / "Cargo.toml").is_file() or not api.is_dir():
    parser.error("root must be the Horizon checkout")

symbols = re.compile(r"\b(BlockStateId|BlockId)\b")
constructors = re.compile(r"\b(?:BlockStateId|BlockId)\s*\(")
old_export = "    horizon_world::{level::BlockId, registry::BlockStateId},\n"
changes = []
pending = []
refused = []


def record_change(path, source, replacement):
    changes.append({
        "file": path.relative_to(root).as_posix(),
        "before_sha256": hashlib.sha256(source.encode()).hexdigest(),
        "after_sha256": hashlib.sha256(replacement.encode()).hexdigest(),
        "states": len(re.findall(r"\bBlockStateId\b", source)),
        "keys": len(re.findall(r"\bBlockId\b", source)),
    })
    pending.append((path, replacement))


for path in sorted(api.rglob("*.rs")):
    source = path.read_text()
    if not symbols.search(source):
        continue
    relative = path.relative_to(root).as_posix()
    if relative == "crates/horizon-mod-api/src/exports/horizon_world.rs":
        if source.count(old_export) != 1:
            refused.append({"file": relative, "reason": "raw export shape changed; inspect it"})
            continue
        replacement = source.replace(old_export, "")
    else:
        if constructors.search(source):
            refused.append({"file": relative, "reason": "numeric constructor fixture must be migrated by hand"})
        if re.search(r"horizon_world::[^;\n]*\b(?:BlockStateId|BlockId)\b", source):
            refused.append({"file": relative, "reason": "qualified engine import needs boundary ownership review"})
        replacement = symbols.sub(lambda match: "BlockState" if match[0] == "BlockStateId" else "BlockKey", source)
    record_change(path, source, replacement)

# These private native methods consume engine states, not API payload values.
for relative in [
    "crates/horizon-client/src/game/modding/native_entity_refs.rs",
    "crates/horizon-client/src/game/modding/native_block_sprites.rs",
]:
    path = root / relative
    if not path.is_file():
        continue
    source = path.read_text()
    replacement = source.replace("mod_api::BlockStateId", "horizon_world::registry::BlockStateId")
    if replacement != source:
        record_change(path, source, replacement)

report = {"applied": args.apply and not refused, "files": changes, "refused": refused}
args.report.parent.mkdir(parents=True, exist_ok=True)
args.report.write_text(json.dumps(report, indent=2) + "\n")
if args.apply:
    if refused:
        raise SystemExit("handwritten prerequisites remain; no source files changed")
    for path, replacement in pending:
        path.write_text(replacement)
print(json.dumps({"files": len(changes), "refused": refused, "applied": report["applied"]}, indent=2))
