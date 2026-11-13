import unittest

from hotspots_repo import Repo
from tools.hotspots import measure

LIST = "pub use a::{\n    A,\n    B,\n};\n"


def with_name(name):
    return LIST.replace("    B,\n", f"    B,\n    {name},\n")


class MeasureTest(unittest.TestCase):
    def setUp(self):
        self.repo = Repo()
        self.addCleanup(self.repo.close)
        r = self.repo
        r.commit("main", {"src/list.rs": LIST, "y.txt": "x\n", "other.txt": "o\n"}, "upstream's last commit")
        r.git("update-ref", "refs/remotes/upstream/main", "main")

    def run_measure(self, *extra):
        return measure.measure(measure.parse_args(["--repo", str(self.repo.path), "--main", "main", *extra]))

    def test_a_member_without_a_number_and_main_merged_into_a_pr_branch(self):
        r = self.repo
        pr1 = r.commit("pr1", {"src/list.rs": with_name("B1")}, base="main")
        pr2 = r.commit("carry-2", {"src/list.rs": with_name("B2")}, base="main")
        pr3 = r.commit("pr3", {"other.txt": "o3\n"}, base="main")
        r.commit("pr4", {"src/list.rs": with_name("B4")}, base="main")
        for n, sha in ((1, pr1), (3, pr3)):
            r.pull(n, sha)
        r.git("checkout", "-q", "-b", "fork-review-rollup-1", "main")
        r.merge("fork-review-rollup-1", "pr1", "Merge PR #1 into fork-review-rollup-1")
        r.merge("fork-review-rollup-1", "pr3", "Merge PR #3 into fork-review-rollup-1")
        r.merge("fork-review-rollup-1", "carry-2", "Merge fork-review-carry-2 into fork-review-rollup-1",
                resolve={"src/list.rs": LIST.replace("    B,\n", "    B,\n    B1,\n    B2,\n")})
        r.merge("main", "fork-review-rollup-1", "Merge pull request #10 from o/fork-review-rollup-1")
        r.merge("pr4", "main", "Merge branch 'main' into pr4",
                resolve={"src/list.rs": LIST.replace("    B,\n", "    B,\n    B1,\n    B2,\n    B4,\n")})
        r.pull(4, r.git("rev-parse", "pr4"))
        r.merge("main", "pr4", "Merge pull request #4 from o/pr4")

        git, raw = self.run_measure()
        entry = raw["files"]["src/list.rs"]
        self.assertEqual(entry["prs"], sorted(["#1", "#4", pr2[:12]]))
        c = measure.counts(entry)
        self.assertEqual((c["rollup"], c["branch"], c["sync"]), (1, 1, 0))
        self.assertEqual(raw["prs"], 4)
        body = measure.report(git, raw, min_rollup=1, min_prs=99)
        self.assertIn("### `src/list.rs`", body)
        self.assertIn("- `pub use a::{` (2)", body)
        self.assertNotIn("other.txt", body)

    def test_a_rollup_merged_into_the_next_counts_each_conflict_once(self):
        r = self.repo
        heads = {n: r.commit(f"p{n}", {"y.txt": f"{n}\n"}, base="main") for n in (21, 22, 24)}
        for n, sha in heads.items():
            r.pull(n, sha)
        r.git("checkout", "-q", "-b", "fork-review-rollup-1", "main")
        r.merge("fork-review-rollup-1", "p21", "Merge PR #21 into fork-review-rollup-1")
        r.merge("fork-review-rollup-1", "p22", "Merge PR #22 into fork-review-rollup-1", resolve={"y.txt": "21 22\n"})
        r.git("checkout", "-q", "-b", "fork-review-rollup-2", "main")
        r.merge("fork-review-rollup-2", "p24", "Merge PR #24 into fork-review-rollup-2")
        r.merge("fork-review-rollup-2", "fork-review-rollup-1", "Merge rollup 1 into fork-review-rollup-2",
                resolve={"y.txt": "21 22 24\n"})
        r.merge("main", "fork-review-rollup-2", "Merge pull request #30 from o/fork-review-rollup-2")

        _, raw = self.run_measure()
        entry = raw["files"]["y.txt"]
        self.assertEqual(entry["prs"], ["#21", "#22", "#24"])
        self.assertEqual(measure.counts(entry)["rollup"], 1)
        self.assertEqual(raw["nested_rollups"], 1)
        # main's merge, #24's, #21's and #22's
        self.assertEqual(raw["replayed"], 4)

    def test_the_window_leaves_out_older_merges(self):
        r = self.repo
        r.pull(5, r.commit("p5", {"other.txt": "5\n"}, base="main"))
        r.merge("main", "p5", "Merge pull request #5 from o/p5")
        r.tick(10 * 86400)
        r.pull(6, r.commit("p6", {"y.txt": "6\n"}, base="main"))
        r.merge("main", "p6", "Merge pull request #6 from o/p6")

        _, raw = self.run_measure("--days", "3")
        self.assertEqual(raw["prs"], 1)
        self.assertNotIn("other.txt", raw["files"])


if __name__ == "__main__":
    unittest.main()
