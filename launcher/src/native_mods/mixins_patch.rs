//! The horizon-mixins tool for `[[native.mixin]]` builds (`horizon_mod_package::mixins_tool`):
//! once per pin and patch the launcher downloads the prebuilt tool from its release, and where
//! that release has none for this computer it downloads the tool's source at the pin, applies the
//! carried patch only to the exact text it was written against, and builds it.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use horizon_mod_package::index::native::PREBUILT_REPO;
use horizon_mod_package::mixins_tool::{self, PATCH, PATCH_NAME};
use sha2::{Digest, Sha256};

use super::{root, run, Step, MIXINS_REV};
use crate::i18n::trf;
use crate::rust_toolchain::{self, Toolchain};

const SUMS: &str = "SHA256SUMS";
/// The archive's largest plausible size, packed or unpacked: a corrupt or hostile one stops there.
const LIMIT: u64 = 512 << 20;
/// Set by `native-build --mixins-from-source`: the prebuilt tool is not looked for.
pub(super) static FROM_SOURCE: AtomicBool = AtomicBool::new(false);

/// One file of the tool's release; `None` when the release has no such file.
type Get<'a> = &'a dyn Fn(&str) -> Result<Option<Box<dyn Read>>, String>;

/// The patched tool at [`MIXINS_REV`], fetched once per pin and patch.
pub(super) fn tool(rust: &Toolchain, progress: &impl Fn(Step)) -> Result<PathBuf, String> {
    // Inside the pin's own tool folder, so storage cleanup counts it with the pin.
    let dir = root()
        .join("tools")
        .join(&MIXINS_REV[..12])
        .join(format!("patched-{}", &mixins_tool::patch_hash()[..12]));
    let bin = dir
        .join("bin")
        .join(format!("horizon-mixins{}", std::env::consts::EXE_SUFFIX));
    if bin.is_file() {
        return Ok(bin);
    }
    progress(Step::Tool);
    let started = std::time::Instant::now();
    let seconds = || format!("{:.1}", started.elapsed().as_secs_f64());
    let target = rust.host.target;
    if !FROM_SOURCE.load(Ordering::Relaxed) {
        let release = format!(
            "https://github.com/{PREBUILT_REPO}/releases/download/{}",
            mixins_tool::release_tag()
        );
        let found = prebuilt(&dir, target, &|name| {
            Ok(
                crate::backend::get_unless_missing(&format!("{release}/{name}"))?
                    .map(|response| Box::new(response.into_reader()) as Box<dyn Read>),
            )
        })?;
        if found && bin.is_file() {
            super::append_log(&trf(
                "horizon-mixins tool: downloaded in {seconds} s (once per pin)",
                &[("seconds", &seconds())],
            ))?;
            return Ok(bin);
        }
        super::append_log(&trf(
            "horizon-mixins tool: no prebuilt for {target}; building from source (once per pin)",
            &[("target", target)],
        ))?;
    }
    let partial = dir.join(".source");
    let top = super::unpack_archive(
        &format!("{}/{MIXINS_REV}", horizon_addresses::HORIZON_MIXINS_SOURCE),
        &partial,
        256 << 20,
    )?;
    mixins_tool::apply(&top, PATCH).map_err(|why| {
        format!("E_MIXINS_PATCH: {PATCH_NAME} does not apply to horizon-mixins {MIXINS_REV}: {why}")
    })?;
    let mut cmd = rust.command(&rust.cargo());
    rust_toolchain::low_priority(&mut cmd);
    rust.configure_cargo(&mut cmd);
    cmd.args(["install", "--locked", "--path"])
        .arg(top.join("crates/horizon-mixins-cli"))
        .arg("--root")
        .arg(&dir);
    let built = run(cmd, "Building horizon-mixins", |_| {});
    let _ = fs::remove_dir_all(&partial);
    built?;
    super::append_log(&trf(
        "horizon-mixins tool: fetched and built in {seconds} s (once per pin)",
        &[("seconds", &seconds())],
    ))?;
    Ok(bin)
}

