"""Run a preparation boundary privately; publish only fixed reason codes, never log text."""

import argparse
import errno
from pathlib import Path
import subprocess

STEPS = ("preparer-build", "resource-export", "seed-extras")
# These are diagnostic hints, not a claim about the root cause. Never interpolate matches.
REASONS = (
    ("no space left on device", "disk-full"),
    ("read-only file system", "read-only-filesystem"),
    ("permission denied", "permission-denied"),
    ("outofmemoryerror", "java-out-of-memory"),
    ("could not resolve host", "network-dns"),
    ("connection timed out", "network-timeout"),
    ("no such file or directory", "missing-file"),
)


def run(step: str, log: Path, command: list[str]) -> int:
    reason = "unclassified"
    try:
        with log.open("wb") as output:
            result = subprocess.run(command, stdout=output, stderr=subprocess.STDOUT, check=False)
        status = result.returncode
        if status == 0:
            return 0
        # Bounded private read. Unknown or late failures stay explicitly unclassified.
        with log.open("rb") as source:
            sample = source.read(1024 * 1024).lower()
        reason = next((code for marker, code in REASONS if marker.encode() in sample), reason)
    except OSError as error:
        status = 1
        reason = {errno.ENOSPC: "disk-full", errno.EROFS: "read-only-filesystem",
                  errno.EACCES: "permission-denied", errno.ENOENT: "missing-file"}.get(error.errno, "os-error")
    # step is validated once by argparse; status is an integer from subprocess, not log data.
    print(f"::error::preparation step={step} exit={status} reason_hint={reason}", flush=True)
    return status if status > 0 else 128 - status


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("step", choices=STEPS)
    parser.add_argument("log", type=Path)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if not args.command:
        parser.error("a command is required")
    return run(args.step, args.log, args.command)


if __name__ == "__main__":
    raise SystemExit(main())
