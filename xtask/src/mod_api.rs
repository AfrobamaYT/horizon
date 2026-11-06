//! `mod-api`: fill new native hooks' placeholder bodies in the hand-written source.
//! `mod-api wit <dir>`: write the mod API's generated files (`mod-api.wit` among them) to `<dir>`,
//! as the crate's build generates them, for CI's artifact and interface report.
use std::path::{Path, PathBuf};

fn checkout() -> Result<PathBuf, String> {
    let root = std::env::current_dir().map_err(|e| e.to_string())?;
    root.ancestors()
        .find(|path| path.join("crates/horizon-mod-api/src/hooks").is_dir())
        .map(Path::to_path_buf)
        .ok_or("run xtask from within the Horizon checkout".into())
}

pub fn run() -> Result<(), String> {
    horizon_mod_api_gen::fill_hooks(&checkout()?)
}

/// Writes `docs/modding/src/reference/hook-notes.md` from the declarations' notes.
pub fn notes() -> Result<(), String> {
    let root = checkout()?;
    let path = root.join("docs/modding/src/reference/hook-notes.md");
    let page = horizon_mod_api_gen::generate_hook_notes(&root)?;
    std::fs::write(&path, page).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}

pub fn wit(out: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    horizon_mod_api_gen::generate(&checkout()?, out, horizon_mod_api_gen::Target::Api)
}
