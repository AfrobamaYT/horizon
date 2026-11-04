#!/usr/bin/env python3
"""The prebuilt game (#589): horizon-client as a Rust dylib plus every crate's metadata, so the launcher builds
native trait mods against it with rustc alone, in seconds, without the game's source or a game build.

    python3 tools/prebuilt_game.py build OUT [--server]   # build and collect the folder, write horizon-prebuilt.json
    python3 tools/prebuilt_game.py smoke OUT              # build the starter mod and the game executable against OUT only

Each OS's folder is built on that OS: the proc-macro libraries in it run inside the player's own rustc.
The folder holds only compiler outputs, never vanilla data; `build` refuses any other file.
"""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PROFILE = "fast"
# What rustc writes for a dylib or proc macro on each OS, plus metadata and the MinGW import libraries.
SUFFIXES = (".rmeta", ".so", ".dll", ".dll.a", ".dylib")
IDENTITY = "horizon-prebuilt.json"
LOCK = "Cargo.lock"


def run(command: list[str], **kwargs) -> str:
    print("+", " ".join(command), flush=True)
    return subprocess.run(command, check=True, text=True, stdout=subprocess.PIPE, **kwargs).stdout


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for block in iter(lambda: file.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def rustc_host() -> str:
    verbose = run(["rustc", "-vV"])
    return next(line.split(": ", 1)[1] for line in verbose.splitlines() if line.startswith("host: "))


def build(out: Path, server: bool) -> None:
    target_dir = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")) / "prebuilt"
    # One std for the game, every mod and the executable: the game links std dynamically, as mods will.
    rustflags = "-C prefer-dynamic"
    features = ["--no-default-features"] if server else []
    run(
        ["cargo", "rustc", "--locked", "-p", "horizon-client", "--lib", "--crate-type", "dylib",
         "--profile", PROFILE, *features],
        cwd=ROOT,
        env={**os.environ, "RUSTFLAGS": rustflags, "CARGO_TARGET_DIR": str(target_dir)},
    )
    if out.exists():
        shutil.rmtree(out)
    deps = out / "deps"
    deps.mkdir(parents=True)
    for file in sorted((target_dir / PROFILE / "deps").iterdir()):
        # Metadata, proc macros and the game; the other crates' code is inside the game's dylib.
        if file.name.endswith(SUFFIXES):
            shutil.copy2(file, deps / file.name)
    shutil.copy2(ROOT / LOCK, out / LOCK)
    files = {str(path.relative_to(out).as_posix()): sha256(path) for path in sorted(out.rglob("*")) if path.is_file()}
    identity = {
        "commit": run(["git", "rev-parse", "HEAD"], cwd=ROOT).strip(),
        "rustc": run(["rustc", "-vV"]).strip(),
        "target": rustc_host(),
        "profile": PROFILE,
        "side": "server" if server else "client",
        "rustflags": rustflags,
        "files": files,
    }
    (out / IDENTITY).write_text(json.dumps(identity, indent=2) + "\n")
    check(out)


def game_dylib(deps: Path) -> Path | None:
    """The game itself: a dylib carries its own metadata, so it has no `.rmeta` beside it."""
    names = {f"{prefix}horizon_client.{ext}" for prefix in ("lib", "") for ext in ("so", "dll", "dylib")}
    return next((path for path in deps.iterdir() if path.name in names), None)


def check(out: Path) -> None:
    """Only compiler outputs: a stray asset, vanilla file or anything else fails the build."""
    stray = [
        path for path in out.rglob("*")
        if path.is_file() and path.name not in (IDENTITY, LOCK)
        and not (path.parent == out / "deps" and path.name.endswith(SUFFIXES))
    ]
    if stray:
        sys.exit(f"prebuilt folder holds files that aren't compiler outputs: {stray}")
    if game_dylib(out / "deps") is None:
        sys.exit("prebuilt folder has no horizon-client dylib")


def smoke(out: Path) -> None:
    """Build what a player's launcher builds: the starter mod and the generated executable, from OUT alone."""
    deps = out / "deps"

    def extern(crate: str) -> list[str]:
        prefix = f"lib{crate}-"
        rmeta = next(path for path in deps.iterdir() if path.name.startswith(prefix) and path.suffix == ".rmeta")
        return ["--extern", f"{crate}={rmeta}"]

    common = ["--edition", "2024", "-C", "prefer-dynamic", "-C", "opt-level=1", "-L", f"dependency={deps}"]
    with tempfile.TemporaryDirectory(dir=out.parent) as scratch:
        scratch = Path(scratch)
        starter = (ROOT / "crates/cargo-horizon-mod/template/native/lib.rs").read_text()
        (scratch / "mod_0.rs").write_text(starter.replace("{id}", "smoke"))
        run(["rustc", *common, "--crate-type", "rlib", "--crate-name", "mod_0", *extern("mod_api"),
             "--out-dir", str(scratch), str(scratch / "mod_0.rs")])
        template = (ROOT / "crates/horizon-client/src/native_main.rs.template").read_text()
        main = template.replace("/* MOD_ENTRIES */", '        "smoke" => mod_0::hooks,\n').replace(
            "/* BUILT_MODS */",
            '    horizon_mods::runtime::BuiltMod { id: "smoke", version: "0.1.0", side: horizon_mods::Side::Both },\n',
        )
        (scratch / "main.rs").write_text(main)
        run(["rustc", *common, "--crate-type", "bin", "--crate-name", "horizon_client_smoke",
             "--extern", f"horizon_client={game_dylib(deps)}", *extern("mod_api"), *extern("horizon_mods"),
             "--extern", f"mod_0={scratch / 'libmod_0.rlib'}", "-L", f"dependency={scratch}",
             "-o", str(scratch / "horizon-client"), str(scratch / "main.rs")])
    print(f"smoke: the starter mod and the game executable built against {out} alone")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)
    build_parser = commands.add_parser("build")
    build_parser.add_argument("out", type=Path)
    build_parser.add_argument("--server", action="store_true", help="the dedicated server's features")
    smoke_parser = commands.add_parser("smoke")
    smoke_parser.add_argument("out", type=Path)
    args = parser.parse_args()
    if args.command == "build":
        build(args.out.resolve(), args.server)
    else:
        smoke(args.out.resolve())


if __name__ == "__main__":
    main()
