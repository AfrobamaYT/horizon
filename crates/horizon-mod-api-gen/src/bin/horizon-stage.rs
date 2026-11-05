//! `horizon-stage ROOT`: the staging pass over the staged copy of the game's source at `ROOT`,
//! which the prebuilt game's build and the launcher's source builds run before compiling it:
//! every point method gets its `#[point]` (#589). Never run on a working tree.
fn main() {
    let Some(root) = std::env::args_os().nth(1) else {
        eprintln!("usage: horizon-stage ROOT");
        std::process::exit(2);
    };
    match horizon_mod_api_gen::points::stage(std::path::Path::new(&root)) {
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
