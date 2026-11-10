import contextlib
import io
import struct
import tempfile
import unittest
from unittest.mock import patch
from pathlib import Path

from tools import prebuilt_game


class AllocatorSelectionTests(unittest.TestCase):
    def test_windows_archives_use_the_system_allocator(self):
        self.assertEqual(prebuilt_game.cargo_features(False, "win32"), ["--features", "system-allocator"])
        self.assertEqual(prebuilt_game.cargo_features(True, "win32"),
                         ["--no-default-features", "--features", "system-allocator"])

    def test_other_systems_keep_mimalloc(self):
        for platform in ("linux", "darwin"):
            self.assertEqual(prebuilt_game.cargo_features(False, platform), [])
            self.assertEqual(prebuilt_game.cargo_features(True, platform), ["--no-default-features"])

    def test_build_uses_explicit_host_target_and_target_dynamic_flags(self):
        class Captured(Exception):
            pass

        def capture(command, **kwargs):
            self.assertEqual(command[command.index("--target") + 1], "fixture-host")
            self.assertIn("-C prefer-dynamic", kwargs["env"]["RUSTFLAGS"])
            self.assertNotIn("CARGO_ENCODED_RUSTFLAGS", kwargs["env"])
            raise Captured

        with patch.object(prebuilt_game, "stage", return_value=Path("staged")), \
             patch.object(prebuilt_game, "rustc_host", return_value="fixture-host"), \
             patch.object(prebuilt_game, "run", side_effect=capture), \
             patch.dict(prebuilt_game.os.environ, {"CARGO_ENCODED_RUSTFLAGS": "-Cprefer-dynamic=no"}):
            with self.assertRaises(Captured):
                prebuilt_game.build(Path("unused-sdk"), False)


