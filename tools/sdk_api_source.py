#!/usr/bin/env python3
"""The SDK's API source: the mod API crates as a workspace of their own, with their generated code written out, so
a mod builds against them with no game source and no generator run.

    python3 tools/sdk_api_source.py build OUT    # write the API source for this checkout's commit into OUT
    python3 tools/sdk_api_source.py check OUT    # check the starter mod against OUT alone, host and wasm32

`build` takes the generated files from the in-tree build scripts' own output (`cargo check` for the host, for
wasm32 and with every feature the API source keeps) and refuses when they differ between those builds. Each crate
carries exactly the source files rustc read for its library (its dep-info), its manifest with workspace keys
resolved and no dev- or build-dependencies, and, for a crate whose build script runs the generator, a build script
that copies `generated/` into `OUT_DIR`. The generated files name source files by absolute path; the checkout's
root is written as ROOT_TOKEN and the build script puts the API source's own root in its place. `Cargo.lock` pins
the versions the game's lockfile has. Same commit, same bytes.
"""

import argparse
import gzip
import hashlib
import io
import json
import lzma
import os
import re
import shutil
import subprocess
import sys
import tarfile
import time
import tomllib
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
API = "horizon-mod-api"
WASM = "wasm32-unknown-unknown"
GENERATED = "generated"
ROOT_TOKEN = "{horizon-api-source-root}"
TEMPLATE = ROOT / "crates/cargo-horizon-mod/template/native"
# Where a mod finds the API source (design: the SDK fetched into the mod's `.horizon/sdk/`).
MOD_PATH = ".horizon/sdk/api"
# The build script of a crate whose generated files come with it. Paths become `/`-separated, valid in Rust
# string literals on every host.
BUILD_SCRIPT = f"""\
//! Writes the generated files that come with this crate into `OUT_DIR`, naming this workspace's root where
//! they name source files.
use std::{{env, fs, path::PathBuf}};

fn main() {{
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let root = manifest.join("../..");
    let root = root.to_str().expect("a UTF-8 path").replace('\\\\', "/");
    println!("cargo:rerun-if-changed={GENERATED}");
    for entry in fs::read_dir(manifest.join("{GENERATED}")).expect("reading {GENERATED}/") {{
        let path = entry.expect("reading {GENERATED}/").path();
        let code = fs::read_to_string(&path).expect("reading a generated file");
        let name = path.file_name().expect("a file name");
        fs::write(out.join(name), code.replace("{ROOT_TOKEN}", &root)).expect("writing OUT_DIR");
    }}
}}
"""


class Refusal(Exception):
    """The API source cannot be written as asked; `code` names the rule."""

    def __init__(self, code: str, detail: str):
        super().__init__(f"{code}: {detail}")
        self.code = code


@dataclass(frozen=True)
class Crate:
    name: str
    directory: str  # relative to the workspace root, `/`-separated: the same place in the API source
    generated: bool  # its build script runs the generator: the API source carries the output instead


def run(command: list[str], **kwargs) -> str:
    print("+", " ".join(command), flush=True)
    return subprocess.run(command, check=True, text=True, stdout=subprocess.PIPE, **kwargs).stdout


def closure(metadata: dict, root: str = API) -> list[Crate]:
    """The workspace crates the API's library needs: its normal dependencies, transitively, proc macros and
    theirs included. Build-dependencies are not followed: a crate's build script either runs the generator, whose
    output the API source carries, or is refused."""
    packages = {package["id"]: package for package in metadata["packages"]}
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    start = [package["id"] for package in metadata["packages"] if package["name"] == root and package["source"] is None]
    if len(start) != 1:
        raise Refusal("E_API_ROOT", f"expected one workspace package named {root}")
    workspace = Path(metadata["workspace_root"])
    pending, seen, crates = start, set(), []
    while pending:
        package_id = pending.pop()
        if package_id in seen:
            continue
        seen.add(package_id)
        package = packages[package_id]
        if package["source"] is not None:
            continue
        build = any("custom-build" in target["kind"] for target in package["targets"])
        directory = Path(package["manifest_path"]).parent.relative_to(workspace).as_posix()
        if build and not any(dependency["kind"] == "build" and dependency.get("path")
                             and dependency["name"] == "horizon-mod-api-gen" for dependency in package["dependencies"]):
            raise Refusal("E_BUILD_SCRIPT", f"{package['name']} has a build script that is not the generator")
        crates.append(Crate(package["name"], directory, build))
        pending.extend(dependency["pkg"] for dependency in nodes[package_id]["deps"]
                       if any(kind["kind"] is None for kind in dependency["dep_kinds"]))
    return sorted(crates, key=lambda crate: crate.name)