/// Puts the release's prebuilt tool for `target` in `dir/bin`. `Ok(false)` when the release, or
/// its archive for this target, does not exist: the caller builds from source. A download that
/// fails, an archive that does not match the release's `SHA256SUMS`, and one built from another
/// source, patch or target are errors, never a source build.
fn prebuilt(dir: &Path, target: &str, get: Get) -> Result<bool, String> {
    let name = mixins_tool::archive(target);
    let download = |why: String| {
        trf(
            "The horizon-mixins tool couldn't be downloaded ({why}). Try again.",
            &[("why", &why)],
        )
    };
    let Some(mut sums) = get(SUMS).map_err(download)? else {
        return Ok(false);
    };
    let mut listing = String::new();
    sums.read_to_string(&mut listing)
        .map_err(|e| download(e.to_string()))?;
    let Some(expected) = super::prebuilt::expected_sum(&listing, &name) else {
        return Ok(false);
    };
    let Some(body) = get(&name).map_err(download)? else {
        return Ok(false);
    };
    let mut bytes = Vec::new();
    body.take(LIMIT)
        .read_to_end(&mut bytes)
        .map_err(|e| download(e.to_string()))?;
    if format!("{:x}", Sha256::digest(&bytes)) != expected {
        return Err(trf(
            "The horizon-mixins tool {name} doesn't match its release's checksum. Try again.",
            &[("name", &name)],
        ));
    }
    let partial = dir.join(".prebuilt");
    let _ = fs::remove_dir_all(&partial);
    // `unpack_in` writes inside a folder that exists.
    fs::create_dir_all(&partial).map_err(|e| e.to_string())?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(&bytes[..]).take(LIMIT));
    for entry in tar.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.header().entry_type();
        if kind.is_file() || kind.is_dir() {
            entry
                .unpack_in(&partial)
                .map_err(|e| format!("Could not unpack the horizon-mixins tool: {e}"))?;
        }
    }
    let identity = fs::read_to_string(partial.join(mixins_tool::IDENTITY)).unwrap_or_default();
    if !identity.starts_with(&mixins_tool::identity(target)) {
        let _ = fs::remove_dir_all(&partial);
        return Err(trf(
            "The horizon-mixins tool {name} was not built from the source, patch and target this launcher pins.",
            &[("name", &name)],
        ));
    }
    let bin = dir.join("bin");
    let _ = fs::remove_dir_all(&bin);
    fs::rename(partial.join("bin"), &bin).map_err(|e| e.to_string())?;
    let _ = fs::remove_dir_all(&partial);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TARGET: &str = "x86_64-unknown-linux-gnu";

    /// A release archive holding the tool and `identity`.
    fn archive(identity: &str) -> Vec<u8> {
        let mut tar = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        ));
        for (path, text) in [
            ("bin/horizon-mixins", "the tool"),
            (mixins_tool::IDENTITY, identity),
        ] {
            let mut header = tar::Header::new_gnu();
            header.set_size(text.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            tar.append_data(&mut header, path, text.as_bytes()).unwrap();
        }
        tar.into_inner().unwrap().finish().unwrap()
    }

    fn sums(archive: &[u8]) -> String {
        format!(
            "{:x}  {}\n",
            Sha256::digest(archive),
            mixins_tool::archive(TARGET)
        )
    }

    /// A release with these two files; `None` is a 404.
    fn release(
        sums: Option<String>,
        archive: Option<Vec<u8>>,
    ) -> impl Fn(&str) -> Result<Option<Box<dyn Read>>, String> {
        move |name| {
            let file = if name == SUMS {
                sums.clone().map(String::into_bytes)
            } else {
                assert_eq!(name, mixins_tool::archive(TARGET));
                archive.clone()
            };
            Ok(file.map(|bytes| Box::new(std::io::Cursor::new(bytes)) as Box<dyn Read>))
        }
    }

    #[test]
    fn the_prebuilt_tool_is_checked_then_put_in_place() {
        let dir = tempfile::tempdir().unwrap();
        // The compiler's line after the checked ones is information only.
        let packed = archive(&format!("{}rustc = any\n", mixins_tool::identity(TARGET)));
        let get = release(Some(sums(&packed)), Some(packed));
        assert_eq!(prebuilt(dir.path(), TARGET, &get), Ok(true));
        assert_eq!(
            fs::read_to_string(dir.path().join("bin/horizon-mixins")).unwrap(),
            "the tool"
        );
        assert!(!dir.path().join(".prebuilt").exists());
    }

    #[test]
    fn no_release_or_no_archive_for_this_computer_means_a_source_build() {
        let dir = tempfile::tempdir().unwrap();
        let packed = archive(&mixins_tool::identity(TARGET));
        let other = sums(&packed).replace(TARGET, "aarch64-apple-darwin");
        for get in [
            release(None, None),
            release(Some(other), None),
            release(Some(sums(&packed)), None),
        ] {
            assert_eq!(prebuilt(dir.path(), TARGET, &get), Ok(false));
        }
        assert!(!dir.path().join("bin").exists());
    }

    #[test]
    fn a_failed_download_is_an_error_and_never_a_source_build() {
        let dir = tempfile::tempdir().unwrap();
        let why = prebuilt(dir.path(), TARGET, &|_| Err("connection reset".into())).unwrap_err();
        assert!(
            why.contains("connection reset") && why.contains("Try again"),
            "{why}"
        );
    }

    #[test]
    fn an_archive_that_is_not_the_releases_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let packed = archive(&mixins_tool::identity(TARGET));
        let get = release(Some(sums(b"another archive")), Some(packed));
        let why = prebuilt(dir.path(), TARGET, &get).unwrap_err();
        assert!(
            why.contains("doesn't match its release's checksum"),
            "{why}"
        );
        assert!(!dir.path().join("bin").exists());
    }

    #[test]
    fn a_tool_built_from_another_pin_or_target_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        for identity in [
            mixins_tool::identity("aarch64-apple-darwin"),
            mixins_tool::identity(TARGET).replace(MIXINS_REV, &"0".repeat(40)),
            String::new(),
        ] {
            let packed = archive(&identity);
            let get = release(Some(sums(&packed)), Some(packed));
            let why = prebuilt(dir.path(), TARGET, &get).unwrap_err();
            assert!(
                why.contains("was not built from the source, patch and target"),
                "{why}"
            );
            assert!(!dir.path().join("bin").exists());
        }
    }
}
