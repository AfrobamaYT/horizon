//! Resolve a mod set's passive Java jars before a launcher-managed launch.
use crate::i18n::{tr, trf};
use horizon_mods::manifest::JarRef;
use horizon_port_pack::{fetch, modrinth};
use std::{collections::BTreeMap, fmt, fs, io, path::Path};

/// Download consent for this mod set, never a permission to execute Java code.
#[non_exhaustive]
pub enum Permission<'a> {
    Refuse,
    Allow,
    Ask(&'a dyn Fn(&[&JarRef]) -> bool),
}

/// The same file cannot claim two different Java mod identities.
#[derive(Debug)]
pub struct JarConflict {
    pub first: Box<JarRef>,
    pub second: Box<JarRef>,
}

impl fmt::Display for JarConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&trf("SHA-512 {hash} names both {first_id} {first_version} and {second_id} {second_version}.", &[
            ("hash", self.first.sha512()), ("first_id", self.first.id()), ("first_version", self.first.version()),
            ("second_id", self.second.id()), ("second_version", self.second.version()),
        ]))
    }
}
impl std::error::Error for JarConflict {}

/// Collect once from the staged mod set, including content-only packages.
/// This order is only for fetching; pack layering remains per mod in the game.
pub fn collect<'a>(
    manifests: impl IntoIterator<Item = &'a horizon_mods::manifest::Manifest>,
) -> Result<Vec<JarRef>, JarConflict> {
    let mut hashes = BTreeMap::new();
    for jar in manifests.into_iter().flat_map(|m| &m.jar) {
        if let Some(first) = hashes.insert(jar.sha512(), jar) {
            if first != jar {
                return Err(JarConflict {
                    first: Box::new(first.clone()),
                    second: Box::new(jar.clone()),
                });
            }
        }
    }
    Ok(hashes.into_values().cloned().collect())
}

/// Cache hits work offline. Missing jars are resolved by exact SHA-512 and
/// downloaded only after one consent decision for the complete mod set.
pub fn prepare(home: &Path, jars: &[JarRef], permission: Permission<'_>) -> Result<(), String> {
    prepare_with(
        home,
        jars,
        permission,
        modrinth::jar_url,
        fetch::download_jar,
    )
}

fn prepare_with(
    home: &Path,
    jars: &[JarRef],
    permission: Permission<'_>,
    mut lookup: impl FnMut(&JarRef) -> Result<Option<String>, String>,
    mut download: impl FnMut(&str, &JarRef, &Path) -> Result<(), String>,
) -> Result<(), String> {
    let cache = home.join("jars");
    let mut missing = Vec::new();
    for jar in jars {
        let path = cache.join(format!("{}.jar", jar.sha512()));
        match fs::File::open(&path) {
            Ok(file) => {
                let hash = fetch::hash_reader::<sha2::Sha512>(file, io::sink())
                    .map_err(|e| instructions(jar, &path, &e.to_string()))?;
                if format!("{hash:x}") != jar.sha512() {
                    return Err(instructions(jar, &path, tr("SHA-512 mismatch")));
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => missing.push(jar),
            Err(e) => return Err(instructions(jar, &path, &e.to_string())),
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    let first = missing[0];
    let path = cache.join(format!("{}.jar", first.sha512()));
    if matches!(permission, Permission::Refuse) {
        return Err(instructions(
            first,
            &path,
            tr("jar downloads are not allowed; use --allow-jar-downloads"),
        ));
    }
    let urls: Vec<String> = missing
        .iter()
        .map(|jar| {
            let path = cache.join(format!("{}.jar", jar.sha512()));
            lookup(jar)
                .map_err(|e| instructions(jar, &path, &e))?
                .ok_or_else(|| instructions(jar, &path, tr("no exact file on Modrinth")))
        })
        .collect::<Result<_, _>>()?;
    if let Permission::Ask(ask) = permission {
        if !ask(&missing) {
            return Err(instructions(
                first,
                &path,
                tr("jar downloads were declined"),
            ));
        }
    }
    fs::create_dir_all(&cache).map_err(|e| instructions(first, &path, &e.to_string()))?;
    for (jar, url) in missing.into_iter().zip(urls) {
        let path = cache.join(format!("{}.jar", jar.sha512()));
        download(&url, jar, &path).map_err(|e| instructions(jar, &path, &e))?;
    }
    Ok(())
}

fn instructions(jar: &JarRef, path: &Path, reason: &str) -> String {
    trf(
        "Jar {id} {version}: {reason}. Put the original file with this exact SHA-512 at {path}, or allow jar downloads.",
        &[("id", jar.id()), ("version", jar.version()), ("reason", reason), ("path", &path.display().to_string())],
    )
}

#[cfg(test)]
#[path = "jars_tests.rs"]
mod tests;
