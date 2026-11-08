//! Cargo owns the group's crate graph. Each mod unit emits both its exact rlib and PIC member.
use super::*;
use std::collections::BTreeMap;

const FLAGS: &str = "-Crelocation-model=pic -Zshare-generics=n -Zdefault-visibility=hidden";

struct Member {
    root: PathBuf,
    stage: PathBuf,
    manifest: toml_edit::DocumentMut,
    krate: String,
    package: String,
}

fn copy(source: &Path, destination: &Path) -> Result<(), String> {
    std::fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        if ["target", ".git", "mod.wasm"].iter().any(|n| name == *n) {
            continue;
        }
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        if ty.is_dir() {
            copy(&entry.path(), &destination.join(name))?;
        } else if ty.is_file() {
            std::fs::copy(entry.path(), destination.join(name)).map_err(|e| e.to_string())?;
        } else {
            return Err(format!(
                "{}: cannot stage a symlink",
                entry.path().display()
            ));
        }
    }
    Ok(())
}

// Preserve all Cargo dependency attributes and aliases; only relocate local paths.
fn paths(
    table: &mut dyn toml_edit::TableLike,
    root: &Path,
    staged: &BTreeMap<PathBuf, PathBuf>,
) -> Result<(), String> {
    for (_, item) in table.iter_mut() {
        if let Some(child) = item.as_table_like_mut() {
            if let Some(path) = child.get("path").and_then(|i| i.as_str()) {
                let original = root
                    .join(path)
                    .canonicalize()
                    .map_err(|e| format!("{}: {e}", root.join(path).display()))?;
                let new = staged.get(&original).unwrap_or(&original);
                child.insert("path", toml_edit::value(new.to_string_lossy().as_ref()));
            }
            paths(child, root, staged)?;
        }
    }
    Ok(())
}