def kept_features(manifests: dict[str, dict]) -> dict[str, dict[str, list[str]]]:
    """Each crate's features that work without the dropped dependencies: a feature that names a dev- or
    build-dependency, or a feature dropped elsewhere, is dropped, until nothing changes."""
    kept = {name: dict(manifest.get("features", {})) for name, manifest in manifests.items()}
    dependencies = {name: set(normal_dependencies(manifest)) for name, manifest in manifests.items()}
    changed = True
    while changed:
        changed = False
        for name, features in kept.items():
            for feature, values in list(features.items()):
                if not all(feature_value_kept(value, name, dependencies, kept, manifests) for value in values):
                    del features[feature]
                    changed = True
    return kept


def feature_value_kept(value: str, crate: str, dependencies: dict, kept: dict, manifests: dict) -> bool:
    if value.startswith("dep:"):
        return value[4:] in dependencies[crate]
    if "/" not in value:
        return value in kept[crate]
    dependency, feature = value.split("/", 1)
    dependency = dependency.rstrip("?")
    if dependency not in dependencies[crate]:
        return False
    package = normal_dependencies(manifests[crate])[dependency].get("package", dependency)
    return package not in kept or feature in kept[package]


def normal_dependencies(manifest: dict) -> dict[str, dict]:
    """`[dependencies]` and every `[target.*.dependencies]`, by the name the crate uses."""
    tables = [manifest.get("dependencies", {})]
    tables += [target.get("dependencies", {}) for target in manifest.get("target", {}).values()]
    return {name: spec if isinstance(spec, dict) else {"version": spec}
            for table in tables for name, spec in table.items()}


def resolve_workspace(value, inherited: dict, what: str):
    if isinstance(value, dict) and value.get("workspace") is True:
        if what not in inherited:
            raise Refusal("E_WORKSPACE_KEY", f"{what} is inherited but the workspace does not set it")
        base = inherited[what]
        rest = {key: item for key, item in value.items() if key != "workspace"}
        if not rest:
            return base
        base = base if isinstance(base, dict) else {"version": base}
        return {**base, **rest, "features": sorted(set(base.get("features", [])) | set(rest.get("features", [])))}
    return value


def normalise_manifest(manifest: dict, workspace: dict, crate: Crate, crates: dict[str, Crate],
                       features: dict[str, list[str]]) -> str:
    """The crate's manifest as the API source carries it: workspace keys resolved, path dependencies pointing
    inside the API source, no dev- or build-dependencies, no targets but the library."""
    inherited_package = workspace.get("workspace", {}).get("package", {})
    inherited_dependencies = workspace.get("workspace", {}).get("dependencies", {})
    package = {key: resolve_workspace(manifest["package"][key], inherited_package, key)
               for key in ("name", "version", "edition", "rust-version", "license", "description", "publish")
               if key in manifest["package"]}
    package.update(autobins=False, autoexamples=False, autotests=False, autobenches=False,
                   build="build.rs" if crate.generated else False)
    out = [table("package", package)]
    if "lib" in manifest:
        out.append(table("lib", {key: manifest["lib"][key] for key in ("name", "path", "proc-macro")
                                 if key in manifest["lib"]}))

    def dependencies(entries: dict) -> dict:
        written = {}
        for name, spec in entries.items():
            spec = resolve_workspace(spec, inherited_dependencies, name)
            if not isinstance(spec, dict):
                written[name] = spec
                continue
            spec = dict(spec)
            if "path" in spec:
                target = crates.get(spec.get("package", name))
                if target is None:
                    raise Refusal("E_PATH_DEPENDENCY", f"{crate.name} depends on {name}, outside the API source")
                spec["path"] = os.path.relpath(target.directory, crate.directory).replace(os.sep, "/")
            written[name] = spec
        return written

    out.append(table("dependencies", dependencies(manifest.get("dependencies", {}))))
    for platform, entries in sorted(manifest.get("target", {}).items()):
        if entries.get("dependencies"):
            out.append(table(f"target.{json.dumps(platform)}.dependencies", dependencies(entries["dependencies"])))
    if features:
        out.append(table("features", features))
    lints = manifest.get("lints", {})
    if lints.get("workspace") is True:
        lints = workspace.get("workspace", {}).get("lints", {})
    for tool, levels in sorted(lints.items()):
        out.append(table(f"lints.{tool}", levels))
    return "\n".join(out)


