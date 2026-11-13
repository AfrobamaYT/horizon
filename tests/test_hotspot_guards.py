"""tools/hotspots/check_guards.py: each guard fails on growth and lets what the base already has stand."""
import io
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from unittest.mock import patch

with patch.object(sys, "path", [*sys.path, str(Path(__file__).resolve().parents[1] / "tools/hotspots")]):
    import check_guards as guards


def summary(*lines):
    return "\n".join(["# Summary", "", "# Concepts", "", *lines[:2], "", "# How-to", "", *lines[2:], ""])


class SummaryGuard(unittest.TestCase):
    def test_a_new_line_for_a_discovered_page_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp)
            for page in ("concepts/listed.md", "concepts/new.md", "concepts/listed/sub.md", "how-to/kept.md",
                         "other/page.md"):
                (src / page).parent.mkdir(parents=True, exist_ok=True)
                (src / page).write_text(f"# {Path(page).stem}\n")
            before = summary("- [Listed](concepts/listed.md)")
            now = summary("- [Listed](concepts/listed.md)", "  - [Sub](concepts/listed/sub.md)",
                          "- [New](concepts/new.md)", "- [Other](other/page.md)")
            found = guards.summary_problems(guards.redundant_summary_lines(now, src),
                                            guards.redundant_summary_lines(before, src))
        self.assertEqual([p.split(" lists ")[0] for p in found],
                         [f"{guards.SUMMARY}: the new line for concepts/listed/sub.md",
                          f"{guards.SUMMARY}: the new line for concepts/new.md"])


class LangGuard(unittest.TestCase):
    def test_more_than_five_language_files_fail_unless_the_change_is_a_sync(self):
        six = [f"launcher/lang/{code}.json" for code in ("de", "fr", "es", "it", "ja", "pl")]
        self.assertEqual(len(guards.lang_problems(six, lambda: False)), 1)
        self.assertEqual(guards.lang_problems(six, lambda: True), [])
        self.assertEqual(guards.lang_problems(six[:5], lambda: self.fail("only asked over the limit")), [])


class NotesGuard(unittest.TestCase):
    def git(self, *args):
        subprocess.run(["git", *args], cwd=self.root, check=True, capture_output=True)

    def test_notes_may_not_grow_past_the_merge_base(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.root = Path(tmp)
            (self.root / guards.BOOK_SRC / "concepts").mkdir(parents=True)
            (self.root / guards.BOOK_SRC / "how-to").mkdir()
            (self.root / guards.SUMMARY).write_text(summary())
            notes = self.root / guards.NOTES
            notes.parent.mkdir(parents=True)
            notes.write_text("pub trait Notes {}\n")
            self.git("init", "-q", "-b", "main")
            self.git("add", ".")
            self.git("-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "base")
            self.git("switch", "-qc", "branch")
            run = lambda: guards.main(["--base", "main"], root=self.root)  # noqa: E731
            with redirect_stdout(io.StringIO()) as out:
                self.assertEqual(run(), 0)
                notes.write_text("pub trait Notes {}\n// More prose.\n")
                self.assertEqual(run(), 1)
        self.assertIn(f"{guards.NOTES}: 2 lines, 1 on the base", out.getvalue())


if __name__ == "__main__":
    unittest.main()
