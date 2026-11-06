#!/usr/bin/env python3
"""The gate for the host split: the generated `mod-api.wit` must not change.

The WIT holds every hook, every host import, each signature and each doc, so byte equality with the
base proves the area traits declare exactly the methods the `#[helper]` entries did, in the same order.
A host method that moved to a different trait of the same union does not change the WIT; one that
changed its signature, doc or order does.

Usage: gate.py BASE_COMMIT [SCRATCH_DIR]
Builds `horizon-mod-api` at BASE_COMMIT in a throwaway worktree and in this checkout, and compares
the two generated WIT files. SCRATCH_DIR (default `target/api-pr1-gate`) holds the worktree and its
target folder; delete it afterwards.
"""
import filecmp
import glob
import os
import subprocess
import sys
from pathlib import Path


def build(directory, target):
    env = dict(os.environ, CARGO_TARGET_DIR=str(target))
    subprocess.run(["cargo", "build", "-p", "horizon-mod-api"], cwd=directory, env=env, check=True)
    wits = glob.glob(str(target / "debug/build/horizon-mod-api-*/out/mod-api.wit"))
    return Path(max(wits, key=os.path.getmtime))


def main():
    base = sys.argv[1]
    root = Path(subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip())
    scratch = Path(sys.argv[2]) if len(sys.argv) > 2 else root / "target/api-pr1-gate"
    tree = scratch / "base"
    if not tree.exists():
        subprocess.run(["git", "worktree", "add", "--detach", str(tree), base], cwd=root, check=True)
    old = build(tree, scratch / "base-target")
    new = build(root, Path(os.environ.get("CARGO_TARGET_DIR", root / "target")))
    if filecmp.cmp(old, new, shallow=False):
        print(f"gate passed: {new} is byte-identical to the WIT at {base}")
        return
    subprocess.run(["diff", str(old), str(new)])
    sys.exit(f"gate failed: the WIT at {base} and this checkout differ")


main()