def toml_value(value) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, str)):
        return json.dumps(value)
    if isinstance(value, list):
        return "[" + ", ".join(toml_value(item) for item in value) + "]"
    if isinstance(value, dict):
        return "{ " + ", ".join(f"{toml_key(key)} = {toml_value(item)}" for key, item in value.items()) + " }"
    raise Refusal("E_MANIFEST_VALUE", f"cannot write {value!r}")


def toml_key(key: str) -> str:
    return key if re.fullmatch(r"[A-Za-z0-9_-]+", key) else json.dumps(key)


def table(name: str, entries: dict) -> str:
    return f"[{name}]\n" + "".join(f"{toml_key(key)} = {toml_value(value)}\n" for key, value in entries.items())


def dep_info(path: Path) -> list[str]:
    """The files rustc read, from the first rule of a Makefile-style dep-info file (`\\ ` escapes a space)."""
    rule = path.read_text().split("\n", 1)[0]
    _, _, files = rule.partition(": ")
    return [file.replace("\x00", " ") for file in files.replace("\\ ", "\x00").split()]


def root_forms(roots: list[Path], crate: Crate) -> list[str]:
    """How the in-tree build script names the checkout: through the crate (`<root>/<crate>/../..`) or directly."""
    forms = {f"{root}/{crate.directory}/../.." for root in roots} | {str(root) for root in roots}
    return sorted(forms, key=len, reverse=True)


def relocate(code: str, forms: list[str]) -> str:
    for form in forms:
        code = code.replace(form + "/", ROOT_TOKEN + "/")
    return code


def check_files(files: dict[str, bytes], crates: list[Crate], tracked: set[str], forbidden: list[str]) -> list[str]:
    """The packing list: every path relative and in its place, every source file tracked by git, no file naming
    a local folder. Sorted, so the same files always give the same list."""
    directories = {crate.directory: crate for crate in crates}
    for path, data in files.items():
        if path.startswith("/") or "\\" in path or ".." in path.split("/"):
            raise Refusal("E_PATH", f"{path} is not a plain relative path")
        if path not in ("Cargo.toml", "Cargo.lock"):
            crate = next((crate for directory, crate in directories.items()
                          if path.startswith(directory + "/")), None)
            rest = path[len(crate.directory) + 1:] if crate else ""
            allowed = crate is not None and (
                rest == "Cargo.toml"
                or (crate.generated and (rest == "build.rs" or re.fullmatch(rf"{GENERATED}/[\w.-]+\.rs", rest)))
                or (not rest.startswith(GENERATED + "/") and path in tracked))
            if not allowed:
                raise Refusal("E_STRAY_FILE", f"{path} is no part of the API source")
        for folder in forbidden:
            if folder.encode() in data:
                raise Refusal("E_ABSOLUTE_PATH", f"{path} names {folder}")
    return sorted(files)


