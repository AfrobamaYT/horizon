//! A mod folder as `horizon-stage uses` scans it: its Rust sources and what its manifest names.
use std::path::Path;

/// The largest Rust source file read, in bytes; a larger one is refused, never truncated.
pub const MAX_SOURCE_BYTES: u64 = 8 << 20;

/// What [`read`] found in a mod folder.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ModInput {
    /// Each `.rs` file's path, relative and `/`-separated, with its text, sorted by path.
    pub sources: Vec<(String, String)>,
    /// The capability keys `horizon-mod.toml` lists.
    pub manifest_names: Vec<String>,
    /// Why the mod is made for exactly one build, when it is: a `[native.*]` table or internals.
    pub exact: Option<String>,
}

/// Reads `dir`: every `.rs` file outside `target/` and hidden folders, and `horizon-mod.toml` when
/// there is one.
pub fn read(dir: &Path) -> Result<ModInput, String> {
    let mut input = ModInput::default();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(folder) = pending.pop() {
        let entries =
            std::fs::read_dir(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("{}: {e}", folder.display()))?;
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let kind = entry
                .file_type()
                .map_err(|e| format!("{}: {e}", path.display()))?;
            if kind.is_dir() {
                if !name.starts_with('.') && name != "target" {
                    pending.push(path);
                }
            } else if kind.is_file() && name.ends_with(".rs") {
                let size = entry
                    .metadata()
                    .map_err(|e| format!("{}: {e}", path.display()))?
                    .len();
                if size > MAX_SOURCE_BYTES {
                    return Err(format!(
                        "{}: {size} bytes; the most read is {MAX_SOURCE_BYTES}",
                        path.display()
                    ));
                }
                let text = std::fs::read_to_string(&path)
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                let relative = path.strip_prefix(dir).unwrap_or(&path);
                let relative: Vec<String> = relative
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();
                input.sources.push((relative.join("/"), text));
            }
        }
    }
    input.sources.sort();
    let manifest = dir.join("horizon-mod.toml");
    if manifest.is_file() {
        let text = std::fs::read_to_string(&manifest)
            .map_err(|e| format!("{}: {e}", manifest.display()))?;
        let value: toml::Table =
            toml::from_str(&text).map_err(|e| format!("{}: {e}", manifest.display()))?;
        if let Some(capabilities) = value.get("capabilities").and_then(toml::Value::as_array) {
            input.manifest_names = capabilities
                .iter()
                .filter_map(toml::Value::as_str)
                .map(str::to_owned)
                .collect();
        }
        let native_tables: Vec<&String> = value
            .get("native")
            .and_then(toml::Value::as_table)
            .map(|native| {
                native
                    .iter()
                    .filter(|(_, v)| v.is_table() || v.is_array())
                    .map(|(k, _)| k)
                    .collect()
            })
            .unwrap_or_default();
        let internals = value
            .get("internals")
            .and_then(toml::Value::as_array)
            .is_some_and(|rows| !rows.is_empty());
        if let Some(table) = native_tables.first() {
            input.exact = Some(format!("it has a [native.{table}] table"));
        } else if internals {
            input.exact = Some("it names game internals".to_owned());
        }
    }
    Ok(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, path: &str, text: &str) {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn reads_sources_outside_target_and_hidden_folders_and_the_manifest_names() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "src/lib.rs", "a");
        write(root, "src/deep/b.rs", "b");
        write(root, "build.rs", "c");
        write(root, "target/debug/build/x.rs", "no");
        write(root, ".horizon/sdk/api/lib.rs", "no");
        write(root, "src/notes.txt", "no");
        write(
            root,
            "horizon-mod.toml",
            "id = \"m\"\ncapabilities = [\"registries\"]\n[native]\ngame = \"26.3\"\n",
        );
        let input = read(root).unwrap();
        let sources: Vec<(&str, &str)> = input
            .sources
            .iter()
            .map(|(p, t)| (p.as_str(), t.as_str()))
            .collect();
        assert_eq!(
            sources,
            [
                ("build.rs", "c"),
                ("src/deep/b.rs", "b"),
                ("src/lib.rs", "a")
            ]
        );
        assert_eq!(input.manifest_names, ["registries"]);
        assert_eq!(input.exact, None);
    }

    #[test]
    fn a_native_table_or_internals_make_the_mod_exact() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "horizon-mod.toml",
            "capabilities = []\n[native]\ngame = \"26.3\"\n[[native.mixin]]\nid = \"x\"\n",
        );
        assert_eq!(
            read(dir.path()).unwrap().exact.as_deref(),
            Some("it has a [native.mixin] table")
        );
        write(
            dir.path(),
            "horizon-mod.toml",
            "capabilities = []\n[[internals]]\npath = \"x\"\n",
        );
        assert_eq!(
            read(dir.path()).unwrap().exact.as_deref(),
            Some("it names game internals")
        );
    }

    #[test]
    fn an_unreadable_manifest_is_an_error_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "horizon-mod.toml", "capabilities = [");
        assert!(read(dir.path()).unwrap_err().contains("horizon-mod.toml"));
    }
}