class PrebuiltContentsTests(unittest.TestCase):
    def folder(self, *names: str) -> Path:
        root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        for name in names:
            (root / name).parent.mkdir(parents=True, exist_ok=True)
            (root / name).write_bytes(b"")
        return root

    def test_only_target_artifacts_and_host_proc_macros_enter_the_archive(self):
        root = self.folder(
            "cache/release/deps/libshared-host.rmeta",
            "cache/release/deps/libderive.so",
            "cache/host/release/deps/libshared-target.rmeta",
            "cache/host/release/deps/libshared-second-version.rmeta",
            "cache/host/release/deps/libshared-target.rlib",
            "cache/host/release/deps/libhorizon_client.so",
        )
        output, macros = root / "sdk/deps", root / "sdk/host"
        output.mkdir(parents=True)
        macros.mkdir()
        def artifact(path, kind):
            return {"reason": "compiler-artifact", "target": {"kind": [kind]},
                    "filenames": [str(root / path)]}
        prebuilt_game.copy_artifacts([
            artifact("cache/release/deps/libshared-host.rmeta", "lib"),
            artifact("cache/release/deps/libderive.so", "proc-macro"),
            artifact("cache/host/release/deps/libshared-target.rmeta", "lib"),
            artifact("cache/host/release/deps/libshared-second-version.rmeta", "lib"),
            artifact("cache/host/release/deps/libshared-target.rlib", "lib"),
            artifact("cache/host/release/deps/libhorizon_client.so", "dylib"),
            {"reason": "build-finished", "success": True},
        ], output, macros, root / "cache/host")
        self.assertEqual({path.name for path in output.iterdir()}, {
            "libshared-target.rmeta", "libshared-second-version.rmeta", "libhorizon_client.so",
        })
        self.assertEqual({path.name for path in macros.iterdir()}, {"libderive.so"})
        # Multiple legitimate target versions remain available to rustc's transitive lookup.
        # Only explicit extern/facade lookup must reject ambiguous names.
        with self.assertRaises(ValueError):
            prebuilt_game.extern_args(output, "shared")

    def test_facades_exclude_build_only_and_ambiguous_runtime_crates(self):
        def package(name, kind="lib"):
            return {"id": name, "name": name, "targets": [{"name": name.replace('-', '_'), "kind": [kind]}]}

        def node(name, *dependencies):
            return {"id": name, "deps": [
                {"pkg": target, "dep_kinds": [{"kind": kind}]} for target, kind in dependencies
            ]}

        metadata = {
            "packages": [package("horizon-client"), package("point_shape"), package("horizon_core"),
                         package("build_tool"), package("derive", "proc-macro"), package("macro_helper")],
            "resolve": {"nodes": [
                node("horizon-client", ("point_shape", None), ("horizon_core", None),
                     ("build_tool", "build"), ("derive", None)),
                node("point_shape"), node("horizon_core"), node("build_tool"),
                node("derive", ("macro_helper", None)), node("macro_helper"),
            ]},
        }
        runtime = prebuilt_game.runtime_crates(metadata)
        self.assertEqual(runtime, {"horizon_client", "point_shape", "horizon_core"})
        deps = self.folder("libpoint_shape-host.rmeta", "libpoint_shape-target.rmeta",
                           "libhorizon_core-one.rmeta", "libbuild_tool-one.rmeta", "libmacro_helper-one.rmeta")
        eligible = prebuilt_game.facade_crates(deps, runtime)
        self.assertEqual(eligible, ["horizon_core"])

    def test_sdk_assertion_refuses_a_duplicate_eligible_facade_on_every_os(self):
        root = self.folder("deps/libhorizon_core-one.rmeta", "deps/libmod_api-one.rmeta",
                           "deps/libhorizon_mod_package-one.rmeta")
        prebuilt_game.assert_externs(root, ["horizon_core"])
        (root / "deps/libhorizon_core-two.rmeta").write_bytes(b"")
        with self.assertRaisesRegex(ValueError, "expected exactly one SDK metadata for horizon_core"):
            prebuilt_game.assert_externs(root, ["horizon_core"])

    def test_native_extern_lookup_refuses_a_second_package_variant(self):
        root = self.folder("libhorizon_mod_package-runtime.rmeta")
        self.assertEqual(prebuilt_game.extern_args(root, "horizon_mod_package"), [
            "--extern", f"horizon_mod_package={root / 'libhorizon_mod_package-runtime.rmeta'}",
        ])
        (root / "libhorizon_mod_package-build.rmeta").write_bytes(b"")
        with self.assertRaisesRegex(ValueError, "expected exactly one SDK metadata for horizon_mod_package"):
            prebuilt_game.extern_args(root, "horizon_mod_package")

    def test_host_macros_pass_but_host_dependency_metadata_is_refused(self):
        root = self.folder("deps/libhorizon_client.so", "deps/libhorizon_mod_wasm.so",
                           "host/libderive.so", "host/derive.dll", "host/libderive.dylib")
        prebuilt_game.check(root)
        (root / "host/libshared-host.rmeta").write_bytes(b"")
        with self.assertRaisesRegex(SystemExit, "aren't compiler outputs"):
            prebuilt_game.check(root)

    def test_unrelated_multiversion_dependencies_do_not_break_named_externs(self):
        root = self.folder("deps/libmod_api-one.rmeta", "deps/libhorizon_mod_package-one.rmeta",
                           "deps/libshared-v1.rmeta", "deps/libshared-v2.rmeta",
                           "host/libmod_api-host.so")
        prebuilt_game.assert_externs(root, [])

    def test_compiler_outputs_pass(self):
        prebuilt_game.check(self.folder(
            "horizon-prebuilt.json", "Cargo.lock", "native_main.rs.template", "deps/libhorizon_client.so", "deps/libmod_api-1.rmeta",
            "native/libwindows.0.52.0.a", "points.json", "internals.json",
            "deps/libserde_derive-2.so", "deps/libhorizon_client.dll.a", "deps/libhorizon_mod_wasm.so",
            "deps/horizon_mod_wasm.dll", "deps/libhorizon_mod_wasm.dylib",
        ))

    def test_anything_else_is_refused(self):
        for stray in ("assets/minecraft/lang/en_us.json", "deps/Cargo.lock", "deps/client.jar", "deps/libmod_api-3.rlib", "native/libfoo.so", "deps/internals.json", "notes.txt"):
            with self.subTest(stray), self.assertRaises(SystemExit):
                prebuilt_game.check(self.folder("deps/libhorizon_client.so", "deps/libhorizon_mod_wasm.so", stray))

    def test_a_folder_without_either_runtime_is_refused(self):
        for present, missing in (("horizon_client", "horizon-mod-wasm"), ("horizon_mod_wasm", "horizon-client")):
            with self.subTest(present), self.assertRaisesRegex(SystemExit, missing):
                prebuilt_game.check(self.folder(f"deps/lib{present}.so"))

    def dll(self, root: Path, name: str, count: int, pe64: bool = True) -> None:
        # One PE section: RVA 0x1000 maps to raw offset 0x200, holding the export directory.
        data = bytearray(0x400)
        data[:2] = b"MZ"
        struct.pack_into("<I", data, 0x3C, 0x80)
        data[0x80:0x84] = b"PE\0\0"
        struct.pack_into("<H", data, 0x86, 1)  # NumberOfSections
        optional_size = 240 if pe64 else 224
        struct.pack_into("<H", data, 0x94, optional_size)
        struct.pack_into("<H", data, 0x98, 0x20B if pe64 else 0x10B)
        struct.pack_into("<II", data, 0x98 + (112 if pe64 else 96), 0x1000, 40)
        struct.pack_into("<III", data, 0x98 + optional_size + 12, 0x1000, 0x200, 0x200)
        struct.pack_into("<I", data, 0x200 + 24, count)  # NumberOfNames
        (root / "deps" / name).write_bytes(data)

    def test_exports_reports_every_dll(self):
        root = self.folder("deps/libhorizon_client.so")
        self.dll(root, "horizon_client.dll", 45000)
        self.dll(root, "horizon_mod_wasm.dll", 12000)
        self.dll(root, "serde_derive.dll", 1, pe64=False)
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            prebuilt_game.exports(root)
        for name, count in (("horizon_client", 45000), ("horizon_mod_wasm", 12000), ("serde_derive", 1)):
            self.assertIn(f"{name}.dll: {count} exported names", output.getvalue())

    def test_exports_warns_about_any_dll_near_the_limit_without_failing(self):
        for large in ("horizon_client", "horizon_mod_wasm"):
            root = self.folder("deps/libhorizon_client.so")
            for name in ("horizon_client", "horizon_mod_wasm"):
                self.dll(root, f"{name}.dll", 49000 if name == large else 48999)
            output = io.StringIO()
            with self.subTest(large), contextlib.redirect_stdout(output):
                prebuilt_game.exports(root)
            warnings = [line for line in output.getvalue().splitlines() if line.startswith("::warning::")]
            self.assertEqual(len(warnings), 1)
            self.assertIn(f"{large}.dll exports 49,000 names", warnings[0])
            self.assertEqual(output.getvalue().count("exported names (Windows' limit"), 2)

    def test_exports_is_silent_without_dlls(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            prebuilt_game.exports(self.folder("deps/libhorizon_client.so", "deps/libhorizon_mod_wasm.so"))
        self.assertEqual(output.getvalue(), "")


if __name__ == "__main__":
    unittest.main()