def packing_index(files: dict[str, bytes]) -> str:
    return "".join(f"{hashlib.sha256(files[path]).hexdigest()}  {path}\n" for path in sorted(files))


def lock_disagreements(slice_lock: dict, game_lock: dict) -> list[str]:
    """Packages the API source pins differently from the game's lockfile."""
    game = {(package["name"], package["version"], package.get("source"), package.get("checksum"))
            for package in game_lock["package"]}
    return sorted(f"{package['name']} {package['version']}" for package in slice_lock["package"]
                  if (package["name"], package["version"], package.get("source"), package.get("checksum")) not in game)


def build_messages(arguments: list[str]) -> list[dict]:
    output = run(["cargo", "check", "--locked", "-p", API, "--message-format", "json", *arguments], cwd=ROOT)
    return [json.loads(line) for line in output.splitlines() if line.startswith("{")]


def collect(crates: list[Crate], features: dict[str, dict], workspace_root: Path) -> dict[str, bytes]:
    """Every file rustc read for the crates' libraries, over the host, wasm32 and all kept features."""
    by_directory = {(workspace_root / crate.directory).resolve(): crate for crate in crates}
    every = [f"{crate.name}/{feature}" for crate in crates for feature in sorted(features[crate.name])]
    builds = {"host": [], WASM: ["--target", WASM], "host, all features": ["--features", ",".join(every)]}
    files: dict[str, bytes] = {}
    generated: dict[str, tuple[str, bytes]] = {}
    for label, arguments in builds.items():
        messages = build_messages(arguments)
        out_dirs = {}
        for message in messages:
            if message["reason"] == "build-script-executed":
                out_dirs[message["package_id"]] = Path(message["out_dir"])
                if message["package_id"].startswith("path+") and any(
                        message[key] for key in ("cfgs", "env", "linked_libs", "linked_paths")):
                    raise Refusal("E_BUILD_SCRIPT", f"{message['package_id']} sets more than generated files")
        for message in messages:
            if message["reason"] != "compiler-artifact" or "custom-build" in message["target"]["kind"]:
                continue
            crate = by_directory.get(Path(message["manifest_path"]).resolve().parent)
            if crate is None:
                continue
            artifact = Path(message["filenames"][0])
            digest = artifact.name.split(".")[0].rsplit("-", 1)[1]
            info = artifact.parent / f"{message['target']['name'].replace('-', '_')}-{digest}.d"
            out_dir = out_dirs.get(message["package_id"])
            for read in dep_info(info):
                path = Path(os.path.normpath(workspace_root / read))
                if out_dir is not None and path.parent == out_dir:
                    data = relocate(path.read_text(), root_forms([workspace_root, ROOT], crate)).encode()
                    name = f"{crate.directory}/{GENERATED}/{path.name}"
                    if name in generated and generated[name][1] != data:
                        raise Refusal("E_TARGET_DEPENDENT", f"{path.name} differs between {generated[name][0]} "
                                                            f"and {label}")
                    generated[name] = (label, data)
                    files[name] = data
                elif path.is_relative_to(workspace_root / crate.directory):
                    files[path.relative_to(workspace_root).as_posix()] = path.read_bytes()
                else:
                    raise Refusal("E_SOURCE_OUTSIDE_CRATE", f"{crate.name} reads {path}")
        print(f"{label}: {len(files)} files so far", flush=True)
    return files


