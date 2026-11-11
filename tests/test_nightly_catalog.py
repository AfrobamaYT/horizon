"""Exercise the catalog export boundary and actual Nightly signing/publishing shell."""
import base64
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("nightly_catalog", ROOT / "tools/ci/nightly_catalog.py")
CATALOG = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CATALOG)


def workflow_run(name):
    workflow = (ROOT / ".github/workflows/nightly-build.yml").read_text()
    section = workflow.split("      - name: " + name + "\n", 1)[1]
    lines = section.split("        run: |\n", 1)[1].splitlines()
    result = []
    for line in lines:
        if line and not line.startswith("          "):
            break
        result.append(line[10:])
    return "\n".join(result)


def distribution(root):
    (root / "dist").mkdir()
    for name in ("horizon-linux-x86_64.tar.gz", "horizon-windows-x64.zip", "horizon-macos-arm64.zip",
                 "horizon-macos-x64.zip", "horizon-server-linux-x86_64.tar.gz", "horizon-source.tar.gz"):
        (root / "dist" / name).write_bytes(b"test archive\n")
    (root / "dist/mod-catalog.json").write_text('{"schema_version":1,"hooks":[],"points":[]}\n')
    (root / "tools/ci").mkdir(parents=True)
    shutil.copy2(ROOT / "tools/ci/sign_release.sh", root / "tools/ci/sign_release.sh")


