#!/usr/bin/env python3
"""Generate a rolling public activity digest; no dependencies or model calls.

Rollups expand only the PR column of their membership table, never arbitrary
issue references. Member PRs also returned by search are deduplicated.
"""
import argparse
from collections import defaultdict
from datetime import datetime, timedelta, timezone
import json
import os
from pathlib import Path
import re
import time
from urllib.parse import quote
from urllib.request import Request, urlopen

REPO = "arismoko/horizon"
WEB = f"https://github.com/{REPO}"
API = f"https://api.github.com/repos/{REPO}"
SECTIONS = ("Merged", "Releases", "New issues open for contributors", "Fixed bugs")


def escaped(value):
    # Entities prevent Markdown, HTML, mentions and line breaks from becoming markup.
    return "".join(c if c.isalnum() or c == " " else f"&#{ord(c)};"
                   for c in " ".join(value.split()))


def get(url, payload=None):
    headers = {"Accept": "application/vnd.github+json"}
    if os.environ.get("GH_TOKEN"):
        headers["Authorization"] = "Bearer " + os.environ["GH_TOKEN"]
    body = json.dumps(payload).encode() if payload is not None else None
    with urlopen(Request(url, data=body, headers=headers), timeout=60) as response:
        return json.load(response)


def search(query):
    # Split by date at the caller to keep below GitHub's 1,000-result ceiling.
    result = []
    for page in range(1, 11):
        # GitHub permits 30 authenticated search requests per minute.
        time.sleep(2.1)
        data = get("https://api.github.com/search/issues?q=" + quote(
            f"repo:{REPO} {query}") + f"&per_page=100&page={page}")
        if data.get("incomplete_results") or data["total_count"] > 1000:
            raise RuntimeError("GitHub search is incomplete; refusing partial digest")
        result.extend(data["items"])
        if len(result) >= data["total_count"]:
            return result
    raise RuntimeError("Search pagination exhausted")


def members(pr):
    return [int(n) for n in re.findall(r"^\|\s*#(\d+)\s*\|", pr.get("body") or "", re.M)]


def merged_prs(day):
    """Batch the fields needed for expansion instead of one REST call per PR."""
    query = """query($query: String!, $after: String) {
      search(query: $query, type: ISSUE, first: 100, after: $after) {
        issueCount pageInfo { hasNextPage endCursor }
        nodes { ... on PullRequest {
          number title body mergedAt headRefName headRefOid
          author { login __typename } labels(first: 100) { nodes { name } }
        } }
      }
    }"""
    after = None
    result = []
    while True:
        time.sleep(2.1)
        response = get("https://api.github.com/graphql", {"query": query, "variables": {
            "query": f"repo:{REPO} is:pr is:merged merged:{day}", "after": after}})
        if response.get("errors"):
            raise RuntimeError("GitHub GraphQL query failed")
        batch = response["data"]["search"]
        if batch["issueCount"] > 1000:
            raise RuntimeError("GitHub search exceeds 1,000 results for one day")
        for pr in batch["nodes"]:
            author = pr["author"] or {"login": "deleted", "__typename": "User"}
            result.append({"number": pr["number"], "title": pr["title"], "body": pr["body"],
                           "merged_at": pr["mergedAt"], "head": {
                               "ref": pr["headRefName"], "sha": pr["headRefOid"]},
                           "user": {"login": author["login"], "type": author["__typename"]},
                           "labels": pr["labels"]["nodes"]})
        if not batch["pageInfo"]["hasNextPage"]:
            return result
        after = batch["pageInfo"]["endCursor"]


def collect(now, days):
    prs, issues = {}, {}
    for offset in range(days):
        day = (now - timedelta(days=offset)).date().isoformat()
        for item in merged_prs(day):
            prs[item["number"]] = item
        for query in (f"is:issue label:\"help wanted\" created:{day}",
                      f"is:issue label:bug closed:{day}"):
            for item in search(query):
                issues[item["number"]] = item
    for pr in list(prs.values()):
        if pr["head"]["ref"].startswith("fork-review-rollup-"):
            for number in members(pr):
                if number not in prs:
                    prs[number] = get(API + f"/pulls/{number}")
    releases = []
    page = 1
    while True:
        batch = get(API + f"/releases?per_page=100&page={page}")
        releases.extend(batch)
        if len(batch) < 100:
            break
        page += 1
    return {"prs": list(prs.values()), "issues": list(issues.values()), "releases": releases}


