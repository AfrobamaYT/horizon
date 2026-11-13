"""The surface report step: which base it compares, when it passes, when it fails."""
import importlib.util
import os
from pathlib import Path
import shutil
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("surface_report", ROOT / "tools/ci/surface_report.py")
surface_report = importlib.util.module_from_spec(spec)
spec.loader.exec_module(surface_report)

MERGE = "tree t\nparent aaaa\nparent bbbb\nauthor x\n\nMerge bbbb into aaaa\n"
SINGLE = "tree t\nparent aaaa\nauthor x\n\nA commit\n"


class BaseCommitTests(unittest.TestCase):
    def test_a_pull_request_compares_against_its_merge_commits_first_parent(self):
        sha, _ = surface_report.base_commit("pull_request", "", lambda *a: MERGE, None)
        self.assertEqual(sha, "aaaa")
        sha, why = surface_report.base_commit("pull_request", "", lambda *a: SINGLE, None)
        self.assertIsNone(sha)
        self.assertIn("not a pull request's merge commit", why)

    def test_a_dispatch_asks_for_the_merge_base_of_the_pull_request(self):
        asked = []

        def api(path):
            asked.append(path)
            if path == "pulls/12":
                return {"base": {"ref": "main"}, "head": {"sha": "cccc"}}
            return {"merge_base_commit": {"sha": "dddd"}}

        sha, _ = surface_report.base_commit("workflow_dispatch", "12", None, api)
        self.assertEqual((sha, asked), ("dddd", ["pulls/12", "compare/main...cccc"]))

    def test_a_push_has_no_base(self):
        sha, why = surface_report.base_commit("push", "", None, None)
        self.assertIsNone(sha)
        self.assertEqual(why, "no base for a push run")


class GateTests(unittest.TestCase):
    def test_the_tracked_switch_is_read_and_must_be_a_boolean(self):
        self.assertIs(surface_report.gate(ROOT / "tools/ci/surface.toml"), False)
        with tempfile.TemporaryDirectory() as directory:
            config = Path(directory, "surface.toml")
            config.write_text("[surface]\ngate = true\n")
            self.assertIs(surface_report.gate(config), True)
            config.write_text("[surface]\ngate = \"yes\"\n")
            with self.assertRaises(SystemExit):
                surface_report.gate(config)


@unittest.skipUnless(shutil.which("bash"), "needs bash for the stub tool")
class ReportTests(unittest.TestCase):
    """report() against a stub horizon-stage whose behaviour each test sets."""

    def run_report(self, base_export=0, diff_exit=0, gate_on=False):
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            tool = work / "horizon-stage"
            tool.write_text(f"""#!/bin/bash
echo "$@" >> {work}/calls
if [ "$1" = surface ]; then
  case "$2" in *base) [ {base_export} = 0 ] || {{ echo 'unclassified: x' >&2; echo 'horizon-stage surface: unknown key `access`' >&2; exit 1; }};; esac
  echo '{{}}' > "$3"; exit 0
fi
echo "Surface: report"; [ {diff_exit} = 0 ] || echo "breaking change without a change note: fn:a" >&2
exit {diff_exit}
""")
            tool.chmod(0o755)
            (work / "base").mkdir()
            code, text = surface_report.report(str(tool), work / "base", work / "head", work, gate_on)
            return code, text, (work / "calls").read_text()

    def test_a_base_the_head_tool_cannot_read_passes_with_one_line(self):
        code, text, calls = self.run_report(base_export=1)
        self.assertEqual(code, 0)
        self.assertEqual(text, "No surface report: the head's horizon-stage cannot read the base tree "
                               "(horizon-stage surface: unknown key `access`).")
        self.assertNotIn("surface-diff", calls)

    def test_the_diff_decides_and_the_gate_is_passed_on(self):
        code, text, calls = self.run_report()
        self.assertEqual((code, text), (0, "Surface: report\n"))
        self.assertNotIn("--gate", calls)
        self.assertIn(f"--notes ", calls)
        code, text, calls = self.run_report(diff_exit=1, gate_on=True)
        self.assertEqual(code, 1)
        self.assertIn("fn:a", text)
        self.assertIn("--gate", calls)
        # An unreadable note (exit 2) fails too.
        code, _, _ = self.run_report(diff_exit=2)
        self.assertEqual(code, 1)


class SummaryTests(unittest.TestCase):
    def size(self, out: str) -> int:
        return len(out.encode())

    def test_a_long_report_is_cut_at_a_line_end(self):
        text = "".join(f"- fn:item{i} (added)\n" for i in range(20_000))
        out = surface_report.summary(text)
        self.assertLessEqual(self.size(out), surface_report.SUMMARY_LIMIT)
        self.assertRegex(out, r"\(added\)\n… cut here; \d+ more lines in the step log\n```\n$")
        self.assertEqual(surface_report.summary("short\n"), "### Mod API surface\n\n```text\nshort\n```\n")

    def test_a_cut_inside_a_multi_byte_character_keeps_whole_characters(self):
        # "é" is two bytes, so the byte budget is half the characters and lands inside one.
        for pad in ("", "x"):
            out = surface_report.summary(pad + "é" * 200_000 + "\nnext\n")
            self.assertLessEqual(self.size(out), surface_report.SUMMARY_LIMIT)
            self.assertGreater(self.size(out), surface_report.SUMMARY_LIMIT - 200)
            self.assertNotIn("\ufffd", out)
            self.assertIn("é\n… cut here; 2 more lines in the step log\n", out)

    def test_one_line_longer_than_the_cap_is_cut(self):
        out = surface_report.summary("a" * 300_000 + "\n")
        self.assertLessEqual(self.size(out), surface_report.SUMMARY_LIMIT)
        self.assertGreater(out.count("a"), 199_000)
        self.assertTrue(out.endswith("a\n… cut here; 1 more lines in the step log\n```\n"))

if __name__ == "__main__":
    unittest.main()
