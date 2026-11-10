#!/usr/bin/env python3
"""Compile an immutable machine sample revision against this checkout's API.

API-breaking PRs may pair a sample update using a standalone body line:
Sample commit: <40-character SHA>
Also link the paired sample PR in the PR description for reviewers.
"""
import json
import os
from pathlib import Path
import re
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
PIN = "77b4b6db3e6c7521acbef53e45fab2f3e66e6305"


def main():
    event_path = os.environ.get("GITHUB_EVENT_PATH")
    event = json.loads(Path(event_path).read_text()) if event_path else {}
    body = event.get("pull_request", {}).get("body") or ""
    match = re.search(r"^Sample commit: ([0-9a-f]{40})\s*$", body, re.MULTILINE)
    revision = match[1] if match else PIN
    started = time.monotonic()
    sample = ROOT.parent / "ci-machine-sample"
    subprocess.run(["git", "clone", "--no-checkout", "https://github.com/arismoko/horizon-machine-sample", str(sample)], check=True)
    subprocess.run(["git", "fetch", "origin", revision], cwd=sample, check=True)
    subprocess.run(["git", "checkout", "--detach", revision], cwd=sample, check=True)
    # Cargo source patches replace the pinned game dependency with this PR's API.
    patch = sample / "api-patch.toml"
    patch.write_text('[patch."https://github.com/arismoko/horizon"]\n'
                     + 'horizon-mod-api = { path = ' + json.dumps(str(ROOT / "crates/horizon-mod-api")) + ' }\n')
    subprocess.run(["cargo", "check", "--all-targets", "--config", str(patch)], cwd=sample, check=True)
    elapsed = time.monotonic() - started
    summary = f"Machine sample `{revision}`: {elapsed:.1f} seconds ({elapsed / 60:.2f} runner minutes).\n"
    print(summary)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as output:
            output.write(summary)


if __name__ == "__main__":
    main()
