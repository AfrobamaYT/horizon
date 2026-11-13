#!/usr/bin/env python3
"""Carry an old-style PR's addition to a hand-kept list into the list's new form, for a rollup worker.

After a switch (tools/hotspots/README.md), a member PR written against the old list conflicts on it in the rollup.
The worker takes main's side of the old file, then runs this: it reads the PR's own diff of the OLD file
(`diff(merge-base(main, head), head)`) and writes the PR's addition in the new form into the working tree. It does
not commit; the worker commits the result as "Carry for #N". Where it cannot place a hunk, it stops before writing
anything and prints the hunk.

Kinds:
- ignored-test: rows of tools/ci/ignored-tests-known-failures.txt go to tools/ci/ignored-tests/<topic>.txt, same
  row format; the topic is --topic or the PR's branch name (from `gh`). A changed or removed row is changed or
  removed where it lives now.
- part-table: available with PR G (exports/part_table.rs split per parts file).
- dispatch-stub: available with PR E (dispatch-test stand-ins, one file each).

    python3 tools/hotspots/carry.py ignored-test 3788
    python3 tools/hotspots/carry.py ignored-test 3788 --head pr/3788 --topic attribute-bounds
"""
import argparse
from dataclasses import dataclass, field
from pathlib import Path
import re
import subprocess
import sys


class CarryError(Exception):
    """A carry that cannot be written; the message says why and shows the hunk."""


def git(repo, *args, ok=(0,)):
    p = subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True)
    if p.returncode not in ok:
        raise CarryError(f"git {' '.join(args)}: {p.stderr.strip()}")
    return p.returncode, p.stdout


def pr_range(repo, number, head=None, main="origin/main"):
    """(merge base with main, head) of a PR; the head is `refs/pull/<n>/head` unless given."""
    ref = head or f"refs/pull/{number}/head"
    rc, sha = git(repo, "rev-parse", "--verify", "--quiet", f"{ref}^{{commit}}", ok=(0, 1))
    if rc:
        raise CarryError(f"{ref} is not fetched: git fetch origin pull/{number}/head:refs/pull/{number}/head, "
                         "or pass --head")
    sha = sha.strip()
    base = git(repo, "merge-base", main, sha)[1].strip()
    return base, sha


# ---- diff reading -------------------------------------------------------------------------------------------------

@dataclass
class Hunk:
    header: str
    removed: list = field(default_factory=list)
    added: list = field(default_factory=list)

    def text(self):
        return "\n".join([self.header] + ["-" + x for x in self.removed] + ["+" + x for x in self.added])


def hunks(repo, base, head, path):
    """The hunks of the PR's diff of one file, without context lines."""
    out = git(repo, "diff", "--no-renames", "-U0", base, head, "--", path)[1]
    found = []
    for line in out.splitlines():
        if line.startswith("@@"):
            found.append(Hunk(line))
        elif found and line.startswith("-") and not line.startswith("---"):
            found[-1].removed.append(line[1:])
        elif found and line.startswith("+") and not line.startswith("+++"):
            found[-1].added.append(line[1:])
    return found


def show(repo, rev, path):
    rc, text = git(repo, "show", f"{rev}:{path}", ok=(0, 128))
    return text if rc == 0 else ""


USE = re.compile(r"\b((?:pub(?:\s*\([^)]*\))?\s+)?)use\s+([^;]*);")
MOD_OPEN = re.compile(r"\bmod\s+(\w+)\s*\{")


def strip_comments(source):
    return re.sub(r"//[^\n]*|/\*.*?\*/", "", source, flags=re.S)


def use_leaves(tree, prefix=""):
    """Leaf paths of a use tree: `a::{b, c::{d as e}}` -> ['a::b', 'a::c::d as e']."""
    tree = tree.strip()
    i = tree.find("{")
    if i < 0:
        return [prefix + tree] if tree else []
    head, body = tree[:i], tree[i + 1:tree.rindex("}")]
    items, depth, start = [], 0, 0
    for j, ch in enumerate(body):
        depth += ch == "{"
        depth -= ch == "}"
        if ch == "," and depth == 0:
            items.append(body[start:j])
            start = j + 1
    items.append(body[start:])
    leaves = []
    for item in items:
        leaves += use_leaves(item, prefix + head.strip())
    return [re.sub(r"\s+", " ", x) for x in leaves]


def brace_lists(source):
    """{(scope, visibility, path before the brace): leaf names} for every braced `use` of a Rust source.

    The scope is the chain of inline `mod name {` blocks around the statement, joined with `::`."""
    source = strip_comments(source)
    lists = {}
    stack = []            # (module name or None, depth at which it opened)
    depth = 0
    pos = 0
    events = sorted([(m.start(), "use", m) for m in USE.finditer(source)]
                    + [(m.end() - 1, "mod", m) for m in MOD_OPEN.finditer(source)], key=lambda e: e[0])
    for at, kind, m in events + [(len(source), "end", None)]:
        for ch in source[pos:at]:
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
                while stack and stack[-1][1] > depth:
                    stack.pop()
        pos = at
        if kind == "mod":
            stack.append((m.group(1), depth + 1))
        elif kind == "use":
            tree = m.group(2)
            if "{" in tree:
                key = ("::".join(s for s, _ in stack), re.sub(r"\s+", "", m.group(1)), tree[:tree.find("{")].strip())
                names = [x[len(key[2]):] for x in use_leaves(tree)]
                lists.setdefault(key, []).extend(names)
            pos = m.end()   # the use's own braces are not module braces
    return lists


