//! `horizon-stage ROOT POINTS_JSON`: the staging pass over the staged copy of the game's source at
//! `ROOT`, which the prebuilt game's build and the launcher's source builds run before compiling
//! it: every point method gets its `#[point]` (#589), and `POINTS_JSON` lists the points, the list
//! the launcher routes injections by. Never run on a working tree.
use std::path::PathBuf;

fn main() {
    let args: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let [root, list] = &args[..] else {
        eprintln!("usage: horizon-stage ROOT POINTS_JSON");
        std::process::exit(2);
    };
    let staged = horizon_mod_api_gen::points::stage(root).and_then(|staged| {
        let points = horizon_mod_api_gen::points::game_points(root)?;
        let text = serde_json::to_string_pretty(&points).map_err(|e| e.to_string())?;
        std::fs::write(list, text + "\n").map_err(|e| format!("{}: {e}", list.display()))?;
        Ok(staged)
    });
    match staged {
        Ok(staged) => println!(
            "staged {} points ({} with a head only) in {} files",
            staged.points, staged.head_only, staged.files
        ),
        Err(error) => {
            eprintln!("horizon-stage: {error}");
            std::process::exit(1);
        }
    }
}
