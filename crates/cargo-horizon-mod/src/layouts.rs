//! Cached host-side layout probe for packaged mod types.
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_root: PathBuf,
    resolve: Resolve,
}
#[derive(Deserialize)]
struct Resolve {
    nodes: Vec<Node>,
}
#[derive(Deserialize)]
struct Node {
    id: String,
    dependencies: Vec<String>,
}
#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    manifest_path: PathBuf,
    source: Option<String>,
    version: String,
    targets: Vec<Target>,
}
#[derive(Deserialize)]
struct Target {
    name: String,
    kind: Vec<String>,
    src_path: PathBuf,
}

fn output(command: &mut Command) -> Result<Vec<u8>, String> {
    let result = command.output().map_err(|e| e.to_string())?;
    if !result.status.success() {
        return Err(String::from_utf8_lossy(&result.stderr).into_owned());
    }
    Ok(result.stdout)
}
fn save(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if fs::read(path).ok().as_deref() != Some(bytes) {
        fs::write(path, bytes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Computes layout identities using the mod's resolved dependencies on the host target.
pub fn build(root: &Path) -> Result<(), String> {
    if !root.join("Cargo.toml").is_file() {
        return Ok(());
    }
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let declaration = horizon_mod_package::Manifest::parse(
        &fs::read_to_string(root.join(horizon_mod_package::manifest::MANIFEST))
            .map_err(|e| e.to_string())?,
    )?;
    let cargo: toml::Value =
        toml::from_str(&fs::read_to_string(root.join("Cargo.toml")).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let lib = root.join(
        cargo
            .get("lib")
            .and_then(|v| v.get("path"))
            .and_then(toml::Value::as_str)
            .unwrap_or("src/lib.rs"),
    );
    if declaration.dependencies.is_empty()
        && lib.is_file()
        && horizon_mod_api_gen::wire_types::public_types("probe", &lib)?.is_empty()
    {
        let destination = root.join("layouts.json");
        if destination.exists() {
            fs::remove_file(destination).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    let metadata: Metadata = serde_json::from_slice(&output(
        Command::new("cargo")
            .args(["metadata", "--format-version", "1"])
            .current_dir(&root),
    )?)
    .map_err(|e| e.to_string())?;
    let owner = metadata
        .packages
        .iter()
        .find(|p| p.manifest_path == root.join("Cargo.toml"))
        .ok_or("mod is not a Cargo package")?;
    let mut needed = std::collections::BTreeSet::new();
    let mut pending = vec![owner.id.clone()];
    while let Some(id) = pending.pop() {
        if needed.insert(id.clone())
            && let Some(node) = metadata.resolve.nodes.iter().find(|n| n.id == id)
        {
            pending.extend(node.dependencies.iter().cloned());
        }
    }
    let mut declarations = Vec::new();
    for package in metadata.packages.iter().filter(|p| needed.contains(&p.id)) {
        let directory = package
            .manifest_path
            .parent()
            .ok_or("package manifest has no directory")?;
        let Ok(text) = fs::read_to_string(directory.join(horizon_mod_package::manifest::MANIFEST))
        else {
            continue;
        };
        let manifest = horizon_mod_package::Manifest::parse(&text)?;
        let Some(target) = package.targets.iter().find(|t| {
            t.kind
                .iter()
                .any(|k| matches!(k.as_str(), "lib" | "rlib" | "dylib" | "cdylib"))
        }) else {
            continue;
        };
        let types = horizon_mod_api_gen::wire_types::public_types(
            &target.name.replace('-', "_"),
            &target.src_path,
        )?;
        if !types.is_empty() {
            declarations.push((package, manifest.id, types));
        }
    }
    let destination = root.join("layouts.json");
    if declarations.is_empty() {
        if destination.exists() {
            fs::remove_file(destination).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    let api = metadata
        .packages
        .iter()
        .find(|p| p.name == "horizon-mod-api")
        .ok_or("crossing types need horizon-mod-api")?;
    let stage = root.join("target/horizon-mod/layout-probe");
    fs::create_dir_all(stage.join("src")).map_err(|e| e.to_string())?;
    let mut manifest = String::from(
        "[package]\nname = \"horizon-layout-probe\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n[dependencies]\n",
    );
    let mut api_dependency = toml::map::Map::new();
    api_dependency.insert("package".into(), api.name.clone().into());
    match api.source.as_deref() {
        Some(source) if source.starts_with("git+") => {
            let (url, rev) = source[4..]
                .rsplit_once('#')
                .ok_or("API git source has no revision")?;
            api_dependency.insert(
                "git".into(),
                url.split('?')
                    .next()
                    .ok_or("API git source has no URL")?
                    .into(),
            );
            api_dependency.insert("rev".into(), rev.into());
        }
        Some(_) => {
            api_dependency.insert("version".into(), format!("={}", api.version).into());
        }
        None => {
            api_dependency.insert(
                "path".into(),
                api.manifest_path
                    .parent()
                    .ok_or("API has no directory")?
                    .to_string_lossy()
                    .into_owned()
                    .into(),
            );
        }
    }
    manifest.push_str(&format!(
        "mod_api = {}\n",
        toml::Value::Table(api_dependency)
    ));
    manifest.push_str("serde_json = \"1\"\n");
    let mut source = String::from(
        "use mod_api::wire::Wire;\nfn main() { let mut libraries = std::collections::BTreeMap::new();\n",
    );
    for (index, (package, id, types)) in declarations.iter().enumerate() {
        let alias = format!("library_{index}");
        manifest.push_str(&format!(
            "{alias} = {{ package = {:?}, path = {:?} }}\n",
            package.name,
            package
                .manifest_path
                .parent()
                .ok_or("no mod directory")?
                .to_string_lossy()
        ));
        source.push_str("let mut types = std::collections::BTreeMap::new();\n");
        for ty in types {
            let (_, suffix) = ty.split_once("::").ok_or("type has no crate")?;
            source.push_str(&format!("types.insert({ty:?}, format!(\"{{:016x}}\", <{alias}::{suffix} as Wire>::layout(&mut Vec::new())));\n"));
        }
        source.push_str(&format!("libraries.insert({id:?}, types);\n"));
    }
    source.push_str("println!(\"{}\", serde_json::to_string(&libraries).unwrap()); }\n");
    save(&stage.join("Cargo.toml"), manifest.as_bytes())?;
    save(&stage.join("src/main.rs"), source.as_bytes())?;
    let lock = metadata.workspace_root.join("Cargo.lock");
    if lock.is_file() {
        save(
            &stage.join("Cargo.lock"),
            &fs::read(lock).map_err(|e| e.to_string())?,
        )?;
    }
    let rustc = output(Command::new("rustc").arg("-vV"))?;
    let rustc = String::from_utf8(rustc).map_err(|e| e.to_string())?;
    let host = rustc
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .ok_or("rustc did not report its host target")?;
    let bytes = output(
        Command::new("cargo")
            .args(["run", "--quiet", "--target", host, "--manifest-path"])
            .arg(stage.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(root.join("target/horizon-mod/probe-target"))
            .env("CARGO_INCREMENTAL", "0"),
    )?;
    let json: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("layout probe: {e}"))?;
    save(
        &destination,
        &serde_json::to_vec(&json).map_err(|e| e.to_string())?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn field_changes_invalidate_the_cached_probe() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../target/layout-probe-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        let api = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../horizon-mod-api")
            .canonicalize()
            .unwrap();
        fs::write(root.join("Cargo.toml"), format!("[package]\nname = \"layout-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[workspace]\n[dependencies]\nhorizon-mod-api = {{ path = {:?} }}\n", api.to_string_lossy())).unwrap();
        fs::write(root.join("horizon-mod.toml"), "id = \"fixture\"\nname = \"Fixture\"\nversion = \"0.1.0\"\napi = \"0.2\"\nside = \"both\"\ncapabilities = [\"registries\"]\n").unwrap();
        fs::write(root.join("src/lib.rs"), "#[derive(mod_api::Wire)] pub struct Record { pub field: u32 }\nmod private { #[derive(mod_api::Wire)] pub struct Secret; }\n").unwrap();
        let start = std::time::Instant::now();
        build(&root).unwrap();
        eprintln!("layout probe cold: {:?}", start.elapsed());
        let before = fs::read(root.join("layouts.json")).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&before).unwrap();
        assert_eq!(value["fixture"].as_object().unwrap().len(), 1);
        let start = std::time::Instant::now();
        build(&root).unwrap();
        eprintln!("layout probe warm: {:?}", start.elapsed());
        assert_eq!(before, fs::read(root.join("layouts.json")).unwrap());
        fs::write(
            root.join("src/lib.rs"),
            "#[derive(mod_api::Wire)] pub struct Record { pub field: u64 }\n",
        )
        .unwrap();
        build(&root).unwrap();
        assert_ne!(before, fs::read(root.join("layouts.json")).unwrap());
        fs::write(root.join("src/lib.rs"), "pub struct Local;\n").unwrap();
        build(&root).unwrap();
        assert!(!root.join("layouts.json").exists());
        let library = root.join("library");
        fs::create_dir_all(library.join("src")).unwrap();
        fs::write(library.join("Cargo.toml"), format!("[package]\nname = \"library\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[dependencies]\nhorizon-mod-api = {{ path = {:?} }}\n", api.to_string_lossy())).unwrap();
        fs::write(library.join("horizon-mod.toml"), "id = \"library\"\nname = \"Library\"\nversion = \"0.1.0\"\napi = \"0.2\"\nside = \"both\"\ncapabilities = [\"registries\"]\n").unwrap();
        fs::write(
            library.join("src/lib.rs"),
            "#[derive(mod_api::Wire)] pub struct Shared { pub value: u16 }\n",
        )
        .unwrap();
        let mut cargo = fs::read_to_string(root.join("Cargo.toml")).unwrap();
        cargo.push_str("library = { path = \"library\" }\n");
        fs::write(root.join("Cargo.toml"), cargo).unwrap();
        let mut manifest = fs::read_to_string(root.join("horizon-mod.toml")).unwrap();
        manifest.push_str("[dependencies]\nlibrary = \"^0.1\"\n");
        fs::write(root.join("horizon-mod.toml"), manifest).unwrap();
        build(&root).unwrap();
        let recorded: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("layouts.json")).unwrap()).unwrap();
        assert!(recorded["library"]["library::Shared"].as_str().is_some());
        assert!(recorded.get("fixture").is_none());
    }
}
