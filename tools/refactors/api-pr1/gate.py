#!/usr/bin/env python3
"""The gate for the host split: nothing mods see about the host may change.

Two comparisons between a base commit and this checkout, both built from scratch:

1. `mod-api.wit` byte for byte. It holds every hook, host import, signature and their order, but no
   method docs.
2. The host manifest (`host-manifest`): one line per host method with its owner trait, signature,
   receiver mutability, rustdoc and hidden flag. The base's generated host is read as it is; this
   checkout's is the generated unions plus the area files, with each area trait counted under the union
   it is a part of. This is what catches a lost or changed doc.

Usage: gate.py BASE_COMMIT [SCRATCH_DIR]
Each resolved BASE_COMMIT gets its own worktree and target folder under SCRATCH_DIR (default
`target/api-pr1-gate`); delete the folder afterwards.
"""
import filecmp
import glob
import os
import subprocess
import sys
from pathlib import Path


def build(directory, target, tool=False):
    """Builds `horizon-mod-api` in `directory` and returns its build output folder (the manifest tool
    is built only here: it reads text, so one tool serves both sides)."""
    env = dict(os.environ, CARGO_TARGET_DIR=str(target))
    if tool:
        subprocess.run(["cargo", "build", "-p", "horizon-mod-api-gen", "--bin", "host-manifest"], cwd=directory, env=env, check=True)
    subprocess.run(["cargo", "build", "-p", "horizon-mod-api"], cwd=directory, env=env, check=True)
    return Path(max(glob.glob(str(target / "debug/build/horizon-mod-api-*/out")), key=os.path.getmtime))


def manifest(tool, files):
    return subprocess.run([str(tool), *map(str, files)], check=True, capture_output=True, text=True).stdout


def main():
    root = Path(subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip())
    base = subprocess.check_output(["git", "rev-parse", sys.argv[1]], cwd=root, text=True).strip()
    scratch = (Path(sys.argv[2]) if len(sys.argv) > 2 else root / "target/api-pr1-gate").resolve() / base[:12]
    tree = scratch / "base"
    if not tree.exists():
        subprocess.run(["git", "worktree", "add", "--detach", str(tree), base], cwd=root, check=True)
    actual = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=tree, text=True).strip()
    dirty = subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=all"], cwd=tree, text=True)
    if actual != base or dirty:
        sys.exit(f"gate refused scratch worktree {tree}: expected clean {base}, found {actual}"
                 + (f"\n{dirty}" if dirty else ""))
    old_out = build(tree, scratch / "target")
    target = Path(os.environ.get("CARGO_TARGET_DIR", root / "target"))
    new_out = build(root, target, tool=True)
    tool = target / "debug/host-manifest"
    failed = False
    if not filecmp.cmp(old_out / "mod-api.wit", new_out / "mod-api.wit", shallow=False):
        subprocess.run(["diff", str(old_out / "mod-api.wit"), str(new_out / "mod-api.wit")])
        print("gate failed: mod-api.wit differs")
        failed = True
    old_areas = sorted((tree / "crates/horizon-mod-api/src/host").glob("*.rs"))
    old = manifest(tool, [old_out / "mod_api_generated_host.rs", *old_areas])
    areas = sorted((root / "crates/horizon-mod-api/src/host").glob("*.rs"))
    new = manifest(tool, [new_out / "mod_api_generated_host.rs", *areas])
    if old != new:
        old_lines, new_lines = set(old.splitlines()), set(new.splitlines())
        for line in sorted(old_lines - new_lines):
            print("only in base:", line[:300])
        for line in sorted(new_lines - old_lines):
            print("only here:   ", line[:300])
        print("gate failed: the host manifest differs")
        failed = True
    if failed:
        sys.exit(1)
    print(f"gate passed against {base[:12]}: identical WIT and {len(new.splitlines())} identical host methods")


main()