pub(super) fn build(root: &Path, catalog: Option<&Path>) -> Result<(), String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let toolchain = toolchain()?;
    let target = root.join("target/horizon-mod");
    let stage = target.join("group");
    let metadata: Value = serde_json::from_str(&output({
        let mut c = pinned("cargo");
        c.args(["metadata", "--format-version", "1"])
            .current_dir(&root);
        c
    })?)
    .map_err(|e| e.to_string())?;
    let mut members = Vec::new();
    for package in metadata["packages"]
        .as_array()
        .ok_or("Cargo metadata has no packages")?
    {
        let source = Path::new(
            package["manifest_path"]
                .as_str()
                .ok_or("no manifest path")?,
        )
        .parent()
        .ok_or("no package directory")?;
        let Ok(text) = std::fs::read_to_string(source.join(MANIFEST)) else {
            continue;
        };
        let manifest: toml_edit::DocumentMut =
            text.parse().map_err(|e| format!("{MANIFEST}: {e}"))?;
        if manifest.contains_key("native") {
            return Err(format!("{}: native mod in wasm group", source.display()));
        }
        let id = manifest["id"].as_str().ok_or("mod has no id")?;
        members.push(Member {
            root: source.canonicalize().map_err(|e| e.to_string())?,
            stage: stage.join(id),
            krate: lib_name(package).ok_or("mod has no library")?,
            package: package["name"].as_str().ok_or("no package name")?.into(),
            manifest,
        });
    }
    let staged: BTreeMap<_, _> = members
        .iter()
        .map(|m| (m.root.clone(), m.stage.clone()))
        .collect();
    let crates: Vec<_> = members.iter().map(|m| m.krate.clone()).collect();
    let mut configs = serde_json::Map::new();
    let mut features = Vec::new();
    for member in &members {
        copy(&member.root, &member.stage)?;
        let path = member.stage.join("Cargo.toml");
        let mut cargo: toml_edit::DocumentMut = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())?
            .parse()
            .map_err(|e| format!("Cargo.toml: {e}"))?;
        // Expand workspace inheritance before moving this package into the group workspace.
        let workspace = Path::new(
            metadata["workspace_root"]
                .as_str()
                .ok_or("no workspace root")?,
        );
        let ws: toml_edit::DocumentMut = std::fs::read_to_string(workspace.join("Cargo.toml"))
            .map_err(|e| e.to_string())?
            .parse()
            .map_err(|e| format!("workspace: {e}"))?;
        if let Some(package) = cargo.get_mut("package").and_then(|v| v.as_table_like_mut()) {
            for (key, value) in package.iter_mut() {
                if value
                    .as_table_like()
                    .and_then(|v| v.get("workspace"))
                    .and_then(|v| v.as_bool())
                    == Some(true)
                {
                    *value = ws["workspace"]["package"]
                        .get(key.get())
                        .ok_or(format!("workspace.package.{key} missing"))?
                        .clone();
                }
            }
            package.remove("workspace");
        }
        for section in ["dependencies", "build-dependencies", "dev-dependencies"] {
            if let Some(deps) = cargo.get_mut(section).and_then(|v| v.as_table_like_mut()) {
                for (key, value) in deps.iter_mut() {
                    if value
                        .as_table_like()
                        .and_then(|v| v.get("workspace"))
                        .and_then(|v| v.as_bool())
                        == Some(true)
                    {
                        let mut inherited = ws["workspace"]["dependencies"]
                            .get(key.get())
                            .ok_or(format!("workspace dependency {key} missing"))?
                            .clone();
                        if let Some(dep) = inherited.as_table_like_mut() {
                            if let Some(path) = dep.get("path").and_then(|v| v.as_str()) {
                                dep.insert(
                                    "path",
                                    toml_edit::value(
                                        workspace.join(path).to_string_lossy().as_ref(),
                                    ),
                                );
                            }
                            for (k, v) in
                                value.as_table_like().ok_or("dependency not table")?.iter()
                            {
                                if k != "workspace" {
                                    dep.insert(k, v.clone());
                                }
                            }
                        }
                        *value = inherited;
                    }
                }
            }
        }
        cargo.remove("workspace");
        paths(cargo.as_table_mut(), &member.root, &staged)?;
        cargo["lib"]["crate-type"] =
            toml_edit::value(["rlib", "cdylib"].into_iter().collect::<toml_edit::Array>());
        std::fs::write(path, cargo.to_string()).map_err(|e| e.to_string())?;
        let (registrations, points) = registrations(&member.manifest, catalog)?;
        configs.insert(member.package.clone(), serde_json::json!({"registrations":registrations,"points":points,"provided":crates.iter().filter(|c| **c != member.krate).map(String::as_str).collect::<Vec<_>>().join(",")}));
        if let Some(deps) = member
            .manifest
            .get("dependencies")
            .and_then(|v| v.as_table_like())
        {
            for (id, dep) in deps.iter() {
                if dep
                    .as_table_like()
                    .and_then(|d| d.get("optional"))
                    .and_then(|v| v.as_bool())
                    == Some(true)
                {
                    features.push(format!("{}/{id}", member.package));
                }
            }
        }
    }
    let lock = Path::new(
        metadata["workspace_root"]
            .as_str()
            .ok_or("no workspace root")?,
    )
    .join("Cargo.lock");
    if lock.is_file() {
        std::fs::copy(lock, stage.join("Cargo.lock")).map_err(|e| e.to_string())?;
    }
    let mut workspace = toml_edit::DocumentMut::new();
    workspace["workspace"]["members"] = toml_edit::value(
        members
            .iter()
            .map(|m| m.stage.to_string_lossy().into_owned())
            .collect::<toml_edit::Array>(),
    );
    workspace["workspace"]["resolver"] = toml_edit::value("3");
    std::fs::write(stage.join("Cargo.toml"), workspace.to_string()).map_err(|e| e.to_string())?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut command = pinned("cargo");
    command
        .args([
            "build",
            "--workspace",
            "--lib",
            "--target",
            TARGET,
            "--profile",
            PROFILE,
            "-Zbuild-std=std,panic_abort",
        ])
        .args([
            "--config",
            &format!("profile.{PROFILE}.inherits=\"release\""),
        ])
        .args(["--config", &format!("profile.{PROFILE}.panic=\"abort\"")])
        .args(["--config", &format!("profile.{PROFILE}.codegen-units=1")])
        .current_dir(&stage)
        .env("CARGO_TARGET_DIR", &target)
        .env("RUSTC_BOOTSTRAP", "1")
        .env("CARGO_INCREMENTAL", "0")
        .env("RUSTFLAGS", FLAGS)
        .env("RUSTC_WRAPPER", &exe)
        .env(
            "HORIZON_GROUP_WRAPPER",
            serde_json::to_string(&configs).map_err(|e| e.to_string())?,
        )
        .env("CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER", &exe)
        .env(LINK_ENV, "member")
        .env(LLD_ENV, &toolchain.lld);
    if !features.is_empty() {
        command.args(["--features", &features.join(",")]);
    }
    if !command.status().map_err(|e| e.to_string())?.success() {
        return Err("Cargo group build failed".into());
    }
    let artifacts = target.join(TARGET).join(PROFILE);
    let rlibs: BTreeMap<_, _> = members
        .iter()
        .map(|m| {
            let bytes = std::fs::read(artifacts.join(format!("lib{}.rlib", m.krate)))
                .map_err(|e| e.to_string())?;
            Ok((
                m.manifest["id"].as_str().ok_or("no mod id")?.to_owned(),
                sha512_of(&bytes),
            ))
        })
        .collect::<Result<_, String>>()?;
    let mut hashes = BTreeMap::new();
    for member in &members {
        let artifacts = target.join(TARGET).join(PROFILE);
        let mut module = std::fs::read(artifacts.join(format!("{}.wasm", member.krate)))
            .map_err(|e| e.to_string())?;
        let rlib = std::fs::read(artifacts.join(format!("lib{}.rlib", member.krate)))
            .map_err(|e| e.to_string())?;
        // Included in the module hash checked by the production loader's [wasm.linked].
        let identity = serde_json::to_vec(&serde_json::json!({"compiler":toolchain.version,"target":TARGET,"flags":FLAGS,"rlib":sha512_of(&rlib), "group_rlibs": rlibs, "profile":PROFILE, "codegen_units":1, "panic":"abort"})).map_err(|e| e.to_string())?;
        let mut section = Vec::new();
        wasm_encoder::Encode::encode(
            &wasm_encoder::CustomSection {
                name: "horizon.build".into(),
                data: identity.into(),
            },
            &mut section,
        );
        module.push(0);
        module.extend(section);
        hashes.insert(
            member.manifest["id"]
                .as_str()
                .ok_or("no mod id")?
                .to_owned(),
            sha512_of(&module),
        );
        std::fs::write(member.root.join(WASM), module).map_err(|e| e.to_string())?;
    }
    for member in &mut members {
        let id = member.manifest["id"]
            .as_str()
            .ok_or("no mod id")?
            .to_owned();
        let dependencies: Vec<String> = member
            .manifest
            .get("dependencies")
            .and_then(|v| v.as_table_like())
            .map(|d| d.iter().map(|(id, _)| id.to_owned()).collect())
            .unwrap_or_default();
        let mut wasm = toml_edit::Table::new();
        wasm["sha512"] = toml_edit::value(&hashes[&id]);
        wasm["toolchain"] = toml_edit::value(&toolchain.version);
        for dep in dependencies {
            let hash = hashes.get(&dep).ok_or(format!(
                "{id}: declared mod dependency {dep} is not in Cargo's group"
            ))?;
            wasm["linked"][&dep] = toml_edit::value(hash);
        }
        member.manifest["wasm"] = toml_edit::Item::Table(wasm);
        std::fs::write(member.root.join(MANIFEST), member.manifest.to_string())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Cargo still resolves and supplies every extern. This wrapper only scopes guest generation
/// and linker ownership to the package whose rustc unit is executing.
pub fn rustc(mut args: Vec<String>) -> Result<(), String> {
    let compiler = args.remove(0);
    let configs: Value =
        serde_json::from_str(&std::env::var("HORIZON_GROUP_WRAPPER").map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let package = std::env::var("CARGO_PKG_NAME").unwrap_or_default();
    let config = &configs[&package];
    let mut command = Command::new(compiler);
    command
        .env_remove("HORIZON_GUEST_REGISTRATIONS")
        .env_remove("HORIZON_POINT_NAMES")
        .env_remove(PROVIDED_ENV);
    if config.is_object() && args.iter().any(|a| a == "cdylib") {
        command
            .env("HORIZON_GROUP_PACKAGE", &package)
            .env(
                "HORIZON_GUEST_REGISTRATIONS",
                config["registrations"].as_str().ok_or("no registrations")?,
            )
            .env(
                "HORIZON_POINT_NAMES",
                config["points"].as_str().ok_or("no points")?,
            )
            .env(
                PROVIDED_ENV,
                config["provided"].as_str().ok_or("no provided crates")?,
            );
        args.push("-Zdefault-visibility=interposable".into());
        args.push(format!(
            "--cfg=horizon_guest_registration=\"{}\"",
            sha512_of(config.to_string().as_bytes())
        ));
    }
    if command
        .args(args)
        .status()
        .map_err(|e| e.to_string())?
        .success()
    {
        Ok(())
    } else {
        Err("rustc failed".into())
    }
}
