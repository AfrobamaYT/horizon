import tempfile
import unittest
from pathlib import Path

from tools import prebuilt_game


class PrebuiltContentsTests(unittest.TestCase):
    def folder(self, *names: str) -> Path:
        root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        for name in names:
            (root / name).parent.mkdir(parents=True, exist_ok=True)
            (root / name).write_bytes(b"")
        return root

    def test_compiler_outputs_pass(self):
        prebuilt_game.check(self.folder(
            "horizon-prebuilt.json", "Cargo.lock", "native_main.rs.template", "deps/libhorizon_client.so", "deps/libmod_api-1.rmeta",
            "native/libwindows.0.52.0.a",
            "deps/libserde_derive-2.so", "deps/libhorizon_client.dll.a",
        ))

    def test_anything_else_is_refused(self):
        for stray in ("assets/minecraft/lang/en_us.json", "deps/Cargo.lock", "deps/client.jar", "deps/libmod_api-3.rlib", "native/libfoo.so", "notes.txt"):
            with self.subTest(stray), self.assertRaises(SystemExit):
                prebuilt_game.check(self.folder("deps/libhorizon_client.so", stray))

    def test_a_folder_without_the_game_is_refused(self):
        with self.assertRaises(SystemExit):
            prebuilt_game.check(self.folder("deps/libmod_api-3.rmeta"))


if __name__ == "__main__":
    unittest.main()
