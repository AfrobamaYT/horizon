import tempfile
import tomllib
import unittest
from pathlib import Path

from tools import sdk_api_source as api_source
from tools.sdk_api_source import Crate, Refusal


def package(name, directory=None, kinds=("lib",), dependencies=(), source=None):
    return {
        "id": name, "name": name, "version": "0.1.0", "source": source,
        "manifest_path": f"/w/{directory or 'crates/' + name}/Cargo.toml",
        "targets": [{"kind": list(kinds), "name": name}],
        "dependencies": [{"name": dependency, "kind": kind, "path": path} for dependency, kind, path in dependencies],
    }


def node(name, *edges):
    return {"id": name, "deps": [{"pkg": dependency, "dep_kinds": [{"kind": kind}]} for dependency, kind in edges]}


def metadata():
    """An API crate whose build script runs the generator, a vocabulary crate, a proc macro with a helper crate,
    a registry crate and a dev-only workspace crate."""
    return {
        "workspace_root": "/w",
        "packages": [
            package("horizon-mod-api", kinds=("lib", "custom-build"),
                    dependencies=[("horizon-mod-api-gen", "build", "/w/crates/horizon-mod-api-gen"),
                                  ("helper", "dev", "/w/crates/helper")]),
            package("vocab", "crates/nested/vocab"),
            package("macros", kinds=("proc-macro",)),
            package("shape"),
            package("horizon-mod-api-gen"),
            package("discovery"),
            package("helper"),
            package("serde", source="registry+https://github.com/rust-lang/crates.io-index"),
        ],
        "resolve": {"nodes": [
            node("horizon-mod-api", ("vocab", None), ("macros", None), ("serde", None),
                 ("horizon-mod-api-gen", "build"), ("helper", "dev")),
            node("vocab", ("serde", None)),
            node("macros", ("shape", None)),
            node("shape"),
            node("horizon-mod-api-gen", ("discovery", None)),
            node("discovery"),
            node("helper"),
            node("serde"),
        ]},
    }


class ClosureTests(unittest.TestCase):
    def test_normal_and_proc_macro_dependencies_only(self):
        self.assertEqual(api_source.closure(metadata()), [
            Crate("horizon-mod-api", "crates/horizon-mod-api", True),
            Crate("macros", "crates/macros", False),
            Crate("shape", "crates/shape", False),
            Crate("vocab", "crates/nested/vocab", False),
        ])

    def test_a_build_script_that_is_not_the_generator_is_refused(self):
        data = metadata()
        data["packages"][3]["targets"][0]["kind"].append("custom-build")
        with self.assertRaises(Refusal) as refused:
            api_source.closure(data)
        self.assertEqual(refused.exception.code, "E_BUILD_SCRIPT")


class ManifestTests(unittest.TestCase):
    WORKSPACE = {"workspace": {
        "package": {"edition": "2024", "license": "MIT", "rust-version": "1.95"},
        "dependencies": {"glam": {"version": "0.30", "features": ["serde"]}},
        "lints": {"clippy": {"type_complexity": "allow"}},
    }}
    API = {
        "package": {"name": "horizon-mod-api", "version": "0.1.0", "edition": {"workspace": True},
                    "license": {"workspace": True}, "publish": False},
        "lib": {"name": "mod_api"},
        "dependencies": {"vocab": {"path": "../nested/vocab"}, "log": "0.4",
                         "glam": {"workspace": True, "features": ["bytemuck"]}},
        "target": {"cfg(unix)": {"dependencies": {"libc": "0.2"}}},
        "build-dependencies": {"horizon-mod-api-gen": {"path": "../horizon-mod-api-gen"}},
        "dev-dependencies": {"helper": {"path": "../helper"}},
        # Declared before what it names, so dropping it needs a second pass.
        "features": {"chained": ["fixture"], "testing": ["vocab/testing"],
                     "fixture": ["horizon-mod-api-gen/fixture", "vocab/fixture"]},
        "lints": {"workspace": True},
        "bin": [{"name": "tool", "path": "src/bin/tool.rs"}],
    }
    VOCAB = {
        "package": {"name": "vocab", "version": "0.1.0"},
        "dependencies": {},
        "features": {"testing": [], "fixture": ["horizon-mod-api-gen/fixture"]},
    }

    def test_features_naming_a_dropped_dependency_go_everywhere_they_reach(self):
        kept = api_source.kept_features({"horizon-mod-api": self.API, "vocab": self.VOCAB})
        self.assertEqual(kept, {"horizon-mod-api": {"testing": ["vocab/testing"]}, "vocab": {"testing": []}})

    def test_manifest_keeps_the_library_and_its_normal_dependencies(self):
        crate = Crate("horizon-mod-api", "crates/horizon-mod-api", True)
        crates = {"horizon-mod-api": crate, "vocab": Crate("vocab", "crates/nested/vocab", False)}
        written = api_source.normalise_manifest(self.API, self.WORKSPACE, crate, crates,
                                                {"testing": ["vocab/testing"]})
        self.assertEqual(tomllib.loads(written), {
            "package": {"name": "horizon-mod-api", "version": "0.1.0", "edition": "2024", "license": "MIT",
                        "publish": False, "autobins": False, "autoexamples": False, "autotests": False,
                        "autobenches": False, "build": "build.rs"},
            "lib": {"name": "mod_api"},
            "dependencies": {"vocab": {"path": "../nested/vocab"}, "log": "0.4",
                             "glam": {"version": "0.30", "features": ["bytemuck", "serde"]}},
            "target": {"cfg(unix)": {"dependencies": {"libc": "0.2"}}},
            "features": {"testing": ["vocab/testing"]},
            "lints": {"clippy": {"type_complexity": "allow"}},
        })

    def test_a_path_dependency_outside_the_api_source_is_refused(self):
        crate = Crate("vocab", "crates/nested/vocab", False)
        manifest = {"package": {"name": "vocab", "version": "0.1.0"},
                    "dependencies": {"horizon-items": {"path": "../../horizon-items"}}}
        with self.assertRaises(Refusal) as refused:
            api_source.normalise_manifest(manifest, self.WORKSPACE, crate, {"vocab": crate}, {})
        self.assertEqual(refused.exception.code, "E_PATH_DEPENDENCY")