def added_names(repo, base, head, path):
    """{(scope, visibility, path): [names]} the PR added to the brace lists of one Rust file."""
    before = brace_lists(show(repo, base, path))
    after = brace_lists(show(repo, head, path))
    added = {}
    for key, names in after.items():
        old = set(before.get(key, ()))
        new = [n for n in names if n not in old]
        if new:
            added[key] = new
    return added


def is_row(line):
    return bool(line.strip()) and not line.lstrip().startswith("#")


# ---- kinds --------------------------------------------------------------------------------------------------------

IGNORED_OLD = "tools/ci/ignored-tests-known-failures.txt"
IGNORED_DIR = "tools/ci/ignored-tests"


def row_key(row):
    return tuple(row.split(" # ", 1)[0].split()[:2])


def carry_ignored_test(repo, base, head, topic):
    """Writes the PR's rows into tools/ci/ignored-tests/<topic>.txt; returns what it did."""
    folder = Path(repo) / IGNORED_DIR
    if not folder.is_dir():
        raise CarryError(f"{IGNORED_DIR}/ does not exist on this tree: the ignored-test switch (PR D) has not landed")
    if not topic:
        raise CarryError("no topic: pass --topic")
    files = {p: p.read_text(encoding="utf-8").splitlines() for p in sorted(folder.glob("*.txt"))}

    def where(key):
        return [(p, i) for p, lines in files.items() for i, x in enumerate(lines) if is_row(x) and row_key(x) == key]

    target = folder / f"{topic}.txt"
    edits, appended, stuck = [], [], []
    for hunk in hunks(repo, base, head, IGNORED_OLD):
        added = {row_key(x): x for x in hunk.added if is_row(x)}
        changes, new_rows, reason = [], [], None
        for row in filter(is_row, hunk.removed):
            places = where(row_key(row))
            if len(places) != 1:
                reason = f"the removed row is in {len(places)} files under {IGNORED_DIR}/, not one"
                break
            changes.append((*places[0], added.pop(row_key(row), None)))
        for key, row in added.items():
            places = where(key)
            if places and any(files[p][i] != row for p, i in places):
                reason = "an added row's test is already listed with a different row"
            elif not places:
                new_rows.append(row)
        if reason:
            stuck.append(f"{reason}:\n{hunk.text()}")
            continue
        edits += changes
        if new_rows:
            appended += [x for x in hunk.added if x.strip() and not is_row(x)] + new_rows
    if stuck:
        raise CarryError("cannot place these hunks; carry them by hand:\n\n" + "\n\n".join(stuck))
    for path, index, row in edits:
        files[path][index] = row
    if appended:
        files.setdefault(target, [])
        files[target] = files[target] + appended
    done = []
    for path, lines in files.items():
        new = [x for x in lines if x is not None]
        text = "\n".join(new) + "\n" if new else ""
        if not path.exists() or path.read_text(encoding="utf-8") != text:
            if text:
                path.write_text(text, encoding="utf-8")
            else:
                path.unlink()
            done.append(path.relative_to(repo).as_posix())
    return done


KINDS = {
    "ignored-test": carry_ignored_test,
    "part-table": "available with PR G, which splits exports/part_table.rs into one fragment per parts file",
    "dispatch-stub": "available with PR E, which gives each dispatch-test stand-in its own file",
}


def topic_from_branch(branch):
    return re.sub(r"[^a-z0-9]+", "-", branch.rsplit("/", 1)[-1].lower()).strip("-")


def branch_of(number):
    p = subprocess.run(["gh", "pr", "view", str(number), "--json", "headRefName", "--jq", ".headRefName"],
                       capture_output=True, text=True)
    return p.stdout.strip() if p.returncode == 0 else ""


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    ap.add_argument("kind", choices=sorted(KINDS))
    ap.add_argument("pr", type=int)
    ap.add_argument("--head", help="the PR's head (default refs/pull/<PR>/head)")
    ap.add_argument("--main", default="origin/main")
    ap.add_argument("--topic", help="ignored-test: the topic file's name (default: from the PR's branch name)")
    ap.add_argument("--repo", default=".")
    args = ap.parse_args(argv)
    run = KINDS[args.kind]
    if isinstance(run, str):
        print(f"carry.py {args.kind}: {run}.", file=sys.stderr)
        return 2
    try:
        repo = Path(git(args.repo, "rev-parse", "--show-toplevel")[1].strip())
        base, head = pr_range(repo, args.pr, args.head, args.main)
        topic = args.topic or topic_from_branch(branch_of(args.pr))
        done = run(repo, base, head, topic)
    except CarryError as e:
        print(f"carry.py {args.kind} #{args.pr}: {e}", file=sys.stderr)
        return 1
    if done:
        print("written (not committed): " + ", ".join(done))
    else:
        print(f"nothing to carry: #{args.pr} adds no row the tree lacks")
    return 0


if __name__ == "__main__":
    sys.exit(main())
