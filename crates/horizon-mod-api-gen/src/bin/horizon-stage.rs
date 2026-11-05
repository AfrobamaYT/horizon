//! `horizon-stage ROOT POINTS_JSON INTERNALS_JSON`: mark points and expose internals in a staged
//! source copy, then write both manifests (#589). Never run on a working tree.
use std::path::PathBuf;

/// Keep unsafe forbidden everywhere except the generated table, in the staged copy only.
fn with_internals_table(source: &str, table: &str) -> String {
    let source = source.replace("#![forbid(unsafe_code)]", "#![deny(unsafe_code)]");
    format!("{source}\n{table}\n")
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
        let (table, counts) = horizon_mod_api_gen::internals_table::table(&widened.entries);
        let lib = root.join("crates/horizon-client/src/lib.rs");
        let text = std::fs::read_to_string(&lib).map_err(|e| format!("{}: {e}", lib.display()))?;
        std::fs::write(&lib, with_internals_table(&text, &table))
            .map_err(|e| format!("{}: {e}", lib.display()))?;
        println!(
            "internals table: {} types, {} thunks, {} functions that can't cross",
            counts.types, counts.thunks, counts.skipped
        );
        for (reason, count) in &counts.skipped_types {
            println!("internals table: skipped {count} types: {reason}");
        }
        for (reason, count) in &counts.skipped_functions {
            println!("internals table: skipped {count} functions: {reason}");
        }
        let staged = horizon_mod_api_gen::points::stage(root)?;
        let points = horizon_mod_api_gen::points::game_points(root)?;
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
    fn the_staged_root_allows_only_the_table_to_override_unsafe() {
        let source = "#![forbid(unsafe_code)]\npub mod game;";
        let staged = with_internals_table(source, "#[allow(unsafe_code)] pub mod __internals {}");
        assert!(staged.starts_with("#![deny(unsafe_code)]\npub mod game;"));
        assert!(!staged.contains("forbid(unsafe_code)"));
        assert!(staged.ends_with("#[allow(unsafe_code)] pub mod __internals {}\n"));
        assert!(source.contains("forbid(unsafe_code)"));
    }
}
