import contextlib
import io
import unittest

from hotspots_repo import Repo
from tools.hotspots import carry

OLD = "tools/ci/ignored-tests-known-failures.txt"
HEADER = "# PACKAGE TEST REFERENCE [policy]\n\n"


class DiffReadingTest(unittest.TestCase):
    def setUp(self):
        self.repo = Repo()
        self.addCleanup(self.repo.close)

    def test_names_added_to_each_brace_list(self):
        r = self.repo
        before = ("mod parts;\npub use parts::{Alpha, Beta};\n"
                  "pub mod __private {\n    pub(crate) use crate::parts::{gamma, delta as d};\n}\n")
        after = ("mod parts;\npub use parts::{\n    Alpha, Beta, // the two\n    Zeta,\n};\n"
                 "pub mod __private {\n    pub(crate) use crate::parts::{\n        delta as d, epsilon::{Eta, Theta},\n"
                 "        gamma,\n    };\n}\n")
        base = r.commit("main", {"x.rs": before})
        head = r.commit("pr", {"x.rs": after}, base="main")
        self.assertEqual(carry.added_names(r.path, base, head, "x.rs"), {
            ("", "pub", "parts::"): ["Zeta"],
            ("__private", "pub(crate)", "crate::parts::"): ["epsilon::Eta", "epsilon::Theta"],
        })

    def test_rows_added_to_a_file(self):
        r = self.repo
        base = r.commit("main", {"list.txt": "a 1\nb 2\nc 3\n"})
        head = r.commit("pr", {"list.txt": "a 1\nb 2 changed\nc 3\nd 4\n"}, base="main")
        found = [(h.removed, h.added) for h in carry.hunks(r.path, base, head, "list.txt")]
        self.assertEqual(found, [(["b 2"], ["b 2 changed"]), ([], ["d 4"])])


class IgnoredTestCarryTest(unittest.TestCase):
    def setUp(self):
        self.repo = Repo()
        self.addCleanup(self.repo.close)
        r = self.repo
        self.base = r.commit("main", {OLD: HEADER + "pkg a::flaky_one #1 flaky\npkg a::gone #2\n"})
        # The switch on main: the old file is gone, its rows live in topic files.
        r.git("rm", "-q", OLD)
        r.commit("main", {"tools/ci/ignored-tests/flaky.txt": "pkg a::flaky_one #1 flaky\n",
                          "tools/ci/ignored-tests/known.txt": "pkg a::gone #2\n"}, "switch")
        self.folder = r.path / "tools/ci/ignored-tests"

    def pr(self, rows):
        return self.repo.commit("pr", {OLD: HEADER + rows}, base=self.base)

    def test_new_rows_go_to_the_topic_file_and_a_changed_row_changes_where_it_lives(self):
        head = self.pr("pkg a::flaky_one #9 flaky\npkg a::gone #2\n# Needs the new fixture.\npkg b::new #7 gpu\n")
        self.repo.git("checkout", "-q", "main")
        done = carry.carry_ignored_test(self.repo.path, self.base, head, "new-fixture")
        self.assertEqual(sorted(done), ["tools/ci/ignored-tests/flaky.txt", "tools/ci/ignored-tests/new-fixture.txt"])
        self.assertEqual((self.folder / "new-fixture.txt").read_text(), "# Needs the new fixture.\npkg b::new #7 gpu\n")
        self.assertEqual((self.folder / "flaky.txt").read_text(), "pkg a::flaky_one #9 flaky\n")
        self.assertEqual((self.folder / "known.txt").read_text(), "pkg a::gone #2\n")

    def test_a_hunk_it_cannot_place_stops_before_writing(self):
        r = self.repo
        r.git("checkout", "-q", "main")
        r.git("rm", "-q", "tools/ci/ignored-tests/known.txt")
        r.commit("main", {"tools/ci/ignored-tests/flaky.txt": "pkg a::flaky_one #1 flaky\n"}, "gone passes")
        # The PR replaces a::gone, which no topic file lists any more, with c::added.
        head = self.pr("pkg a::flaky_one #1 flaky\npkg c::added #3\n")
        r.git("checkout", "-q", "main")
        with self.assertRaises(carry.CarryError) as e:
            carry.carry_ignored_test(r.path, self.base, head, "added")
        self.assertIn("-pkg a::gone #2", str(e.exception))
        self.assertFalse((self.folder / "added.txt").exists())

    def test_the_kinds_of_later_switches_say_which_pr_brings_them(self):
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            self.assertEqual(carry.main(["part-table", "3710", "--repo", str(self.repo.path)]), 2)
        self.assertIn("PR G", err.getvalue())


if __name__ == "__main__":
    unittest.main()
