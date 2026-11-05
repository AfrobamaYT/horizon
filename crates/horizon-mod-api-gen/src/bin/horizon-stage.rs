//! `horizon-stage ROOT POINTS_JSON INTERNALS_JSON`: mark points and expose internals in a staged
//! source copy, then write both manifests (#589). Never run on a working tree.
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

/// One file per mod; sorting and deduplication make the union independent of directory order.
fn declarations(root: &Path) -> Result<Vec<String>, String> {
    let directory = root.join("crates/horizon-mod-api-gen/wasm-internals");
    let files = match std::fs::read_dir(&directory) {
        Ok(files) => files,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("{}: {error}", directory.display())),
    };
    let mut paths = BTreeSet::new();
    for file in files {
        let file = file
            .map_err(|e| format!("{}: {e}", directory.display()))?
            .path();
        if file.extension().is_none_or(|ext| ext != "txt") {
            continue;
        }
        let text =
            std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        for line in text.lines() {
            let path = line.split('#').next().unwrap_or_default().trim();
            if !path.is_empty() {
                paths.insert(path.to_owned());
            }
        }
    }
    Ok(paths.into_iter().collect())
}

/// Keep unsafe forbidden everywhere except the generated table, in the staged copy only.
fn with_internals_table(source: &str) -> String {
    source.replace("#![forbid(unsafe_code)]", "#![deny(unsafe_code)]")
}

fn main() {
    let args: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let [root, list, internals] = &args[..] else {
        eprintln!("usage: horizon-stage ROOT POINTS_JSON INTERNALS_JSON");
        std::process::exit(2);
    };
    // Internals first, so the manifest lists the game's own items and not the points' raw types.
    let staged = horizon_mod_api_gen::internals::widen(root).and_then(|widened| {
        let text = serde_json::to_string_pretty(&widened.entries).map_err(|e| e.to_string())?;
        std::fs::write(internals, text + "\n")
            .map_err(|e| format!("{}: {e}", internals.display()))?;
        // The table a wasm mod reaches the internals through, at the game crate's root.
        let declared = declarations(root)?;
        let points = horizon_mod_api_gen::points::game_points(root)?;
        let (table, counts) = horizon_mod_api_gen::internals_table::table_with_points(
            &widened.entries,
            &declared,
            &points,
        );
        let lib = root.join("crates/horizon-client/src/lib.rs");
        let text = std::fs::read_to_string(&lib).map_err(|e| format!("{}: {e}", lib.display()))?;
        std::fs::write(&lib, with_internals_table(&text))
            .map_err(|e| format!("{}: {e}", lib.display()))?;
        let module = root.join("crates/horizon-client/src/__internals.rs");
        std::fs::write(&module, table).map_err(|e| format!("{}: {e}", module.display()))?;
        println!(
            "internals table: {} types, {} thunks from {} declared paths ({} skipped)",
            counts.types, counts.thunks, counts.declared, counts.skipped
        );
        for (path, reason) in &counts.refused {
            println!("internals table: skipped {path}: {reason}");
        }
        let staged = horizon_mod_api_gen::points::stage(root)?;
        let text = serde_json::to_string_pretty(&points).map_err(|e| e.to_string())?;
        std::fs::write(list, text + "\n").map_err(|e| format!("{}: {e}", list.display()))?;
        Ok((staged, widened))
    });
    match staged {
        Ok((staged, widened)) => {
            println!(
                "staged {} points ({} with a head only, {} with a raw form) in {} files",
                staged.points, staged.head_only, staged.raw, staged.files
            );
            let shapes = widened
                .entries
                .iter()
                .filter(|entry| entry.shape_only)
                .count();
            println!(
                "exposed {} internals and listed {} type shapes in {} changed files: {:?}",
                widened.entries.len() - shapes,
                shapes,
                widened.files,
                widened.changed
            );
        }
        Err(error) => {
            eprintln!("horizon-stage: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declaration_files_form_a_deterministic_union() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(format!("wasm-declarations-{}", std::process::id()));
        assert!(declarations(&root).unwrap().is_empty());
        let directory = root.join("crates/horizon-mod-api-gen/wasm-internals");
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("first.txt"),
            "# whole comment\nz\na # inline\n\n",
        )
        .unwrap();
        std::fs::write(directory.join("second.txt"), "a\nb\n").unwrap();
        std::fs::write(directory.join("README.md"), "not a path").unwrap();
        assert_eq!(declarations(&root).unwrap(), ["a", "b", "z"]);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn the_staged_root_allows_only_the_table_to_override_unsafe() {
        let source = "#![forbid(unsafe_code)]\npub mod game;";
        let staged = with_internals_table(source);
        assert!(staged.starts_with("#![deny(unsafe_code)]\npub mod game;"));
        assert!(!staged.contains("forbid(unsafe_code)"));
        assert!(!staged.contains("pub mod __internals"));
        assert!(source.contains("forbid(unsafe_code)"));
    }
}
