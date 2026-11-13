//! The API route (SDK design §1, PR 3b): an instance whose native mods are all API-only builds
//! them, from the prebuilt package's `api` files alone, into the `horizon_native_mods` library
//! that the package's prebuilt API executable loads in place of its stub. The executable is never
//! relinked and the `engine` archive is never read; the library, like any native build, is exact
//! to the game build.
use super::{api, dependencies, internals, prebuilt};
use crate::rust_toolchain::Toolchain;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The template the mods library is written from, in the `api` slice.
pub(super) const TEMPLATE: &str = "native_mod_entry.rs.template";
/// The folder of the `runtime` slice holding the API executable and its stub.
const BIN: &str = "bin";

/// Compiles the mods (`resolution`'s order) and the `horizon_native_mods` library against the
/// `api` slice alone, into `out`, and links the library against the `runtime` slice's game
/// libraries. Returns the `runtime` slice's API executable and the libraries to install beside
/// it: the game's, std, the toolchain's runtime on Windows, and the mods library.
pub(super) fn compile(
    rust: &Toolchain,
    runtime: &Path,
    api: &Path,
    out: &Path,
    projects: &[api::Project<'_>],
    declared: &[Vec<dependencies::Dependency>],
    resolution: &dependencies::Resolution,
) -> Result<(PathBuf, Vec<PathBuf>), String> {
    let sdk = prebuilt::identity(runtime)?;
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let deps = api.join("deps");
    let host = api.join("host");
    let runtime_deps = runtime.join("deps");
    let search = |edition: &str| prebuilt::rustc_with(rust, &[&deps, &host, &runtime_deps, out], edition);
    let mut libraries = HashMap::new();
    for (package, name) in api::PACKAGES {
        if package != api::INTERNALS_PACKAGE {
            libraries.insert(name, prebuilt::library(&deps, name)?);
        }
    }
    libraries.insert("mod_vocab", prebuilt::library(&deps, "mod_vocab")?);
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
            &prebuilt::library_entry(
                &api::template(&api.join(TEMPLATE))?,
                &[&project.bundle.manifest],
                &internals::Catalog::read(api)?,
                sdk.commit(),
                &project.bundle.manifest.id,
                &project.bundle.manifest.version.to_string(),
                &[],
            ),
            sdk.commit(),
        )?;
        rlibs.insert(project.bundle.manifest.id.as_str(), rlib);
    }
    let mut installed = prebuilt::runtime_libraries(rust, &runtime.join("deps"))?;
    for &index in &resolution.order {
        installed.push(rlibs[projects[index].bundle.manifest.id.as_str()].clone());
    }
    Ok((runtime.join(BIN).join(crate::backend::CLIENT), installed))
}
