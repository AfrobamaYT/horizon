#!/usr/bin/env python3
"""Choose a hosted runner using repository-local Actions history, never boat secrets."""
from datetime import datetime, timezone
import json
import os
import subprocess


def health_reason(jobs, now):
    """Return a runner outage reason, not an ordinary build/test failure."""
    for job in jobs:
        if "boat-large" not in job.get("labels", []):
            continue
        if job["status"] == "queued":
            created = datetime.fromisoformat(job["created_at"].replace("Z", "+00:00"))
            if (now - created).total_seconds() > 600:
                return f"boat job {job['id']} queued for more than 10 minutes"
        if job.get("conclusion") == "failure" and not any(
            step.get("started_at") for step in job.get("steps", [])
            if step.get("name") not in ("Set up job", "Complete job")
        ):
            return f"boat job {job['id']} failed before its first step"
    return ""


def select_label(enabled, users, author, actor, jobs, now):
    if enabled != "on" or author not in users or actor not in users:
        return "ubuntu-latest", "boat is disabled or the author/actor is not listed"
    reason = health_reason(jobs, now)
    return ("ubuntu-latest", reason) if reason else ("boat-large", "recent boat jobs show no runner outage")


def api(path):
    result = subprocess.run(["gh", "api", path], text=True, capture_output=True, check=True)
    return json.loads(result.stdout)


def main():
    repo = os.environ["GITHUB_REPOSITORY"]
    jobs = []
    reason = ""
    enabled = os.environ.get("BOAT_CI", "")
    users = os.environ.get("BOAT_USERS", "").split(",")
    author, actor = os.environ["AUTHOR"], os.environ["ACTOR"]
    if enabled == "on" and author in users and actor in users:
        try:
            # Bound the read cost: five recent runs, at most 100 jobs per run.
            for run in api(f"repos/{repo}/actions/runs?per_page=5")["workflow_runs"]:
                jobs.extend(api(f"repos/{repo}/actions/runs/{run['id']}/jobs?per_page=100")["jobs"])
        except (subprocess.CalledProcessError, json.JSONDecodeError, KeyError) as failure:
            reason = f"boat health history is unavailable ({type(failure).__name__})"
    label, decision = select_label(enabled, users, author, actor, jobs, datetime.now(timezone.utc))
    if reason:
        label, decision = "ubuntu-latest", reason
    print(f"hosted runner: {label}: {decision}")
    with open(os.environ["GITHUB_OUTPUT"], "a") as output:
        output.write(f"label={label}\n")
    with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as summary:
        summary.write(f"### Hosted runner selection\n\n{label}: {decision}.\n")


if __name__ == "__main__":
    main()
