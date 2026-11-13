//! `horizon-stage ROOT POINTS_JSON INTERNALS_JSON API_VERSION`: expose internals, mark points and give functions their replacement slots in a staged
//! source copy, then write both manifests (#589). Never run on a working tree.
//!
//! `horizon-stage surface ROOT OUT`: write the surface export (`surface.json`) of the source tree
//! at ROOT to OUT, and nothing else. It only reads the tree, so a working tree is fine.
//!
//! `horizon-stage uses SURFACE MOD_DIR [--json OUT --made-for BUILD]`: list
//! what the mod in MOD_DIR uses of SURFACE, each item with the place that brought it in, then
//! how it builds each struct it builds with a literal; with `--json`, write the table a package
//! carries.
//!
//! `horizon-stage fits USES_JSON TARGET_SURFACE`: whether a uses table fits a build's surface,
//! naming each changed item. Exit 0 fits, 1 does not fit, 2 error.
use horizon_mod_api_gen::mod_uses;
use horizon_mod_api_gen::staging::{self, Mode};
use mod_surface::Surface;
use mod_surface::uses::{Fit, ManifestFacts, Origin, UsesTable, fits, uses};
use std::path::PathBuf;

fn surface(root: &std::path::Path, out: &std::path::Path) {
    let started = std::time::Instant::now();
    // The generator writes source paths into `#[path]`s it then reads: they must be absolute.
    let root = match std::fs::canonicalize(root) {
        Ok(root) => root,
        Err(error) => {
            eprintln!("horizon-stage surface: {}: {error}", root.display());
            std::process::exit(1);
        }
    };
    let export = match horizon_mod_api_gen::surface::export(&root) {
        Ok(export) => export,
        Err(error) => {
            eprintln!("horizon-stage surface: {error}");
            std::process::exit(1);
        }
    };
    if let Err(error) = std::fs::write(out, export.surface.to_json()) {
        eprintln!("horizon-stage surface: {}: {error}", out.display());
        std::process::exit(1);
    }
    for note in &export.notes {
        eprintln!("unclassified: {note}");
    }
    let counts: Vec<String> = export
        .counts()
        .iter()
        .map(|(kind, count)| format!("{kind}={count}"))
        .collect();
    println!(
        "surface {} unclassified={} hash={}",
        counts.join(" "),
        export.notes.len(),
        export.surface.hash
    );
    println!(
        "stage_phase=surface seconds={:.3}",
        started.elapsed().as_secs_f64()
    );
}

/// Prints `message` as an error and exits 2.
fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("horizon-stage: {message}");
    std::process::exit(2)
}

fn read_surface(path: &std::path::Path) -> Surface {
    let bytes = std::fs::read(path).unwrap_or_else(|e| fail(format!("{}: {e}", path.display())));
    Surface::read(&bytes).unwrap_or_else(|e| fail(format!("{}: {e}", path.display())))
}

