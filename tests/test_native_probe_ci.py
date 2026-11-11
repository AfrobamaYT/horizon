import importlib.util
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "native_probe_ci", Path(__file__).resolve().parents[1] / "tools/native_probe_ci.py")
ci = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ci)


class SelectionTests(unittest.TestCase):
    def test_sdk_graph_and_boundaries(self):
        root = Path(__file__).resolve().parents[1]
        roots = ci.sdk_roots(root)
        for path in ("Cargo.lock", "Cargo.toml", "crates/new-crate/Cargo.toml",
                     "crates/horizon-mod-api-gen/src/main.rs", "crates/horizon-client/src/game.rs",
                     "crates/cargo-horizon-mod/template/native/lib.rs", "tools/native_dylib_probe/a.rs",
                     "launcher/src/rust_toolchain.rs", "rust-toolchain.toml"):
            with self.subTest(path=path):
                self.assertTrue(ci.selected([path], roots))
        for path in ("launcher/src/main.rs", "launcher/src/native_mods/plan.rs", "docs/native.md",
                     "tests/test_other.py", "crates/horizon-client/tests/other.rs",
                     "crates/horizon-client/examples/other.rs", "crates/horizon-client/README.md"):
            with self.subTest(path=path):
                self.assertFalse(ci.selected([path], roots))

    def test_discovers_transitive_build_target_dependencies_not_dev(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "Cargo.toml").write_text("")
            for name, manifest in {
                "horizon-mod-host": '[build-dependencies]\nb = {path="../b"}\n',
                "horizon-mod-api-gen": '',
                "b": '[target."cfg(windows)".dependencies]\nc = {path="../c"}\n[dev-dependencies]\nmissing = {path="../missing"}\n',
                "c": '',
            }.items():
                folder = root / "crates" / name
                folder.mkdir(parents=True)
                (folder / "Cargo.toml").write_text(manifest)
            self.assertIn("crates/c/", ci.sdk_roots(root))


class SummaryTests(unittest.TestCase):
    def test_baseline_filters(self):
        good = dict(id=1, run_number=1, event="push", head_branch="main",
                    head_repository={"full_name": "fork/repo"}, conclusion="success")
        runs = [good]
        for patch in ({"id": 9, "run_number": 9}, {"event": "pull_request"},
                      {"head_branch": "topic"}, {"head_repository": {"full_name": "upstream/repo"}},
                      {"conclusion": "failure"}):
            runs.append({**good, "id": 2, "run_number": 2, **patch})
        self.assertEqual(ci.baseline(runs, dict(id=9, run_number=9, repository="fork/repo")), good)
        self.assertIsNone(ci.baseline([], dict(id=9, run_number=9, repository="fork/repo")))

    def test_no_baseline_and_only_fork_merge_prs(self):
        self.assertIn("No successful", ci.summary("fork/repo", "head", None, [], lambda _: []))
        def pr(repo, number, sha="commit"):
            return dict(number=number, merge_commit_sha=sha, merged_at="date",
                        base={"ref": "main", "repo": {"full_name": repo}}, title="$(untrusted)")
        report = ci.summary("fork/repo", "head", dict(html_url="green", head_sha="base"),
                            [{"sha": "commit"}], lambda _: [pr("fork/repo", 42), pr("upstream/repo", 99),
                                                           pr("fork/repo", 43, "other")])
        self.assertIn("fork/repo/pull/42", report)
        self.assertNotIn("pull/99", report)
        self.assertNotIn("pull/43", report)
        self.assertNotIn("untrusted", report)
        self.assertIn("base...head", report)


class RecoveryTests(unittest.TestCase):
    def test_coalesced_sdk_push_then_readme_still_selects(self):
        root = Path(__file__).resolve().parents[1]
        # A was last green; B changed SDK and is replaced in the queue by C.
        changes = {"A": ["crates/horizon-client/src/game.rs", "crates/horizon-client/README.md"],
                   "B": ["crates/horizon-client/README.md"]}
        self.assertFalse(ci.selected(changes["B"], ci.sdk_roots(root)))
        self.assertTrue(ci.select_interval(root, {"head_sha": "A"}, changes.__getitem__))
        self.assertTrue(ci.select_interval(root, None, lambda _: self.fail("must not diff unknown baseline")))

    def test_history_ceiling_is_not_first_run(self):
        current = dict(id=20, run_number=20, repository="fork/repo")
        self.assertIsNone(ci.find_green(lambda _: {"total_count": 0, "workflow_runs": []}, None, current))
        with self.assertRaisesRegex(RuntimeError, "baseline unavailable"):
            ci.find_green(lambda _: {"total_count": 1001, "workflow_runs": []}, None, current)

    def test_known_range_survives_compare_or_pr_failure(self):
        green = dict(head_sha="A", html_url="green-url")
        for fail_compare in (True, False):
            output = []
            def pages(path, key=None):
                # Range must already be durable before the first request.
                self.assertIn("A...C", output[0])
                self.assertIn("green-url", output[0])
                if fail_compare or path.startswith("commits/"):
                    raise OSError("network unavailable")
                return [{"sha": "B"}]
            ci.write_failure("fork/repo", "C", green, pages, output.append)
            self.assertIn("lookup failed", output[-1])
            self.assertNotIn("No successful", "\n".join(output))


if __name__ == "__main__":
    unittest.main()
