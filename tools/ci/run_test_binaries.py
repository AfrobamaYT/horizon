"""Run a workspace's built test binaries several at a time, as `cargo test --no-fail-fast` would one by one.

    python3 tools/ci/run_test_binaries.py --jobs 4 -- cargo +stable test $JOBS --profile test --workspace --lib --bins --tests

The cargo command is run once more with `--no-run --message-format=json-render-diagnostics` to list the test
executables (already built, so nothing compiles). Each binary then runs as Cargo runs it: from its package's
directory, with the package's CARGO_* variables, every test of it, and its whole output printed together after it
ends. Binaries start largest first, since the largest are the slow ones. The exit status is non-zero when any
binary fails, crashes or is killed, after every binary has run.
"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path


def package_identity(package_id: str) -> tuple[str, str]:
    """(name, version) from a Cargo package id, old (`name 1.0 (path+...)`) or new (`path+...#name@1.0`)."""
    if found := re.fullmatch(r"(\S+) (\S+) \(.*\)", package_id):
        return found[1], found[2]
    fragment = package_id.rsplit("#", 1)[-1]
    name, _, version = fragment.rpartition("@")
    if not name:  # `path+file:///x/horizon-world#0.1.0`: the name is the last path segment.
        name = package_id.split("#", 1)[0].rstrip("/").rsplit("/", 1)[-1]
    return name, version


def test_binaries(messages: str) -> list[dict]:
    """The test executables Cargo reported, each once, with what running it needs."""
    binaries, seen = [], set()
    for line in messages.splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line)
        if message.get("reason") != "compiler-artifact" or not message["profile"]["test"]:
            continue
        executable = message.get("executable")
        if not executable or executable in seen:
            continue
        seen.add(executable)
        name, version = package_identity(message["package_id"])
        manifest = message["manifest_path"]
        binaries.append({
            "executable": executable,
            "label": f"{name} {Path(message['target']['src_path']).name} ({Path(executable).name})",
            "cwd": str(Path(manifest).parent),
            "env": {
                "CARGO_MANIFEST_DIR": str(Path(manifest).parent),
                "CARGO_MANIFEST_PATH": manifest,
                "CARGO_PKG_NAME": name,
                "CARGO_PKG_VERSION": version,
                "CARGO_CRATE_NAME": message["target"]["name"].replace("-", "_"),
            },
        })
    return binaries


def run_one(binary: dict, extra_env: dict) -> tuple[dict, int, str, float]:
    started = time.monotonic()
    process = subprocess.run([binary["executable"]], cwd=binary["cwd"], env={**os.environ, **extra_env, **binary["env"]},
                             stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
    return binary, process.returncode, process.stdout, time.monotonic() - started


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--jobs", type=int, required=True, help="test binaries running at once")
    parser.add_argument("cargo", nargs=argparse.REMAINDER, help="the cargo test command, after --")
    args = parser.parse_args(argv)
    command = args.cargo[1:] if args.cargo[:1] == ["--"] else args.cargo
    listing = subprocess.run([*command, "--no-run", "--message-format=json-render-diagnostics"],
                             stdout=subprocess.PIPE, text=True)
    if listing.returncode:
        print(f"listing the test binaries failed (cargo exit {listing.returncode})")
        return listing.returncode
    binaries = sorted(test_binaries(listing.stdout), key=lambda b: os.path.getsize(b["executable"]), reverse=True)
    # As Cargo does for `cargo test`: std and the target's dynamic libraries, and the cargo that ran the build.
    toolchain = [command[1]] if len(command) > 1 and command[1].startswith("+") else []
    sysroot = subprocess.run(["rustc", *toolchain, "--print", "sysroot"], capture_output=True,
                             text=True).stdout.strip() if shutil.which("rustc") else ""
    deps = {str(Path(b["executable"]).parent) for b in binaries}
    library_path = os.pathsep.join([*sorted(deps), *([f"{sysroot}/lib"] if sysroot else []),
                                    *filter(None, [os.environ.get("LD_LIBRARY_PATH")])])
    extra_env = {"LD_LIBRARY_PATH": library_path, "CARGO": os.environ.get("CARGO", "cargo")}
    print(f"running {len(binaries)} test binaries, {args.jobs} at a time", flush=True)
    failed, started = [], time.monotonic()
    with ThreadPoolExecutor(args.jobs) as pool:
        for binary, code, output, seconds in pool.map(lambda b: run_one(b, extra_env), binaries):
            print(f"     Running {binary['label']} [{seconds:.1f}s]")
            print(output, end="" if output.endswith("\n") else "\n", flush=True)
            if code:
                failed.append(f"{binary['label']}: exit {code}")
    print(f"ran {len(binaries)} test binaries in {time.monotonic() - started:.1f}s; {len(failed)} failed")
    for line in failed:
        print(f"::error::{line}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
