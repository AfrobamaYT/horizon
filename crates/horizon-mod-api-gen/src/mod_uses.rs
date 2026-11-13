//! A mod folder as `horizon-stage uses` scans it: its Rust sources and what its manifest names.
use mod_surface::scan::{Token, tokens};
use std::path::{Component, Path, PathBuf};

/// The largest Rust source file read, in bytes; a larger one is refused, never truncated.
pub const MAX_SOURCE_BYTES: u64 = 8 << 20;
/// The most Rust source read from one mod, in bytes.
pub const MAX_TOTAL_BYTES: u64 = 64 << 20;
/// The most Rust source files read from one mod.
pub const MAX_FILES: usize = 4096;

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

/// Why a mod folder cannot be judged.
#[derive(Debug, PartialEq, Eq)]
pub enum ReadError {
    /// A file or folder could not be read.
    Io { path: PathBuf, error: String },
    /// A symbolic link in the folder: what it points to is not the mod's own.
    Symlink(PathBuf),
    /// A `#[path]` or `include!` in `file` brings in source from outside the folder, or from a
    /// place the scan cannot know.
    Escapes { file: String, target: String },
    /// More source than is read: a file, or the whole mod.
    TooLarge { what: String, size: u64, most: u64 },
    /// `horizon-mod.toml` is not valid TOML.
    Manifest(String),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, error } => write!(f, "{}: {error}", path.display()),
            Self::Symlink(path) => write!(
                f,
                "{} is a symbolic link; the mod's source cannot be judged",
                path.display()
            ),
            Self::Escapes { file, target } => write!(
                f,
                "{file} brings in `{target}`, outside the mod's folder; the mod's source cannot be judged"
            ),
            Self::TooLarge { what, size, most } => {
                write!(f, "{what} is {size} bytes; the most read is {most}")
            }
            Self::Manifest(error) => write!(f, "horizon-mod.toml: {error}"),
        }
    }
}

impl std::error::Error for ReadError {}

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> ReadError + '_ {
    move |error| ReadError::Io {
        path: path.to_path_buf(),
        error: error.to_string(),
    }
}

