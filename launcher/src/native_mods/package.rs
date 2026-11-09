//! Validated flat packages and launcher-owned horizon-mixins input.
use super::{Bundle, MANIFEST};
use horizon_mod_package::{manifest, Package};
use std::{fs, path::Path};

pub(super) fn read(dir: &Path) -> Result<Option<Bundle>, String> {
    for old in [
        "horizon-native.json",
        "horizon-mixin.toml",
        "native",
        "rust",
    ] {
        if dir.join(old).exists() {
            return Err(format!("{}: unsupported package path {old}", dir.display()));
        }
    }
    if !dir.join(manifest::MANIFEST).is_file() {
        return Ok(None);
    }
    let package = Package::read(dir)?;
    if package.manifest.native.is_none() {
        return Ok(None);
    }
    package.directory_files(dir)?;
    Ok(Some(Bundle {
        manifest: package.manifest,
        dir: dir.to_owned(),
    }))
}

/// Generate tool input in the build's incoming copy, never in a modder's project.
pub(super) fn tool_manifest(bundle: &Bundle) -> Result<(), String> {
    let mut manifest = toml::Table::new();
    manifest.insert(
        "mod".into(),
        toml::Value::Table(toml::Table::from_iter([
            ("id".into(), bundle.manifest.id.clone().into()),
            ("version".into(), bundle.manifest.version.clone().into()),
        ])),
    );
    manifest.insert(
        "compatibility".into(),
        toml::Value::Table(toml::Table::from_iter([(
            "horizon_revision".into(),
            bundle.native().horizon.clone().into(),
        )])),
    );
    if !bundle.native().module.is_empty() {
        manifest.insert(
            "module".into(),
            toml::Value::try_from(&bundle.native().module).map_err(|e| e.to_string())?,
        );
    }
    fs::write(
        bundle.dir.join(MANIFEST),
        toml::to_string(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

fn checked_files(bundle: &Bundle) -> Result<std::collections::BTreeMap<String, Vec<u8>>, String> {
    Package::read(&bundle.dir)?.directory_files(&bundle.dir)
}

pub(super) fn files(bundle: &Bundle) -> Result<Vec<(String, std::path::PathBuf)>, String> {
    Ok(checked_files(bundle)?
        .into_keys()
        .map(|name| {
            let path = bundle.dir.join(&name);
            (name, path)
        })
        .collect())
}

pub(super) fn copy(bundle: &Bundle, to: &Path) -> Result<(), String> {
    copy_files(to, checked_files(bundle)?)
}

/// Only code enters the tool; runtime resource packs stay with the package.
pub(super) fn copy_tool_input(bundle: &Bundle, to: &Path) -> Result<(), String> {
    copy_files(
        to,
        checked_files(bundle)?
            .into_iter()
            .filter(|(name, _)| manifest::source_file(name)),
    )
}

fn copy_files(to: &Path, files: impl IntoIterator<Item = (String, Vec<u8>)>) -> Result<(), String> {
    for (name, bytes) in files {
        let path = to.join(name);
        fs::create_dir_all(path.parent().ok_or("invalid destination")?)
            .map_err(|e| e.to_string())?;
        fs::write(path, bytes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Check target-crossing edges before the native-only dependency resolver.
pub(super) fn checked_bundles(folder: &Path) -> Result<Vec<Bundle>, String> {
    let found = super::scan_bundles(folder)?;
    let wasm: Vec<_> = horizon_mod_package::find(folder)
        .into_iter()
        .chain(horizon_mod_package::find(
            &folder
                .parent()
                .unwrap_or(folder)
                .join(horizon_mod_package::mod_list::DIR),
        ))
        .filter_map(|path| horizon_mod_package::Package::read(&path).ok())
        .filter(|package| package.manifest.wasm.is_some())
        .collect();
    for bundle in &found {
        let dependent = horizon_mod_package::Package::read(&bundle.dir)?;
        for library in &wasm {
            if !dependent
                .manifest
                .dependencies
                .contains_key(&library.manifest.id)
            {
                continue;
            }
            let reason = if let Some(name) = dependent
                .layouts
                .first_difference(&library.manifest.id, &library.layouts)
            {
                crate::i18n::trf(
                    "{mod}: layout mismatch in library {library} for type {type}",
                    &[
                        ("mod", &dependent.manifest.id),
                        ("library", &library.manifest.id),
                        ("type", name),
                    ],
                )
            } else {
                crate::i18n::trf("{library} is a wasm mod; native mod {mod} depends on it: mixed native and wasm dependencies aren't supported yet", &[("library", &library.manifest.id), ("mod", &dependent.manifest.id)])
            };
            return Err(reason);
        }
    }
    Ok(found)
}