class NightlyCatalog(unittest.TestCase):
    def test_wrong_commit_and_dirty_source_refuse_before_generation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            (root / "source").write_text("committed\n")
            subprocess.run(["git", "add", "source"], cwd=root, check=True)
            subprocess.run(["git", "-c", "user.name=Test", "-c", "user.email=test@example.test",
                            "commit", "-qm", "fixture"], cwd=root, check=True)
            commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
            output = root / "mod-catalog.json"
            with self.assertRaisesRegex(ValueError, "not selected Nightly commit"):
                CATALOG.export(root, "0" * 40, output)
            (root / "source").write_text("changed\n")
            with self.assertRaises(subprocess.CalledProcessError):
                CATALOG.export(root, commit, output)
            self.assertFalse(output.exists())

    def test_workflow_exports_selected_commit_and_collects_the_asset(self):
        build = (ROOT / ".github/workflows/game-build.yml").read_text()
        export = build.split("      - name: Export the pinned Nightly SDK catalog\n", 1)[1].split("      - name: Build the official game", 1)[0]
        self.assertIn("CATALOG_COMMIT: ${{ inputs.ref }}", export)
        self.assertIn('--commit "$CATALOG_COMMIT"', export)
        self.assertIn("matrix.name == 'linux'", export)
        self.assertIn("inputs.nightly != ''", export)
        self.assertIn("name: horizon-mod-catalog", export)
        self.assertIn("path: ${{ runner.temp }}/mod-catalog.json", export)
        nightly = (ROOT / ".github/workflows/nightly-build.yml").read_text()
        self.assertIn("pattern: horizon-*", nightly)

    @unittest.skipUnless(shutil.which("cargo") and shutil.which("rustc"), "requires Rust for actual pinned catalog export")
    def test_actual_pinned_export_from_clean_checkout(self):
        if subprocess.run(["git", "diff", "--quiet", "HEAD", "--"], cwd=ROOT).returncode:
            self.skipTest("requires committed source for the selected-commit provenance check")
        commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "mod-catalog.json"
            CATALOG.export(ROOT, commit, output)
            document = json.loads(output.read_text())
            self.assertEqual(document["schema_version"], 1)
            hook = next(hook for hook in document["hooks"] if hook["id"] == "use_item")
            self.assertIn("hand", [argument["name"] for argument in hook["arguments"]])
            self.assertTrue(document["points"])
            self.assertFalse((ROOT / ".horizon-source-staged").exists())

    @unittest.skipUnless(os.name == "posix" and shutil.which("bash") and shutil.which("openssl"),
                         "requires Bash and OpenSSL for real Nightly signing")
    def test_catalog_is_signed_only_for_the_fork_and_payload_tampering_fails(self):
        shell = workflow_run("Sign the checksums for the nightly channel (SHA256SUMS.sig) and for this build (SHA256SUMS.v2.sig)")
        for repository in ("arismoko/horizon", "horizon-rust/horizon"):
            with self.subTest(repository=repository), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                distribution(root)
                subprocess.run(["bash", "-e", "-c", shell], cwd=root, check=True, capture_output=True,
                               env={**os.environ, "GITHUB_REPOSITORY": repository, "EPHEMERAL": "yes",
                                    "ALSO_FOR": "nightly-2026-11-10-1234567", "RELEASE_SIGNING_KEY": "",
                                    "RELEASE_SIGNING_KEY_NEXT": ""})
                manifest = (root / "dist/SHA256SUMS").read_bytes()
                payload = root / "dist/mod-catalog.json"
                entry = hashlib.sha256(payload.read_bytes()).hexdigest() + "  mod-catalog.json"
                self.assertEqual(entry in manifest.decode(), repository == "arismoko/horizon")
                key = base64.b64decode((root / "dist/ephemeral.pub").read_bytes())
                public = root / "public.der"
                public.write_bytes(bytes.fromhex("302a300506032b6570032100") + key)
                for tag, signature in (("nightly", "SHA256SUMS.sig"),
                                       ("nightly-2026-11-10-1234567", "SHA256SUMS.v2.sig")):
                    message = root / "message"
                    message.write_bytes(f"horizon release {tag}\n".encode() + manifest)
                    raw = root / "signature"
                    raw.write_bytes(base64.b64decode((root / "dist" / signature).read_bytes()))
                    subprocess.run(["openssl", "pkeyutl", "-verify", "-pubin", "-keyform", "DER", "-inkey",
                                    str(public), "-rawin", "-in", str(message), "-sigfile", str(raw)],
                                   check=True, capture_output=True)
                if repository == "arismoko/horizon":
                    payload.write_text("tampered\n")
                    result = subprocess.run(["sha256sum", "-c", "SHA256SUMS"], cwd=root / "dist", capture_output=True)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn(b"mod-catalog.json: FAILED", result.stdout)

    @unittest.skipUnless(os.name == "posix" and shutil.which("bash"), "requires Bash signing step")
    def test_missing_fork_catalog_refuses_signing(self):
        shell = workflow_run("Sign the checksums for the nightly channel (SHA256SUMS.sig) and for this build (SHA256SUMS.v2.sig)")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            distribution(root)
            (root / "dist/mod-catalog.json").unlink()
            result = subprocess.run(["bash", "-e", "-c", shell], cwd=root, capture_output=True,
                                    env={**os.environ, "GITHUB_REPOSITORY": "arismoko/horizon", "EPHEMERAL": "yes",
                                         "ALSO_FOR": "nightly-2026-11-10-1234567"})
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((root / "dist/SHA256SUMS.sig").exists())

    @unittest.skipUnless(os.name == "posix" and shutil.which("bash"), "requires Bash publishing step")
    def test_release_invocation_attaches_catalog_only_for_fork(self):
        workflow = (ROOT / ".github/workflows/nightly-build.yml").read_text()
        # This block's name is deliberately located from the actual release invocation.
        section = workflow[:workflow.index("          gh release create nightly")]
        name = section.rsplit("      - name: ", 1)[1].splitlines()[0]
        shell = workflow_run(name)
        for repository in ("arismoko/horizon", "horizon-rust/horizon"):
            with self.subTest(repository=repository), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                distribution(root)
                tools = root / "bin"
                tools.mkdir()
                (tools / "python3").write_text("#!/bin/sh\nexit 0\n")
                (tools / "gh").write_text(f"#!{sys.executable}\nimport json, os, sys\nfrom pathlib import Path\nPath(os.environ['CAPTURE']).write_text(json.dumps(sys.argv[1:]))\n")
                for executable in tools.iterdir():
                    executable.chmod(0o755)
                capture = root / "arguments.json"
                subprocess.run(["bash", "-e", "-c", shell], cwd=root, check=True, capture_output=True,
                               env={**os.environ, "PATH": str(tools) + os.pathsep + os.environ["PATH"],
                                    "GITHUB_REPOSITORY": repository, "SHA": "1" * 40, "SHORT": "1" * 7,
                                    "DATE": "2026-11-10", "SOURCE_TAG": "nightly-source/test", "CAPTURE": str(capture)})
                arguments = json.loads(capture.read_text())
                attached = [argument.split("#", 1)[0] for argument in arguments]
                self.assertEqual("dist/mod-catalog.json" in attached, repository == "arismoko/horizon")
                self.assertIn("dist/SHA256SUMS", attached)


if __name__ == "__main__":
    unittest.main()
