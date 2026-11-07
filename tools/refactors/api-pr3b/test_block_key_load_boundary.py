#!/usr/bin/env python3
"""Run the exact game completion expression without linking the whole game."""
import json
import os
from pathlib import Path
import subprocess
import shutil

root = Path(__file__).resolve().parents[3]
game = (root / "crates/horizon-client/src/game.rs").read_text()
call = next(line.strip() for line in game.splitlines() if "api_values::resolve_block_definitions(" in line)
source = (root / "crates/horizon-client/src/game/modding/api_values.rs").read_text()
def production_function(name):
    start = source.index(f"fn {name}(")
    start = source.rfind("\n", 0, start) + 1
    return source[start:source.index("\n}\n", start) + 3]

source = (production_function("resolve_block_definitions") + "\n#[test]\n" +
          production_function("block_definition_import_preserves_defaults_and_is_immutable_after_load"))
source = source.replace("pub(in crate::game)", "pub(crate)")
fixture = Path(__file__).with_name("block_key_load_boundary.rs").read_text()
fixture = "mod api_values {\n" + source + "}\n" + fixture
fixture = fixture.replace("//! Execute", "// Execute")
fixture = fixture.replace("// PRODUCTION_COMPLETION", "struct Assets<'a> { registry: &'a BlockRegistry }\nlet assets = Assets { registry };\n" + call)
scratch = root / "target/block-key-load-boundary"
(scratch / "tests").mkdir(parents=True, exist_ok=True)
(scratch / "tests/load.rs").write_text(fixture)
manifest = '[workspace]\n[package]\nname="block-key-load-boundary"\nversion="0.0.0"\nedition="2024"\n[dependencies]\n'
for name in ("horizon-world", "horizon-mod-runtime", "horizon-mod-api"):
    key = "mod_api" if name == "horizon-mod-api" else name.replace("-", "_")
    features = ', features=["testing"]' if name == "horizon-mod-api" else ""
    manifest += f'{key} = {{ package={json.dumps(name)}, path={json.dumps(str(root / "crates" / name))}{features} }}\n'
(scratch / "Cargo.toml").write_text(manifest)
shutil.copyfile(root / "Cargo.lock", scratch / "Cargo.lock")
subprocess.run(["cargo", "test", "--offline", "--manifest-path", str(scratch / "Cargo.toml"), "--test", "load"],
               env=dict(os.environ, CARGO_TARGET_DIR=os.environ.get("CARGO_TARGET_DIR", str(root / "target"))), check=True)
