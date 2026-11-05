//! `horizon-stage ROOT POINTS_JSON INTERNALS_JSON`: mark points and expose internals in a staged
//! source copy, then write both manifests (#589). Never run on a working tree.
use std::path::PathBuf;

fn main() {
    let args: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let [root, list, internals] = &args[..] else {
        eprintln!("usage: horizon-stage ROOT POINTS_JSON INTERNALS_JSON");
        std::process::exit(2);
    };
    let staged = horizon_mod_api_gen::points::stage(root).and_then(|staged| {
        let points = horizon_mod_api_gen::points::game_points(root)?;
        let text = serde_json::to_string_pretty(&points).map_err(|e| e.to_string())?;
        std::fs::write(list, text + "\n").map_err(|e| format!("{}: {e}", list.display()))?;
        let widened = horizon_mod_api_gen::internals::widen(root)?;
        let text = serde_json::to_string_pretty(&widened.entries).map_err(|e| e.to_string())?;
        std::fs::write(internals, text + "\n")
            .map_err(|e| format!("{}: {e}", internals.display()))?;
        // The table a wasm mod reaches the internals through, at the game crate's root.
        let (table, counts) = horizon_mod_api_gen::internals_table::table(&widened.entries);
        let lib = root.join("crates/horizon-client/src/lib.rs");
        let text = std::fs::read_to_string(&lib).map_err(|e| format!("{}: {e}", lib.display()))?;
        std::fs::write(&lib, text + "\n" + &table + "\n")
            .map_err(|e| format!("{}: {e}", lib.display()))?;
        println!(
            "internals table: {} types, {} thunks, {} functions that can't cross",
            counts.types, counts.thunks, counts.skipped
        );
        Ok((staged, widened))
    });
    match staged {
        Ok((staged, widened)) => {
            println!(
                "staged {} points ({} with a head only) in {} files",
                staged.points, staged.head_only, staged.files
            );
            println!(
                "exposed {} internals in {} changed files: {:?}",
                widened.entries.len(),
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
