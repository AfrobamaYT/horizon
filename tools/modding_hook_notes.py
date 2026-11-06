#!/usr/bin/env python3
"""mdBook preprocessor: render catalog notes without writing a generated source page."""
import json
from pathlib import Path
import subprocess
import sys

PAGE = "reference/hook-notes.md"


def expand(book, content):
    """Fill the draft chapter from SUMMARY, retaining its published URL and position."""
    found = []

    def visit(items):
        for item in items:
            if "Chapter" not in item:
                continue
            chapter = item["Chapter"]
            if chapter["name"] == "Hook notes" and chapter["path"] is None:
                chapter.update(content=content, path=PAGE, source_path=None)
                found.append(chapter)
            visit(chapter["sub_items"])

    visit(book["sections"])
    assert len(found) == 1, "SUMMARY must contain one draft [Hook notes]() chapter"
    return book


def preprocess(context, book):
    root = Path(context["root"]).resolve().parents[1]
    content = subprocess.run(
        ["cargo", "run", "--quiet", "-p", "xtask", "--", "mod-api", "notes"],
        cwd=root, check=True, stdout=subprocess.PIPE, text=True,
    ).stdout
    return expand(book, content)


if __name__ == "__main__" and sys.argv[1:2] != ["supports"]:
    print(json.dumps(preprocess(*json.load(sys.stdin))))
