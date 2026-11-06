//! Physical game areas share one logical module scope; compiler and source tools use this list.
use crate::catalog::Catalog;
use std::{collections::BTreeMap, fs, path::Path};

pub(crate) const INCLUDE: &str = "include!(concat!(env!(\"OUT_DIR\"), \"/game_modules.rs\"));";

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
            if path.is_dir() && path.file_name().is_none_or(|name| name != "native_hooks") {
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
            let cfg = if name.ends_with("_tests")
                && !source
                    .lines()
                    .any(|line| line == "// game-module-cfg: always")
            {
                "#[cfg(test)]\n"
            } else {
                ""
            };
            let absolute = file.canonicalize().map_err(|e| e.to_string())?;
            let declaration = format!(
                "{cfg}#[path = {:?}]\n{visibility} mod {name};\n",
                absolute.to_string_lossy()
            );
            if modules.insert(name.clone(), declaration).is_some() {
                return Err(format!("duplicate game module {name}"));
            }
        }
    }
    Ok(modules.into_values().collect())
}

/// Expand the same directory-derived include before source traversal, without needing OUT_DIR.
/// Other includes are left alone: source tools must not evaluate arbitrary build-time code.
pub(crate) fn expand(path: &Path, source: &str) -> Result<String, String> {
    const NATIVE: &str = "include!(concat!(env!(\"OUT_DIR\"), \"/native_hook_modules.rs\"));";
    if !source.contains(INCLUDE) && !source.contains(NATIVE) {
        return Ok(source.to_owned());
    }
    let file = syn::parse_file(source).map_err(|e| e.to_string())?;
    let mut lines: Vec<_> = source.split_inclusive('\n').map(str::to_owned).collect();
    for item in file.items {
        if let syn::Item::Macro(item) = item
            && item.mac.path.is_ident("include")
        {
            let line = item.mac.path.segments[0].ident.span().start().line - 1;
            let declarations = match lines[line].trim_end_matches('\n') {
                INCLUDE => declarations(&path.with_extension(""))?,
                NATIVE => crate::build::native_hook_modules(
                    path.canonicalize()
                        .map_err(|e| e.to_string())?
                        .ancestors()
                        .nth(6)
                        .ok_or("native hook source outside the game tree")?,
                )?,
                _ => continue,
            };
            // OUT_DIR lists need absolute paths, but materialized source must survive
            // the launcher's stage.new -> stage copy and removal of the original tree.
            let directory = path
                .parent()
                .ok_or("game source has no parent")?
                .canonicalize()
                .map_err(|e| e.to_string())?;
            let mut declarations = syn::parse_file(&declarations).map_err(|e| e.to_string())?;
            for item in &mut declarations.items {
                let syn::Item::Mod(item) = item else { continue };
                for attribute in &mut item.attrs {
                    if let syn::Meta::NameValue(value) = &mut attribute.meta
                        && value.path.is_ident("path")
                        && let syn::Expr::Lit(value) = &mut value.value
                        && let syn::Lit::Str(literal) = &mut value.lit
                    {
                        let absolute = literal.value();
                        let relative = Path::new(&absolute)
                            .strip_prefix(&directory)
                            .map_err(|e| e.to_string())?
                            .to_string_lossy();
                        *literal = syn::LitStr::new(&relative, literal.span());
                    }
                }
            }
            lines[line] = prettyplease::unparse(&declarations);
        }
    }
    Ok(lines.concat())
}

pub(crate) fn parse(path: &Path, source: &str) -> Result<syn::File, String> {
    syn::parse_file(&expand(path, source)?).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_game_exports_keep_their_visibility() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../horizon-client/src");
        let code = declarations(&src.join("game")).unwrap();
        for (name, visibility) in [
            ("cushions", "pub(crate)"),
            ("mods", "pub(crate)"),
            ("native_block_entity_draw", "pub"),
            ("native_client_services", "pub"),
        ] {
            assert!(code.contains(&format!("{visibility} mod {name};")));
        }
        let files = crate::points::game_files(&src, true).unwrap();
        let game_tests = "horizon_client::game::game_tests";
        assert!(files.iter().any(|file| file.module == game_tests));
    }

    #[test]
    fn discovery_preserves_scope_cfg_and_visibility_and_rejects_drift() {
        let root = std::env::temp_dir().join(format!("game-areas-{}", std::process::id()));
        let game = root.join("game");
        let area = game.join("network");
        fs::create_dir_all(&area).unwrap();
        fs::write(area.join(".game-area"), "").unwrap();
        let source = format!("const DOC: &str = r#\"\n{INCLUDE}\n\"#;\n{INCLUDE}");
        fs::write(root.join("game.rs"), &source).unwrap();
        assert!(
            expand(&root.join("game.rs"), &source)
                .unwrap()
                .contains(INCLUDE)
        );
        fs::create_dir_all(area.join("remote")).unwrap();
        fs::write(area.join("remote/mod.rs"), "mod child;").unwrap();
        fs::write(area.join("remote/child.rs"), "").unwrap();
        fs::write(area.join("probe_tests.rs"), "").unwrap();
        fs::write(area.join("game_tests.rs"), "// game-module-cfg: always\n").unwrap();
        fs::write(
            area.join("service.rs"),
            "// game-module-visibility: pub(crate)\n",
        )
        .unwrap();
        let files = crate::points::game_files(&root, false).unwrap();
        let names: Vec<_> = files.iter().map(|file| file.module.as_str()).collect();
        assert!(names.contains(&"horizon_client::game::remote::child"));
        assert!(names.contains(&"horizon_client::game::game_tests"));
        assert!(
            !names
                .iter()
                .any(|name| name.contains("probe_tests") || name.contains("network"))
        );
        let code = declarations(&game).unwrap();
        assert!(code.contains("pub(crate) mod service;"));
        fs::write(area.join("new_feature.rs"), "").unwrap();
        assert!(declarations(&game).unwrap().contains("mod new_feature;"));
        fs::write(game.join("loose.rs"), "").unwrap();
        assert!(declarations(&game).unwrap_err().contains("area folder"));
        fs::remove_file(game.join("loose.rs")).unwrap();
        let other = game.join("world");
        fs::create_dir(&other).unwrap();
        fs::write(other.join(".game-area"), "").unwrap();
        fs::write(other.join("remote.rs"), "").unwrap();
        assert!(
            declarations(&game)
                .unwrap_err()
                .contains("duplicate game module remote")
        );
        fs::remove_dir_all(root).unwrap();
    }
}
