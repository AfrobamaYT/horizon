#!/usr/bin/env python3
"""Cheap SDK input selection and failed-main attribution for the native linkage probe.

Path dependencies are conservatively traversed across all features (including build
and target dependencies). This may over-select, but never needs a hand-kept crate list.
"""
import json
import os
from pathlib import Path
import subprocess
import tomllib
import urllib.request

WORKFLOW = "native-dylib-probe.yml"


def sdk_roots(root):
    pending = [root / "crates/horizon-mod-host", root / "crates/horizon-mod-api-gen"]
    workspace = tomllib.loads((root / "Cargo.toml").read_text())
    shared = workspace.get("workspace", {}).get("dependencies", {})
    for patches in workspace.get("patch", {}).values():
        pending.extend(root / dep["path"] for dep in patches.values() if "path" in dep)
    seen = set()
    while pending:
        crate = pending.pop().resolve()
        if crate in seen:
            continue
        seen.add(crate)
        manifest = tomllib.loads((crate / "Cargo.toml").read_text())
        tables = [manifest, *manifest.get("target", {}).values()]
        for table in tables:
            for kind in ("dependencies", "build-dependencies"):
                for name, dep in table.get(kind, {}).items():
                    base = crate
                    if isinstance(dep, dict) and dep.get("workspace"):
                        dep = shared[name]
                        base = root
                    if isinstance(dep, dict) and "path" in dep:
                        pending.append(base / dep["path"])
    return [p.relative_to(root.resolve()).as_posix() + "/" for p in seen]


def selected(paths, roots):
    exact = {"Cargo.toml", "Cargo.lock", "launcher/src/rust_toolchain.rs",
             "tools/prebuilt_game.py", "tools/native_dylib_probe.py", "tools/native_probe_ci.py",
             ".github/workflows/native-dylib-probe.yml"}
    for path in paths:
        if path in exact or path.startswith(("rust-toolchain", ".cargo/", "tools/native_dylib_probe/",
                                               "crates/cargo-horizon-mod/template/native/")):
            return True
        if path.endswith(("/Cargo.toml", "/Cargo.lock")) and path.startswith(("crates/", "third_party/")):
            return True
        for prefix in roots:
            if path.startswith(prefix):
                suffix = path[len(prefix):]
                if not suffix.startswith(("tests/", "examples/", "benches/")) and not suffix.endswith(".md"):
                    return True
    return False


def baseline(runs, current):
    eligible = [r for r in runs if r["id"] != current["id"]
                and r["run_number"] < current["run_number"]
                and r["event"] == "push" and r["head_branch"] == "main"
                and r["head_repository"]["full_name"] == current["repository"]
                and r["conclusion"] == "success"]
    return max(eligible, key=lambda r: r["run_number"], default=None)


def summary(repo, head, green, commits, prs):
    lines = ["## MAIN BREAK: native dylib linkage probe failed", "Fast-path fix required."]
    if green is None:
        return "\n".join(lines + ["No successful main-push baseline exists; no last-green PR range can be claimed."])
    lines += [f"Last green: {green['html_url']}",
              f"Range: https://github.com/{repo}/compare/{green['head_sha']}...{head}"]
    found = sorted({p["number"] for c in commits for p in prs(c["sha"])
                    if p.get("merge_commit_sha") == c["sha"] and p.get("merged_at")
                    and p["base"]["repo"]["full_name"] == repo and p["base"]["ref"] == "main"})
    lines += ["Merged fork PRs in range: " + (", ".join(f"https://github.com/{repo}/pull/{n}" for n in found)
                                            or "none identified (inspect compare for direct pushes/upstream syncs)."),
              "PR attribution uses fork merge commits only; upstream PR numbers and rebased/squashed associations may be absent. Inspect the full compare range."]
    return "\n".join(lines)


def main():
    root = Path(__file__).resolve().parent.parent
    if os.environ.get("PROBE_SELECT"):
        paths = subprocess.check_output(["git", "diff", "--name-only", os.environ["BEFORE"], "HEAD"], text=True).splitlines()
        with open(os.environ["GITHUB_OUTPUT"], "a") as out:
            out.write(f"selected={str(selected(paths, sdk_roots(root))).lower()}\n")
        return
    repo = os.environ["GITHUB_REPOSITORY"]
    def api(path):
        request = urllib.request.Request(f"https://api.github.com/repos/{repo}/{path}",
                                         headers={"Authorization": f"Bearer {os.environ['GH_TOKEN']}",
                                                  "Accept": "application/vnd.github+json"})
        with urllib.request.urlopen(request) as response:
            return json.load(response)
    def pages(path, key=None):
        result = []
        for page in range(1, 1001):
            data = api(f"{path}{'&' if '?' in path else '?'}per_page=100&page={page}")
            items = data[key] if key else data
            result.extend(items)
            if len(items) < 100:
                return result
        raise RuntimeError("GitHub pagination limit reached; refusing incomplete attribution")
    current = api(f"actions/runs/{os.environ['GITHUB_RUN_ID']}")
    current["repository"] = repo
    runs = pages(f"actions/workflows/{WORKFLOW}/runs?branch=main&event=push&status=success", "workflow_runs")
    # A path-filtered no-op is not a green linkage proof.
    green = baseline(runs, current)
    while green:
        jobs = pages(f"actions/runs/{green['id']}/jobs", "jobs")
        proofs = [j for j in jobs if j["name"].startswith("linkage ") and j["conclusion"] == "success"]
        if len(proofs) == 2:
            break
        runs.remove(green)
        green = baseline(runs, current)
    commits = pages(f"compare/{green['head_sha']}...{current['head_sha']}", "commits") if green else []
    report = summary(repo, current["head_sha"], green, commits,
                     lambda sha: pages(f"commits/{sha}/pulls"))
    with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as out:
        out.write(report + "\n")


if __name__ == "__main__":
    main()