def render(data, now, days=30, cap=12):
    first = (now - timedelta(days=days - 1)).date().isoformat()
    last = now.date().isoformat()
    groups = defaultdict(lambda: defaultdict(list))

    def add(date, section, text, number=0):
        if date and first <= date[:10] <= last:
            groups[date[:10]][section].append((number, text))

    prs = {p["number"]: p for p in data["prs"]}
    rollups = [p for p in prs.values() if p["head"]["ref"].startswith("fork-review-rollup-")]
    dates = {n: p["merged_at"] for p in rollups for n in members(p)}
    for n, pr in prs.items():
        if pr in rollups or not pr.get("merged_at"):
            continue
        labels = {label["name"] for label in pr["labels"]}
        if "skip-release" in labels:
            continue
        if not labels and (pr["user"].get("type") == "Bot" or
                           re.match(r"^(ci|chore\(ci\)|build)(:|\()", pr["title"], re.I)):
            continue
        text = f"[{escaped(pr['title'])} (#{n})]({WEB}/pull/{n}) — @{escaped(pr['user']['login'])}"
        if re.search(r"upstream.*sync|sync.*upstream|merge upstream", pr["title"], re.I):
            text += f" — upstream commit `{pr['head']['sha']}`"
        add(dates.get(n, pr["merged_at"]), "Merged", text, n)
    for item in data["issues"]:
        labels = {label["name"] for label in item["labels"]}
        text = f"[{escaped(item['title'])} (#{item['number']})]({WEB}/issues/{item['number']})"
        if "help wanted" in labels and item["state"] == "open":
            add(item["created_at"], SECTIONS[2], text, item["number"])
        if "bug" in labels and item["state"] == "closed":
            add(item["closed_at"], SECTIONS[3], text, item["number"])
    for release in data["releases"]:
        if not release["draft"]:
            tag = release["tag_name"]
            add(release["published_at"], "Releases", f"[{escaped(release['name'] or tag)}]({WEB}/releases/tag/{quote(tag, safe='')})", release["id"])
    lines = ["> 🤖 Generated by Aria's agent swarm, Dyna, not by Aria.", "",
             "# Changelog", "", f"Generated {now.strftime('%Y-%m-%d %H:%M UTC')}. Last {days} UTC calendar days.", "",
             "Public repository activity, not a curated release promise. CI/build and bot PRs without labels, and skip-release PRs are omitted. Rollups expand their membership tables. Issue labels and open status reflect generation time.", ""]
    for day in sorted(groups, reverse=True):
        lines += [f"## {day}", ""]
        for section in SECTIONS:
            entries = sorted(groups[day][section], reverse=True)
            if not entries:
                continue
            lines += [f"### {section}", ""]
            lines += ["- " + text for _, text in entries[:cap]]
            if len(entries) > cap:
                query = {"Merged": f"is:pr is:merged merged:{day}", "Releases": "", SECTIONS[2]: f'is:issue label:"help wanted" is:open created:{day}', SECTIONS[3]: f"is:issue label:bug is:closed closed:{day}"}[section]
                url = WEB + ("/releases" if section == "Releases" else "/issues?q=" + quote(query))
                lines.append(f"- [and {len(entries) - cap} more]({url})")
            lines.append("")
    if not groups:
        lines += ["No matching activity.", ""]
    return "\n".join(lines)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--days", type=int, choices=range(1, 31), default=30)
    parser.add_argument("--output", default="CHANGELOG.md")
    args = parser.parse_args()
    now = datetime.now(timezone.utc)
    Path(args.output).write_text(render(collect(now, args.days), now, args.days), encoding="utf-8")
