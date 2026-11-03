#!/usr/bin/env python3
"""Run the #[ignore]d library tests that need the prepared vanilla data, against a list of known failures.

Usage: ignored_tests.py PACKAGE...  (HORIZON_TEST_RESOURCES points at the prepared 26.3 data)

Each package's library tests run with `--ignored`. The job fails when a test fails that
tools/ci/ignored-tests-known-failures.txt doesn't list, when a listed test passes (take it off the
list: the list only shrinks), and when a listed test no longer exists. A test marked `flaky` there
may pass or fail; one marked `harness` is a parity harness's step, run by its harness with the
environment it sets, and is skipped here.

Only test names, results and panic locations are printed, never a test's output or panic message:
the prepared data is Mojang's, and a failing assertion could quote it.
"""

import re
import subprocess
import sys
from pathlib import Path

KNOWN = Path(__file__).with_name("ignored-tests-known-failures.txt")
RESULT = re.compile(r"^test (\S+) \.\.\. (ok|FAILED)$")
SUMMARY = re.compile(r"^test result: ")
PANIC = re.compile(r"^thread '([^']+)'.* panicked at (\S+):$")


def known_failures():
    """(package, test) -> its mark (None, "flaky" or "harness"), from the checked-in list."""
    known = {}
    for number, line in enumerate(KNOWN.read_text().splitlines(), 1):
        line = line.split(" #", 1)[0].split()
        if not line:
            continue
        if len(line) < 3 or line[3:] not in ([], ["flaky"], ["harness"]):
            sys.exit(f"{KNOWN.name}:{number}: expected `PACKAGE TEST REFERENCE [flaky|harness]`")
        known[(line[0], line[1])] = line[3] if len(line) > 3 else None
    return known


def run(package, skip):
    """Each test's result in `package`'s ignored library tests but `skip`, or None if they didn't build."""
    skips = [arg for test in skip for arg in ("--skip", test)]
    process = subprocess.run(
        ["cargo", "test", "-p", package, "--lib", "--", "--ignored", "--exact", "--test-threads=4", *skips],
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace",
    )
    results = {}
    summaries = []
    for line in process.stdout.splitlines():
        if found := RESULT.match(line):
            results[found[1]] = found[2] == "ok"
        elif found := PANIC.match(line):
            print(f"  {found[1]} panicked at {found[2]}")
        elif SUMMARY.match(line):
            summaries.append(line)
    if not summaries:
        # A build failure: the compiler's output quotes only this repository's code.
        print(process.stdout[-20000:])
        return None
    print(f"{package}: {summaries[-1]}")
    return results


def main(packages):
    known = known_failures()
    problems = []
    for package in packages:
        skip = [test for (listed, test), mark in known.items() if listed == package and mark == "harness"]
        results = run(package, skip)
        if results is None:
            problems.append(f"{package}: the tests didn't build")
            continue
        for test, passed in sorted(results.items()):
            listed = (package, test) in known
            if not passed and not listed:
                problems.append(f"{package} {test}: failed, and isn't a known failure")
            elif passed and listed and known[(package, test)] is None:
                problems.append(f"{package} {test}: passes now; take it off {KNOWN.name}")
        for (listed, test), mark in known.items():
            if listed == package and mark != "harness" and test not in results:
                problems.append(f"{package} {test}: listed in {KNOWN.name} but no longer runs")
    for problem in problems:
        print(f"::error::{problem}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