/// `target` joined to `base` (both relative to the mod's folder), if it stays inside it.
fn inside(base: &[String], target: &str) -> bool {
    if target.starts_with('/') || target.starts_with('\\') || target.contains(':') {
        return false;
    }
    let mut parts: Vec<&str> = base.iter().map(String::as_str).collect();
    for component in Path::new(target).components() {
        match component {
            Component::Normal(name) => match name.to_str() {
                Some(name) => parts.push(name),
                None => return false,
            },
            Component::CurDir => {}
            Component::ParentDir => {
                if parts.pop().is_none() {
                    return false;
                }
            }
            Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

/// Whether the token at `at` sits inside an attribute's brackets, `#[…]` or `#![…]`.
fn in_attribute(list: &[Token<'_>], at: usize) -> bool {
    let mut depth = 0usize;
    for index in (0..at).rev() {
        match list[index] {
            Token::Punct(b')' | b']' | b'}') => depth += 1,
            Token::Punct(b'(' | b'[' | b'{') if depth > 0 => depth -= 1,
            Token::Punct(b'[') => {
                return index >= 1
                    && (list[index - 1] == Token::Punct(b'#')
                        || (index >= 2
                            && list[index - 1] == Token::Punct(b'!')
                            && list[index - 2] == Token::Punct(b'#')));
            }
            _ => {}
        }
    }
    false
}

/// The first `#[path = "…"]` or `include!(…)` in `text` that leaves the folder, given the file's
/// folder relative to the mod's.
fn escape(text: &str, folder: &[String]) -> Option<String> {
    let list: Vec<Token<'_>> = tokens(text).map(|(t, _)| t).collect();
    let literal = |at: usize| match list.get(at) {
        Some(Token::Literal(text))
            if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 =>
        {
            Some(&text[1..text.len() - 1])
        }
        _ => None,
    };
    for at in 0..list.len() {
        match list[at] {
            // `path = "…"` anywhere inside an attribute, `cfg_attr(…, path = …)` included.
            Token::Ident("path")
                if list.get(at + 1) == Some(&Token::Punct(b'='))
                    && list.get(at + 2) != Some(&Token::Punct(b'='))
                    && in_attribute(&list, at) =>
            {
                match literal(at + 2) {
                    Some(target) if inside(folder, target) => {}
                    Some(target) => return Some(target.to_owned()),
                    None => return Some("a path the scan cannot read".to_owned()),
                }
            }
            Token::Ident("include")
                if list.get(at + 1) == Some(&Token::Punct(b'!'))
                    && list.get(at + 2) == Some(&Token::Punct(b'(')) =>
            {
                match (literal(at + 3), list.get(at + 4)) {
                    (Some(target), Some(Token::Punct(b')'))) if inside(folder, target) => {}
                    (Some(target), Some(Token::Punct(b')'))) => return Some(target.to_owned()),
                    _ => return Some("a computed path".to_owned()),
                }
            }
            _ => {}
        }
    }
    None
}

/// Reads `dir`: every `.rs` file outside the mod's `target/` and hidden folders, and
/// `horizon-mod.toml` when there is one.
pub fn read(dir: &Path) -> Result<ModInput, ReadError> {
    let mut input = ModInput::default();
    let mut total = 0u64;
    let mut pending = vec![dir.to_path_buf()];
    while let Some(folder) = pending.pop() {
        for entry in std::fs::read_dir(&folder).map_err(io(&folder))? {
            let entry = entry.map_err(io(&folder))?;
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let kind = entry.file_type().map_err(io(&path))?;
            if kind.is_symlink() {
                return Err(ReadError::Symlink(path));
            }
            if kind.is_dir() {
                let build_output = folder == dir && name == "target";
                if !name.starts_with('.') && !build_output {
                    pending.push(path);
                }
            } else if kind.is_file() && name.ends_with(".rs") {
                let size = entry.metadata().map_err(io(&path))?.len();
                let relative: Vec<String> = path
                    .strip_prefix(dir)
                    .unwrap_or(&path)
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();
                let shown = relative.join("/");
                if size > MAX_SOURCE_BYTES {
                    return Err(ReadError::TooLarge {
                        what: shown,
                        size,
                        most: MAX_SOURCE_BYTES,
                    });
                }
                total += size;
                if total > MAX_TOTAL_BYTES || input.sources.len() >= MAX_FILES {
                    return Err(ReadError::TooLarge {
                        what: format!("the mod's source ({} files)", input.sources.len() + 1),
                        size: total,
                        most: MAX_TOTAL_BYTES,
                    });
                }
                let text = std::fs::read_to_string(&path).map_err(io(&path))?;
                if let Some(target) = escape(&text, &relative[..relative.len() - 1]) {
                    return Err(ReadError::Escapes {
                        file: shown,
                        target,
                    });
                }
                input.sources.push((shown, text));
            }
        }
    }
    input.sources.sort();
    let manifest = dir.join("horizon-mod.toml");
    if manifest.is_file() {
        let text = std::fs::read_to_string(&manifest).map_err(io(&manifest))?;
        let value: toml::Table =
            toml::from_str(&text).map_err(|e| ReadError::Manifest(e.to_string()))?;
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
        write(root, "src/target/aim.rs", "d");
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
                ("src/lib.rs", "a"),
                ("src/target/aim.rs", "d")
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
        assert!(matches!(
            read(dir.path()).unwrap_err(),
            ReadError::Manifest(_)
        ));
    }

    #[test]
    fn source_from_outside_the_folder_cannot_be_judged() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // Inside the folder: fine.
        write(
            root,
            "src/lib.rs",
            "#[path = \"gen/x.rs\"] mod x;\ninclude!(\"../src/y.rs\");\n// #[path = \"/etc\"]",
        );
        assert!(read(root).is_ok());
        let escapes = |text: &str| {
            write(root, "src/lib.rs", text);
            read(root).unwrap_err()
        };
        assert_eq!(
            escapes("#[path = \"../../other/x.rs\"] mod x;"),
            ReadError::Escapes {
                file: "src/lib.rs".into(),
                target: "../../other/x.rs".into()
            }
        );
        assert!(matches!(
            escapes("#[path = \"/abs/x.rs\"] mod x;"),
            ReadError::Escapes { .. }
        ));
        assert_eq!(
            escapes("#[cfg_attr(all(), path = \"../../elsewhere/x.rs\")] mod x;"),
            ReadError::Escapes {
                file: "src/lib.rs".into(),
                target: "../../elsewhere/x.rs".into()
            }
        );
        assert_eq!(
            escapes("include!(concat!(env!(\"OUT_DIR\"), \"/gen.rs\"));"),
            ReadError::Escapes {
                file: "src/lib.rs".into(),
                target: "a computed path".into()
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_symbolic_link_cannot_be_judged() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "src/lib.rs", "a");
        std::os::unix::fs::symlink(root.join("src"), root.join("linked")).unwrap();
        assert_eq!(
            read(root).unwrap_err(),
            ReadError::Symlink(root.join("linked"))
        );
    }
}
