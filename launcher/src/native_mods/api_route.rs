//! The API route (SDK design §1, PR 3b): an instance whose native mods are all API-only builds
//! them, from the prebuilt package's `api` files alone, into the `horizon_native_mods` library
//! that the package's prebuilt API executable loads in place of its stub. The executable is never
//! relinked and the `engine` archive is never read; the library, like any native build, is exact
//! to the game build.
use super::{api, dependencies, internals, prebuilt};
use crate::i18n::trf;
use crate::rust_toolchain::Toolchain;
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

/// The template the mods library is written from, carried in the package's `api` files.
pub(super) const TEMPLATE: &str = "native_mods.rs.template";
/// The package folder holding the API executable and its stub.
const RUNTIME: &str = "runtime";

/// Compiles the mods (`resolution`'s order) and the `horizon_native_mods` library against the
/// package in `game`, into `out`. Returns the package's API executable and the libraries to
/// install beside it: the game's, std, the toolchain's runtime on Windows, and the mods library.
pub(super) fn compile(
    rust: &Toolchain,
    game: &Path,
    out: &Path,
    projects: &[api::Project<'_>],
    declared: &[Vec<dependencies::Dependency>],
    resolution: &dependencies::Resolution,
) -> Result<(PathBuf, Vec<PathBuf>), String> {
    let sdk = prebuilt::identity(game)?;
    let (Some(native_mods), Some(slices)) = (&sdk.native_mods, &sdk.slices) else {
        return Err(crate::i18n::tr(
            "This prebuilt game has no API route. Update Horizon and rebuild the mods against this game.",
        )
        .to_string());
    };
    let _ = fs::remove_dir_all(out);
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    // The `api` files alone, as their archive unpacks: nothing else is on the search path.
    let slice = out.join("api");
    stage(game, &slices.api, &slice)?;
    let deps = slice.join("deps");
    let search =
        |edition: &str| prebuilt::rustc_with(rust, &[&deps, &slice.join("host"), out], edition);
    let mut libraries = HashMap::new();
    for (package, name) in api::PACKAGES {
        if package != api::INTERNALS_PACKAGE {
            libraries.insert(name, prebuilt::library(&deps, name)?);
        }
    }
    let mut rlibs: HashMap<&str, PathBuf> = HashMap::new();
    for &index in &resolution.order {
        let project = &projects[index];
        let (rlib, _) = prebuilt::compile_mod(
            project,
            &declared[index],
            out,
            search,
            |package| &libraries[package],
            &rlibs,
        )?;
        rlibs.insert(project.bundle.manifest.id.as_str(), rlib);
    }
    let catalog = internals::Catalog::read(&slice)?;
    let manifests: Vec<_> = resolution
        .order
        .iter()
        .map(|&i| &projects[i].bundle.manifest)
        .collect();
    let template = api::template(&slice.join(TEMPLATE))?;
    let source = out.join(format!("{}.rs", native_mods.krate));
    fs::write(&source, api::main_rs(&template, &manifests, &catalog)).map_err(|e| e.to_string())?;
    let rlib = out.join(format!("lib{}.rlib", native_mods.krate));
    let mut cmd = search("2024");
    cmd.args(["--crate-type", "rlib", "--crate-name", &native_mods.krate])
        .arg("-C")
        .arg(format!("metadata={}", native_mods.metadata));
    for name in ["mod_api", "mod_points"] {
        cmd.arg("--extern")
            .arg(format!("{name}={}", libraries[name].display()));
    }
    for (position, &index) in resolution.order.iter().enumerate() {
        let id = projects[index].bundle.manifest.id.as_str();
        cmd.arg("--extern")
            .arg(format!("mod_{position}={}", rlibs[id].display()));
    }
    cmd.arg("-o").arg(&rlib).arg(&source);
    super::run(cmd, "Compiling the mods library", |_| {})?;

    let objects = out.join("objects");
    let mut members = Vec::new();
    for (n, archive) in std::iter::once(&rlib)
        .chain(
            resolution
                .order
                .iter()
                .map(|&i| &rlibs[projects[i].bundle.manifest.id.as_str()]),
        )
        .enumerate()
    {
        members.extend(extract_objects(archive, &objects.join(n.to_string()))?);
    }
    let runtime_deps = game.join("deps");
    let mut installed = prebuilt::runtime_libraries(rust, &runtime_deps)?;
    let library = out.join(library_name(&native_mods.krate));
    let std = installed
        .iter()
        .find(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            name.starts_with("libstd-") || name.starts_with("std-")
        })
        .cloned()
        .ok_or("no dynamic standard library")?;
    super::run(
        link(rust, &library, &members, &runtime_deps, &std)?,
        "Linking the mods library",
        |_| {},
    )?;
    installed.push(library);
    Ok((game.join(RUNTIME).join(crate::backend::CLIENT), installed))
}

