#!/usr/bin/env python3
"""Execute the production metadata importer and its regression without linking the game.

Only the function's module visibility changes in this isolated test crate.
"""
import json
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[3]
source = (root / "crates/horizon-client/src/game/modding/api_values.rs").read_text()
start = source.index("pub(in crate::game) fn resolve_block_definitions(")
stop = source.index("/// Registry access for generated stack write-back", start)
source = source[start:stop].replace("pub(in crate::game)", "pub(crate)")
scratch = root / "target/block-definition-import"
(scratch / "src").mkdir(parents=True, exist_ok=True)
(scratch / "src/lib.rs").write_text(source)
manifest = '[workspace]\n[package]\nname="block-definition-import"\nversion="0.0.0"\nedition="2024"\n[dependencies]\n'
for name in ("horizon-world", "horizon-mod-runtime", "horizon-mod-api"):
    key = "mod_api" if name == "horizon-mod-api" else name.replace("-", "_")
    features = ', features=["testing"]' if name == "horizon-mod-api" else ""
    manifest += f'{key} = {{ package={json.dumps(name)}, path={json.dumps(str(root / "crates" / name))}{features} }}\n'
(scratch / "Cargo.toml").write_text(manifest)
subprocess.run(["cargo", "test", "--manifest-path", str(scratch / "Cargo.toml")],
               env=dict(os.environ, CARGO_TARGET_DIR=str(root / "target")), check=True)