def build(out: Path) -> dict[str, bytes]:
    if out.exists() and any(out.iterdir()):
        raise Refusal("E_OUT_NOT_EMPTY", f"{out} is not empty")
    metadata = json.loads(run(["cargo", "metadata", "--locked", "--format-version", "1"], cwd=ROOT))
    workspace_root = Path(metadata["workspace_root"])
    crates = closure(metadata)
    by_name = {crate.name: crate for crate in crates}
    workspace = tomllib.loads((workspace_root / "Cargo.toml").read_text())
    manifests = {crate.name: tomllib.loads((workspace_root / crate.directory / "Cargo.toml").read_text())
                 for crate in crates}
    features = kept_features(manifests)
    files = collect(crates, features, workspace_root)
    for crate in crates:
        files[f"{crate.directory}/Cargo.toml"] = normalise_manifest(
            manifests[crate.name], workspace, crate, by_name, features[crate.name]).encode()
        if crate.generated:
            files[f"{crate.directory}/build.rs"] = BUILD_SCRIPT.encode()
    resolver = workspace.get("workspace", {}).get("resolver", "2")
    files["Cargo.toml"] = ("# The mod API's source with its generated code, for one game build.\n"
                           + table("workspace", {"resolver": resolver,
                                                 "members": [crate.directory for crate in crates]})).encode()
    out.mkdir(parents=True, exist_ok=True)
    for path, data in files.items():
        (out / path).parent.mkdir(parents=True, exist_ok=True)
        (out / path).write_bytes(data)
    # Cargo keeps the game's pins and drops the packages the API does not use.
    shutil.copyfile(workspace_root / "Cargo.lock", out / "Cargo.lock")
    run(["cargo", "metadata", "--format-version", "1"], cwd=out, env=isolated())
    files["Cargo.lock"] = (out / "Cargo.lock").read_bytes()
    disagreeing = lock_disagreements(tomllib.loads(files["Cargo.lock"].decode()),
                                     tomllib.loads((workspace_root / "Cargo.lock").read_text()))
    if disagreeing:
        raise Refusal("E_LOCK", "pinned differently from the game: " + ", ".join(disagreeing))
    tracked = set(filter(None, run(["git", "ls-files", "-z"], cwd=workspace_root).split("\0")))
    check_files(files, crates, tracked, forbidden_folders(workspace_root))
    (out / "Cargo.lock").write_bytes(files["Cargo.lock"])
    print(f"{len(files)} files, {sum(map(len, files.values()))} bytes; crates: "
          + ", ".join(crate.name for crate in crates), flush=True)
    return files


def forbidden_folders(workspace_root: Path) -> list[str]:
    folders = {str(workspace_root), str(ROOT), str(Path.home())}
    for variable in ("CARGO_HOME", "CARGO_TARGET_DIR"):
        if os.environ.get(variable):
            folders.add(os.environ[variable])
    return sorted(folder for folder in folders if len(folder) > 1)


def isolated() -> dict[str, str]:
    """Cargo's environment for the API source: never the checkout's target folder."""
    return {key: value for key, value in os.environ.items() if key != "CARGO_TARGET_DIR"}


def archive_sizes(folder: Path) -> dict[str, int]:
    """Raw, gzip and xz sizes of a reproducible tar of the folder (sorted names, no owners, time zero)."""
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w", format=tarfile.PAX_FORMAT) as archive:
        for path in sorted(p for p in folder.rglob("*") if p.is_file()):
            info = tarfile.TarInfo(path.relative_to(folder).as_posix())
            info.size, info.mode, info.mtime = path.stat().st_size, 0o644, 0
            archive.addfile(info, io.BytesIO(path.read_bytes()))
    raw = buffer.getvalue()
    return {"files": sum(p.stat().st_size for p in folder.rglob("*") if p.is_file()), "tar": len(raw),
            "gz": len(gzip.compress(raw, 9, mtime=0)), "xz": len(lzma.compress(raw, preset=9 | lzma.PRESET_EXTREME))}


def starter_mod(folder: Path) -> None:
    """The native template as `cargo horizon-mod new` writes it, with the API dependency in the path form."""
    def fill(text: str) -> str:
        return text.replace("{id}", "starter").replace("{name}", "Starter")

    manifest, count = re.subn(r"^horizon-mod-api = .*$", f'horizon-mod-api = {{ path = "{MOD_PATH}/crates/{API}" }}',
                              fill((TEMPLATE / "api.toml").read_text()), flags=re.MULTILINE)
    if count != 1:
        raise Refusal("E_TEMPLATE", "the template names horizon-mod-api once")
    # Its own workspace: cargo must not climb to a folder above it.
    (folder / "src").mkdir(parents=True)
    (folder / "Cargo.toml").write_text(manifest + "\n[workspace]\n")
    (folder / "src/lib.rs").write_text(fill((TEMPLATE / "lib.rs").read_text()))