class PackingTests(unittest.TestCase):
    CRATES = [Crate("horizon-mod-api", "crates/horizon-mod-api", True), Crate("core", "crates/core", False)]
    TRACKED = {"crates/horizon-mod-api/src/lib.rs", "crates/core/src/lib.rs"}

    def files(self):
        return {
            "Cargo.toml": b"[workspace]\n",
            "Cargo.lock": b"version = 4\n",
            "crates/horizon-mod-api/Cargo.toml": b"",
            "crates/horizon-mod-api/build.rs": b"fn main() {}\n",
            "crates/horizon-mod-api/generated/hooks.rs": b'#[path = "{horizon-api-source-root}/x.rs"] mod x;\n',
            "crates/horizon-mod-api/src/lib.rs": b"",
            "crates/core/Cargo.toml": b"",
            "crates/core/src/lib.rs": b"",
        }

    def check(self, files):
        return api_source.check_files(files, self.CRATES, self.TRACKED, ["/home/dev/horizon"])

    def refusal(self, files):
        with self.assertRaises(Refusal) as refused:
            self.check(files)
        return refused.exception.code

    def test_the_list_and_index_do_not_depend_on_the_order_files_were_found(self):
        files = self.files()
        reversed_files = dict(reversed(list(files.items())))
        self.assertEqual(self.check(files), self.check(reversed_files))
        self.assertEqual(self.check(files)[:3], ["Cargo.lock", "Cargo.toml", "crates/core/Cargo.toml"])
        self.assertEqual(api_source.packing_index(files), api_source.packing_index(reversed_files))

    def test_a_file_naming_a_local_folder_is_refused(self):
        files = self.files()
        files["crates/horizon-mod-api/generated/hooks.rs"] = b'#[path = "/home/dev/horizon/crates/x.rs"] mod x;\n'
        self.assertEqual(self.refusal(files), "E_ABSOLUTE_PATH")

    def test_stray_files_are_refused(self):
        for stray in (".local/minecraft/client.jar", "crates/core/src/untracked.rs", "crates/core/build.rs",
                      "crates/core/generated/hooks.rs", "crates/horizon-mod-api/generated/sub/hooks.rs",
                      "crates/other/src/lib.rs", "/etc/passwd", "crates/core/../../secret"):
            with self.subTest(stray):
                self.assertIn(self.refusal({**self.files(), stray: b""}), ("E_STRAY_FILE", "E_PATH"))


class RelocationTests(unittest.TestCase):
    def test_both_ways_the_build_script_names_the_checkout_become_the_token(self):
        crate = Crate("vocab", "crates/vocab", True)
        forms = api_source.root_forms([Path("/w")], crate)
        code = '#[path = "/w/crates/vocab/../../crates/vocab/src/a.rs"] mod a; include!("/w/crates/vocab/src/b.rs");'
        self.assertEqual(api_source.relocate(code, forms),
                         '#[path = "{horizon-api-source-root}/crates/vocab/src/a.rs"] mod a; '
                         'include!("{horizon-api-source-root}/crates/vocab/src/b.rs");')

    def test_dep_info_keeps_escaped_spaces(self):
        with tempfile.TemporaryDirectory() as folder:
            info = Path(folder) / "x.d"
            info.write_text("/t/libx.rmeta: src/lib.rs src/my\\ file.rs /o/gen.rs\n\nsrc/lib.rs:\n")
            self.assertEqual(api_source.dep_info(info), ["src/lib.rs", "src/my file.rs", "/o/gen.rs"])


class LockTests(unittest.TestCase):
    def test_a_version_the_game_does_not_pin_is_named(self):
        registry = "registry+https://github.com/rust-lang/crates.io-index"
        game = {"package": [{"name": "serde", "version": "1.0.228", "source": registry, "checksum": "a"},
                            {"name": "horizon-core", "version": "0.1.0"}]}
        ours = {"package": [{"name": "serde", "version": "1.0.229", "source": registry, "checksum": "b"},
                            {"name": "horizon-core", "version": "0.1.0"}]}
        self.assertEqual(api_source.lock_disagreements(ours, game), ["serde 1.0.229"])
        self.assertEqual(api_source.lock_disagreements(game, game), [])


if __name__ == "__main__":
    unittest.main()