/// The mods library's file name on this OS.
fn library_name(krate: &str) -> String {
    match std::env::consts::OS {
        "windows" => format!("{krate}.dll"),
        "macos" => format!("lib{krate}.dylib"),
        _ => format!("lib{krate}.so"),
    }
}

/// Link the object files `members` into the shared library `library`, against the game's two
/// libraries in `deps` and `std`, with this OS's C toolchain: the rlibs' metadata members are left
/// out, which rustc's own dylib link cannot do without the game's crates as rlibs.
fn link(
    rust: &Toolchain,
    library: &Path,
    members: &[PathBuf],
    deps: &Path,
    std: &Path,
) -> Result<std::process::Command, String> {
    let game = |krate: &str| prebuilt::library(deps, krate);
    let name = library
        .file_name()
        .ok_or("invalid library path")?
        .to_string_lossy()
        .into_owned();
    let mut cmd = match std::env::consts::OS {
        "windows" => {
            // MinGW's linker takes the DLLs themselves; no import libraries are needed.
            let mut cmd = rust.command(&rust.mingw_bin().join("gcc.exe"));
            cmd.arg("-shared").arg("-o").arg(library).args(members);
            cmd.arg(game("horizon_mod_host")?)
                .arg(game("horizon_mod_wasm")?)
                .arg(std)
                .arg("-Wl,--no-undefined");
            cmd
        }
        "macos" => {
            let mut cmd = rust.command(Path::new("cc"));
            cmd.arg("-dynamiclib").arg("-o").arg(library).args(members);
            // Found through the executable's rpath, where the stub sits.
            cmd.arg("-install_name")
                .arg(format!("@rpath/{name}"))
                .arg(game("horizon_mod_host")?)
                .arg(game("horizon_mod_wasm")?)
                .arg(std);
            cmd
        }
        _ => {
            let mut cmd = rust.command(Path::new("cc"));
            cmd.arg("-shared").arg("-o").arg(library).args(members);
            // By file name, so each needed entry names a library beside the executable.
            let file = |path: &Path| {
                format!(
                    "-l:{}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                )
            };
            cmd.arg("-Wl,--no-undefined")
                .arg("-L")
                .arg(deps)
                .arg(file(&game("horizon_mod_host")?))
                .arg(file(&game("horizon_mod_wasm")?))
                .arg("-L")
                .arg(std.parent().ok_or("invalid std path")?)
                .arg(file(std))
                .args(["-lgcc_s", "-lc"]);
            cmd
        }
    };
    cmd.current_dir(library.parent().ok_or("invalid library path")?);
    Ok(cmd)
}

/// Write the object files of the archive `rlib` into `into`, returning their paths: every member
/// but the crate metadata (`lib.rmeta`) and the archive's symbol tables.
fn extract_objects(rlib: &Path, into: &Path) -> Result<Vec<PathBuf>, String> {
    let shown = |e: &dyn std::fmt::Display| format!("{}: {e}", rlib.display());
    let data = fs::read(rlib).map_err(|e| shown(&e))?;
    let archive = object::read::archive::ArchiveFile::parse(&*data).map_err(|e| shown(&e))?;
    fs::create_dir_all(into).map_err(|e| shown(&e))?;
    let mut objects = Vec::new();
    for member in archive.members() {
        let member = member.map_err(|e| shown(&e))?;
        let name = String::from_utf8_lossy(member.name()).into_owned();
        if !name.ends_with(".o") {
            continue;
        }
        if !plain(&name) {
            return Err(shown(&format!("an archive member named {name}")));
        }
        let path = into.join(&name);
        fs::write(&path, member.data(&*data).map_err(|e| shown(&e))?).map_err(|e| shown(&e))?;
        objects.push(path);
    }
    if objects.is_empty() {
        return Err(shown(&"no object files"));
    }
    Ok(objects)
}

/// Copy (hard-link where it can) the package files `names` from `game` into `into`, refusing a
/// name that is not a plain relative path and naming every missing file.
fn stage(game: &Path, names: &[String], into: &Path) -> Result<(), String> {
    let mut missing = Vec::new();
    for name in names {
        if !plain(name) {
            return Err(trf(
                "The prebuilt game's API file list names {file}, which is not a plain path.",
                &[("file", name)],
            ));
        }
        let from = game.join(name);
        if !from.is_file() {
            missing.push(name.as_str());
            continue;
        }
        let to = into.join(name);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        if fs::hard_link(&from, &to).is_err() {
            fs::copy(&from, &to).map_err(|e| format!("{}: {e}", from.display()))?;
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(trf(
            "The prebuilt game is missing its API files: {files}.",
            &[("files", &missing.join(", "))],
        ))
    }
}

/// Whether `name` is a relative path of plain components only.
fn plain(name: &str) -> bool {
    !name.is_empty()
        && Path::new(name)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staging_takes_only_the_listed_files_and_names_every_missing_one() {
        let game = super::super::tests::dir("api-route-stage-game");
        let into = super::super::tests::dir("api-route-stage-into");
        for name in [
            "deps/libmod_api-1.rmeta",
            "host/libmacros-1.so",
            "deps/libhorizon_client-1.rmeta",
        ] {
            fs::create_dir_all(game.join(name).parent().unwrap()).unwrap();
            fs::write(game.join(name), name).unwrap();
        }
        let listed = [
            "deps/libmod_api-1.rmeta".to_owned(),
            "host/libmacros-1.so".to_owned(),
        ];
        stage(&game, &listed, &into).unwrap();
        assert!(into.join("deps/libmod_api-1.rmeta").is_file());
        assert!(into.join("host/libmacros-1.so").is_file());
        assert!(!into.join("deps/libhorizon_client-1.rmeta").exists());
        let gone = ["deps/a".to_owned(), "deps/b".to_owned()];
        let error = stage(
            &game,
            &gone,
            &super::super::tests::dir("api-route-stage-gone"),
        )
        .unwrap_err();
        assert!(error.contains("deps/a, deps/b"), "{error}");
        for bad in ["../x", "/etc/x", ""] {
            let error = stage(&game, &[bad.to_owned()], &into).unwrap_err();
            assert!(error.contains("not a plain path"), "{bad}: {error}");
        }
    }

    #[test]
    fn only_object_members_come_out_of_an_rlib() {
        let folder = super::super::tests::dir("api-route-objects");
        let mut archive = b"!<arch>\n".to_vec();
        for (name, data) in [
            ("lib.rmeta", &b"meta"[..]),
            ("a.o", b"obj1"),
            ("b.o", b"obj2"),
        ] {
            archive.extend(
                format!(
                    "{:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n",
                    format!("{name}/"),
                    0,
                    0,
                    0,
                    644,
                    data.len()
                )
                .as_bytes(),
            );
            archive.extend(data);
        }
        let rlib = folder.join("libx.rlib");
        fs::write(&rlib, &archive).unwrap();
        let objects = extract_objects(&rlib, &folder.join("out")).unwrap();
        let names: Vec<_> = objects
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["a.o", "b.o"]);
        assert_eq!(fs::read(&objects[1]).unwrap(), b"obj2");
        fs::write(&rlib, b"!<arch>\n").unwrap();
        assert!(extract_objects(&rlib, &folder.join("none"))
            .unwrap_err()
            .contains("no object files"));
    }
}
