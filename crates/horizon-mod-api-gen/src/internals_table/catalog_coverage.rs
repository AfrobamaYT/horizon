//! Expensive binding coverage over the actual staged game, not a synthetic catalog.
use crate as horizon_mod_api_gen;
use horizon_mod_api_gen::{crossing::Rule, internals_table, internals_view, points, staging};
use std::{collections::BTreeMap, fs, path::Path, process::Command};

fn checked(command: &mut Command) -> std::process::Output {
    let output = command.output().expect("start coverage command");
    assert!(
        output.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
#[ignore = "stages and checks the universal host and wasm32 guest; main-push wasm-mods only"]
fn real_catalog_has_compiling_host_and_guest_bindings() {
    let started = std::time::Instant::now();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let build = root.join("target/internals-catalog-coverage");
    if build.exists() {
        fs::remove_dir_all(&build).unwrap();
    }
    let stage = build.join("stage");
    fs::create_dir_all(&stage).unwrap();
    let files = checked(
        Command::new("git")
            .current_dir(&root)
            .args(["ls-files", "-z"]),
    );
    for path in files.stdout.split(|b| *b == 0).filter(|p| !p.is_empty()) {
        let path = std::str::from_utf8(path).unwrap();
        let source = root.join(path);
        if !source.is_file() {
            continue;
        }
        let dest = stage.join(path);
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::copy(source, dest).unwrap();
    }
    let catalog = stage.join("internals.json");
    println!("staging real game catalog");
    let mut staged = staging::stage(
        &stage,
        &stage.join("points.json"),
        &catalog,
        staging::Mode::Build,
    )
    .unwrap();
    // Shape-only layouts are not mod declarations, but still require a working codec/node on
    // both sides. This coverage catalog exposes them as roots solely to compile every layout.
    for entry in &mut staged.internals.entries {
        entry.shape_only = false;
    }
    let entries = &staged.internals.entries;
    fs::write(&catalog, serde_json::to_vec(entries).unwrap()).unwrap();
    let all: Vec<_> = entries
        .iter()
        .filter(|i| i.cfg.is_empty())
        .map(|i| i.path.clone())
        .collect();
    let game_points = points::game_points(&stage).unwrap();
    println!("selecting universal bindings");
    let rule = Rule::new(entries);
    let hashes = entries
        .iter()
        .filter(|i| matches!(i.kind, "struct" | "enum") && i.cfg.is_empty())
        .map(|i| {
            (
                crate::crossing::bare(&i.path),
                (i.path.as_str(), i.hash.as_str()),
            )
        })
        .collect();
    let selected = internals_table::select(
        entries,
        all.iter().map(String::as_str).collect(),
        &game_points,
        &rule,
        &hashes,
    );
    let mut skipped = BTreeMap::<String, String>::new();
    let mut declared = Vec::new();
    for entry in entries {
        let bound = match entry.kind {
            "struct" | "enum" => selected
                .selection
                .full
                .contains(&crate::crossing::bare(&entry.path)),
            "field" => selected
                .members
                .iter()
                .any(|(i, _, _)| i.path == entry.path && i.hash == entry.hash),
            "fn" | "method" => selected
                .accepted
                .iter()
                .any(|i| i.path == entry.path && i.hash == entry.hash),
            _ => false,
        };
        let reason = if !entry.cfg.is_empty() {
            Some("conditional item")
        } else if bound {
            None
        } else {
            Some(
                selected
                    .counts
                    .unbound
                    .get(&(entry.path.clone(), entry.kind, entry.hash.clone()))
                    .copied()
                    .or_else(|| selected.counts.refused.get(&entry.path).copied())
                    .unwrap_or_else(|| {
                        panic!("unexplained missing binding: {} {}", entry.kind, entry.path)
                    }),
            )
        };
        if let Some(reason) = reason {
            skipped.insert(format!("{} ({})", entry.path, entry.kind), reason.into());
        } else {
            declared.push(internals_view::Declaration {
                path: entry.path.clone(),
                hash: entry.hash.clone(),
                kind: Some(entry.kind.into()),
            });
        }
    }
    assert!(!declared.is_empty(), "coverage must not be vacuous");
    let raw: Vec<_> = selected
        .raw
        .iter()
        .enumerate()
        .map(|(index, (entry, _))| internals_view::Injection {
            target: entry.path.clone(),
            at_return: false,
            listener: format!("raw_{index}"),
        })
        .collect();
    let paths: Vec<_> = declared
        .iter()
        .map(|i| i.path.clone())
        .chain(raw.iter().map(|i| i.target.clone()))
        .collect();
    let (host, bound) = internals_table::table_with_points(entries, &paths, &game_points);
    assert!(
        bound.refused.is_empty(),
        "accepted entries lost host bindings: {:?}",
        bound.refused
    );
    // Generate now, too: an entry silently lost from selection must fail before compilation.
    println!(
        "generating universal guest view: {} declarations, {} raw records",
        declared.len(),
        raw.len()
    );
    internals_view::generate(entries, &declared, &raw, &game_points).unwrap();
    fs::write(
        stage.join("crates/horizon-client/src/__internals.rs"),
        crate::rustfmt(&host).unwrap(),
    )
    .unwrap();
    let lib = stage.join("crates/horizon-client/src/lib.rs");
    fs::write(
        &lib,
        fs::read_to_string(&lib)
            .unwrap()
            .replace("#![forbid(unsafe_code)]", "#![deny(unsafe_code)]"),
    )
    .unwrap();
    let mut manifest = String::from("[native]\n");
    for d in &declared {
        manifest.push_str(&format!(
            "[[native.internals]]\npath = {:?}\nhash = {:?}\nkind = {:?}\n",
            d.path,
            d.hash,
            d.kind.as_ref().unwrap()
        ));
    }
    for injection in &raw {
        manifest.push_str(&format!(
            "[[native.injection]]\ntarget = {:?}\nkind = \"HEAD\"\nraw = {:?}\n",
            injection.target, injection.listener
        ));
    }
    let manifest_path = build.join("coverage.toml");
    fs::write(&manifest_path, manifest).unwrap();
    let report = format!(
        "## Internals binding coverage\n\n{} entries, {} host types, {} thunks.\n\n### Renamed methods\n\n{}\n\n### Explicit exclusions\n\n{}\n",
        declared.len(),
        bound.types,
        bound.thunks,
        bound
            .renamed_methods
            .iter()
            .map(|(p, n)| format!("- `{p}`: `{n}`"))
            .collect::<Vec<_>>()
            .join("\n"),
        skipped
            .iter()
            .map(|(p, r)| format!("- `{p}`: {r}"))
            .chain(bound.refused_map_keys.iter().map(|key| format!(
                "- map key `{key}`: native equality is not proven structural or cannot be decoded"
            )))
            .collect::<Vec<_>>()
            .join("\n")
    );
    fs::write(build.join("summary.md"), &report).unwrap();
    if let Some(summary) = std::env::var_os("GITHUB_STEP_SUMMARY") {
        use std::io::Write;
        fs::OpenOptions::new()
            .append(true)
            .open(summary)
            .unwrap()
            .write_all(report.as_bytes())
            .unwrap();
    }
    println!("{report}");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let target = root.join("target/internals-catalog-check");
    println!("checking staged host");
    checked(
        Command::new(&cargo)
            .current_dir(&stage)
            .env("CARGO_TARGET_DIR", &target)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .args([
                "check",
                "--message-format=short",
                "-j4",
                "-p",
                "horizon-client",
                "--lib",
                "--no-default-features",
            ]),
    );
    let wasm_cargo =
        std::env::var_os("HORIZON_COVERAGE_WASM_CARGO").unwrap_or_else(|| cargo.clone());
    let mut guest = Command::new(wasm_cargo);
    guest
        .current_dir(&stage)
        .env("CARGO_TARGET_DIR", target.join("wasm"))
        .env("HORIZON_MOD_INTERNALS_MANIFEST", &manifest_path)
        .env("HORIZON_MOD_INTERNALS_CATALOG", &catalog)
        .args([
            "check",
            "--message-format=short",
            "-j4",
            "-p",
            "horizon-mod-internals",
            "--target",
            "wasm32-unknown-unknown",
        ]);
    if let Some(rustc) = std::env::var_os("HORIZON_COVERAGE_WASM_RUSTC") {
        guest.env("RUSTC", rustc).env("RUSTC_WRAPPER", "");
    }
    println!("checking generated wasm32 guest");
    guest
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());
    checked(&mut guest);
    println!("real catalog coverage passed in {:?}", started.elapsed());
}
