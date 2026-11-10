//! Directory-derived game module declarations.
use std::{collections::BTreeMap, fs, path::Path};

pub(crate) fn declarations(game: &Path) -> Result<String, String> {
    let mut modules = BTreeMap::new();
    for entry in fs::read_dir(game).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            if path
                .file_name()
                .is_some_and(|name| name == "native_hooks.rs")
            {
                continue;
            }
            return Err(format!(
                "{}: put game module roots in an area folder",
                path.display()
            ));
        }
        if !path.join(".game-area").is_file() {
            if path.is_dir()
                && path
                    .file_name()
                    .is_none_or(|name| name != "native_hooks" && name != "host")
            {
                return Err(format!("{}: missing .game-area marker", path.display()));
            }
            continue;
        }
        let mut roots: Vec<_> = Catalog::modules(&path)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|name| (name.clone(), path.join(format!("{name}.rs"))))
            .collect();
        for child in fs::read_dir(&path).map_err(|e| e.to_string())? {
            let child = child.map_err(|e| e.to_string())?;
            let file = child.path().join("mod.rs");
            if file.is_file() {
                let name = child.file_name().to_string_lossy().into_owned();
                syn::parse_str::<syn::Ident>(&name).map_err(|e| e.to_string())?;
                roots.push((name, file));
            }
        }
        for (name, file) in roots {
            let source = fs::read_to_string(&file).map_err(|e| e.to_string())?;
            let visibility = source
                .lines()
                .find_map(|line| line.strip_prefix("// game-module-visibility: "))
                .unwrap_or("");
            if !matches!(visibility, "" | "pub" | "pub(crate)") {
                return Err(format!(
                    "{}: invalid game module visibility",
                    file.display()
                ));
            }
            let cfg = if name == "friends" {
                "#[cfg(feature = \"friends\")]\n"
            } else if name == "friends_off" {
                "#[cfg(not(feature = \"friends\"))]\n"
            } else if name.ends_with("_tests")
                && !source
                    .lines()
                    .any(|line| line == "// game-module-cfg: always")
            {
                "#[cfg(test)]\n"
            } else {
                ""
            };
            let absolute = file.canonicalize().map_err(|e| e.to_string())?;
            let declaration_name = if name == "friends_off" {
                "friends".to_owned()
            } else {
                name.clone()
            };
            let declaration = format!(
                "{cfg}#[path = {:?}]\n{visibility} mod {declaration_name};\n",
                absolute.to_string_lossy()
            );
            if modules.insert(name.clone(), declaration).is_some() {
                return Err(format!("duplicate game module {name}"));
            }
        }
    }
    Ok(modules.into_values().collect())
}