def check(api: Path, scratch: Path) -> None:
    """The starter mod checks against the API source alone, for the host and wasm32, offline after one fetch; the
    API source's crates are the prebuilt game's API closure at the game's versions."""
    sys.path.insert(0, str(ROOT / "tools"))
    import prebuilt_game

    if scratch.exists():
        shutil.rmtree(scratch)
    mod = scratch / "starter"
    starter_mod(mod)
    shutil.copytree(api, mod / MOD_PATH)
    shutil.copyfile(api / "Cargo.lock", mod / "Cargo.lock")
    environment = {**isolated(), "CARGO_TARGET_DIR": str(scratch / "target"), "RUSTFLAGS": "-D warnings"}
    # Each is the root of its own workspace: cargo climbs to no folder above it.
    for folder, expected in ((mod, mod / "Cargo.toml"),
                             (mod / MOD_PATH / "crates" / API, mod / MOD_PATH / "Cargo.toml")):
        found = json.loads(run(["cargo", "locate-project", "--workspace"], cwd=folder, env=environment))["root"]
        if Path(found) != expected:
            raise Refusal("E_WORKSPACE_ESCAPE", f"{folder} belongs to {found}, not {expected}")
    run(["cargo", "fetch"], cwd=mod, env=environment)
    timings = {}
    for label, command, folder in (
            ("starter, host", ["cargo", "check", "--offline"], mod),
            ("starter, wasm32", ["cargo", "check", "--offline", "--target", WASM], mod),
            ("API source, host, all features", ["cargo", "check", "--offline", "--locked", "--workspace",
                                                "--all-features"], mod / MOD_PATH)):
        started = time.monotonic()
        run(command, cwd=folder, env=environment)
        timings[label] = time.monotonic() - started
    game = json.loads(run(["cargo", "metadata", "--locked", "--format-version", "1"], cwd=ROOT))
    ours = json.loads(run(["cargo", "metadata", "--locked", "--offline", "--format-version", "1"],
                          cwd=mod / MOD_PATH, env=environment))

    def api_closure(metadata: dict) -> set[tuple[str, str]]:
        packages = {package["id"]: package for package in metadata["packages"]}
        return {(packages[package]["name"], packages[package]["version"])
                for package in prebuilt_game.runtime_packages(metadata, API)}

    if api_closure(game) != api_closure(ours):
        raise Refusal("E_CLOSURE", f"the prebuilt game's API closure {sorted(api_closure(game) - api_closure(ours))} "
                                   f"against the API source's {sorted(api_closure(ours) - api_closure(game))}")
    disagreeing = lock_disagreements(tomllib.loads((mod / "Cargo.lock").read_text()),
                                     tomllib.loads((ROOT / "Cargo.lock").read_text()))
    if disagreeing != ["mod-starter 0.1.0"]:
        raise Refusal("E_LOCK", "the starter resolves differently from the game: " + ", ".join(disagreeing))
    print(f"API closure: {len(api_closure(ours))} crates, as the prebuilt game's")
    for label, seconds in timings.items():
        print(f"cargo check, {label}: {seconds:.1f}s")
    print("size: " + ", ".join(f"{kind} {size}" for kind, size in archive_sizes(api).items()))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("build").add_argument("out", type=Path)
    check_parser = commands.add_parser("check")
    check_parser.add_argument("out", type=Path)
    check_parser.add_argument("--scratch", type=Path, help="default: the target folder's sdk-api-source-check")
    args = parser.parse_args()
    try:
        if args.command == "build":
            build(args.out.resolve())
        else:
            target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
            check(args.out.resolve(), (args.scratch or target / "sdk-api-source-check").resolve())
    except Refusal as refusal:
        sys.exit(f"error: {refusal}")


if __name__ == "__main__":
    main()