fn uses_command(args: &[PathBuf]) {
    let [surface, dir, options @ ..] = args else {
        fail("usage: horizon-stage uses SURFACE MOD_DIR [--json OUT --made-for BUILD]")
    };
    let (mut json, mut made_for) = (None, None);
    let mut options = options.iter();
    while let Some(option) = options.next() {
        let value = options
            .next()
            .unwrap_or_else(|| fail(format!("{} needs a value", option.display())));
        match option.to_str() {
            Some("--json") => json = Some(value.clone()),
            Some("--made-for") => made_for = value.to_str().map(str::to_owned),
            _ => fail(format!("unknown option {}", option.display())),
        }
    }
    let surface = read_surface(surface);
    let input = mod_uses::read(dir).unwrap_or_else(|e| fail(e));
    if let Some(why) = &input.exact {
        eprintln!(
            "note: this mod is made for exactly one build ({why}); the fit test does not apply"
        );
    }
    // No manifest form names a hook version today (the schema refuses `hook_version`).
    let manifest = ManifestFacts {
        names: input.manifest_names,
        hook_versions: Default::default(),
    };
    let uses = uses(
        input.sources.iter().map(|(p, t)| (p, t)),
        &manifest,
        &surface,
    );
    let mut kinds = std::collections::BTreeMap::<&str, usize>::new();
    for used in &uses.items {
        let place = match &used.origin {
            Origin::Source { file, line } => format!("{file}:{line}"),
            Origin::Manifest => "horizon-mod.toml".to_owned(),
            Origin::Via(id) => format!("via {id}"),
        };
        println!("{}  {}  {place}", used.entry.id, used.entry.hash);
        let kind = used.entry.id.split(':').next().unwrap_or_default();
        *kinds.entry(kind).or_default() += 1;
    }
    let counts: Vec<String> = kinds.iter().map(|(k, n)| format!("{k}={n}")).collect();
    println!("uses {} items: {}", uses.items.len(), counts.join(" "));
    for note in uses.entries().filter_map(|entry| entry.building_note()) {
        println!("{note}");
    }
    if let Some(out) = json {
        let made_for = made_for.unwrap_or_else(|| fail("--json needs --made-for BUILD"));
        std::fs::write(&out, uses.table(made_for).to_json())
            .unwrap_or_else(|e| fail(format!("{}: {e}", out.display())));
    }
}

fn fits_command(args: &[PathBuf]) -> ! {
    let [table, target] = args else {
        fail("usage: horizon-stage fits USES_JSON TARGET_SURFACE")
    };
    let bytes = std::fs::read(table).unwrap_or_else(|e| fail(format!("{}: {e}", table.display())));
    let table =
        UsesTable::read(&bytes).unwrap_or_else(|e| fail(format!("{}: {e}", table.display())));
    match fits(&table.entries, &read_surface(target)) {
        Fit::Fits => {
            println!(
                "fits ({} items, made for {})",
                table.entries.len(),
                table.made_for
            );
            std::process::exit(0)
        }
        Fit::DoesNotFit(misfits) => {
            println!(
                "does not fit: {} of the {} items it uses changed",
                misfits.len(),
                table.entries.len()
            );
            for misfit in &misfits {
                println!("  {misfit}  [{}]", misfit.id);
            }
            std::process::exit(1)
        }
    }
}

fn main() {
    let args: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    match args.first().and_then(|c| c.to_str()) {
        Some("uses") => return uses_command(&args[1..]),
        Some("fits") => fits_command(&args[1..]),
        _ => {}
    }
    if let [command, root, out] = &args[..]
        && command == "surface"
    {
        return surface(root, out);
    }
    let usage = || {
        eprintln!(
            "usage: horizon-stage ROOT POINTS_JSON INTERNALS_JSON API_VERSION [--sync-internals]\n       horizon-stage surface ROOT OUT\n       horizon-stage uses SURFACE MOD_DIR [--json OUT --made-for BUILD]\n       horizon-stage fits USES_JSON TARGET_SURFACE"
        );
        std::process::exit(2)
    };
    let [root, list, internals, api_version, options @ ..] = &args[..] else {
        usage()
    };
    // Discovery never builds or publishes a game.
    let mode = match options {
        [] => Mode::Build,
        [flag] if flag == "--sync-internals" => Mode::Discovery,
        _ => usage(),
    };
    let Some(api_version) = api_version.to_str() else {
        usage()
    };
    match staging::stage(root, list, internals, mode, api_version) {
        Ok(staging::Staged {
            points: staged,
            replace,
            internals: widened,
        }) => {
            println!(
                "staged {} points ({} with a head only; {} functions have no API record) in {} files",
                staged.points, staged.head_only, staged.refused, staged.files
            );
            println!(
                "gave {} functions a replacement slot in {} files of {} crates",
                replace.functions, replace.files, replace.crates
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
