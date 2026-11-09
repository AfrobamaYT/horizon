#!/usr/bin/env python3
"""Trusted API-only recovery of fork PR boat startup outages.

This script comes exclusively from the controller's default-branch workflow SHA.
It neither downloads nor executes code from the triggering PR.
"""
from datetime import datetime, timezone
import json
import os
import subprocess
import time

from boat_health import api, genuine_failure, recovery_reason


def eligible(run, repo, workflow):
    return (run["workflow_id"] == workflow and run["event"] == "pull_request"
            and run["run_attempt"] == 1
            and run.get("conclusion") not in ("success", "cancelled", "skipped")
            and run.get("head_repository") is not None
            and run["head_repository"]["full_name"] != repo)


def log(message):
    print(message, flush=True)
    with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as summary:
        summary.write(message + "\n\n")


def post(path):
    subprocess.run(["gh", "api", "--method", "POST", path], check=True)


def recover(repo, run):
    path = f"repos/{repo}/actions/runs/{run['id']}"
    jobs = api(path + "/attempts/1/jobs?per_page=100")["jobs"]
    reason = recovery_reason(jobs, datetime.now(timezone.utc))
    if not reason:
        return
    log(f"Run {run['id']}: {reason}.")
    if run["status"] != "completed":
        log(f"Cancelling run {run['id']} to release its stranded boat jobs.")
        post(path + "/cancel")
        for _ in range(20):
            if api(path)["status"] == "completed":
                break
            time.sleep(3)
        else:
            raise RuntimeError("Cancellation has not completed; refusing an overlapping rerun")
    # The preceding attempt is now immutable. Recheck before rerunning so a real
    # failure arriving between inspection and cancellation is never retried.
    jobs = api(path + "/attempts/1/jobs?per_page=100")["jobs"]
    if genuine_failure(jobs):
        log(f"Run {run['id']}: a genuine failure appeared; no rerun requested.")
        return
    log(f"Rerunning run {run['id']}; changes reads the previous attempt's outage and forces ubuntu-latest.")
    post(path + "/rerun")


def main():
    repo = os.environ["GITHUB_REPOSITORY"]
    workflow = api(f"repos/{repo}/actions/workflows/dyna-ci.yml")["id"]
    with open(os.environ["GITHUB_EVENT_PATH"]) as event_file:
        event = json.load(event_file)
    if os.environ["GITHUB_EVENT_NAME"] == "workflow_run":
        run_id = event["workflow_run"]["id"]
        run = api(f"repos/{repo}/actions/runs/{run_id}")
        if eligible(run, repo, workflow):
            recover(repo, run)
    else:
        for run in api(f"repos/{repo}/actions/workflows/{workflow}/runs?event=pull_request&per_page=30")["workflow_runs"]:
            if eligible(run, repo, workflow):
                recover(repo, run)


if __name__ == "__main__":
    main()
