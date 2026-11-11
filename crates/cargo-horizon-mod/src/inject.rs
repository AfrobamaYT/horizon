//! Tool-written native inject declarations, pinned to the game's metadata.
use horizon_mod_package::{
    Manifest,
    internals::ListedInternal,
    manifest::{
        MANIFEST,
        inject::{InjectError, Kind},
    },
};
use std::{fs, path::Path};
use toml_edit::{DocumentMut, Table, value};

const USAGE: &str = "usage: cargo horizon-mod inject add <at> --kind <kind> [--with <fn>]";

#[derive(serde::Deserialize)]
struct Catalog {
    #[serde(default)]
    points: Vec<Point>,
    /// The `mod_api` types a mod point's `api` record may hold.
    #[serde(default)]
    wire: std::collections::BTreeSet<String>,
}
/// A function's point, as `points.json` lists it: whether its API listeners have a record.
#[derive(serde::Deserialize)]
struct Point {
    target: String,
    #[serde(default)]
    api_refusal: Option<String>,
}
struct Request<'a> {
    at: &'a str,
    kind: Kind,
    kind_name: &'a str,
    with: &'a str,
}

fn request(args: &[String]) -> Result<Request<'_>, String> {
    let [command, at, rest @ ..] = args else {
        return Err(USAGE.into());
    };
    if command != "add" || at.starts_with('-') {
        return Err(USAGE.into());
    }
    let (mut kind, mut with) = (None, None);
    for pair in rest.chunks(2) {
        let [flag, val] = pair else {
            return Err(USAGE.into());
        };
        if val.starts_with('-') {
            return Err(USAGE.into());
        }
        match flag.as_str() {
            "--kind" if kind.is_none() => kind = Some(val.as_str()),
            "--with" if with.is_none() => with = Some(val.as_str()),
            _ => return Err(USAGE.into()),
        }
    }
    let kind_name = kind.ok_or(USAGE)?;
    let kind = serde_json::from_value(serde_json::Value::String(kind_name.into()))
        .map_err(|e| format!("invalid inject kind: {e}"))?;
    Ok(Request {
        at,
        kind,
        kind_name,
        with: with.unwrap_or("on_inject"),
    })
}

#[path = "inject_update.rs"]
pub(crate) mod update;

pub fn run(args: &[String]) -> Result<(), String> {
    if args.first().is_some_and(|command| command == "update") {
        return update::run(Path::new("."), &args[1..]);
    }
    if args.first().is_some_and(|command| command == "sync") {
        return crate::inject_sync::run(&args[1..]);
    }
    add(Path::new("."), args)
}

fn add(dir: &Path, args: &[String]) -> Result<(), String> {
    let request = request(args)?;
    let path = dir.join(MANIFEST);
    let original = crate::bounded::read(&path, crate::bounded::MAX_MANIFEST)?;
    // Manifest::parse owns mod id, native pin and existing declaration validation.
    let manifest = Manifest::parse(&original)?;
    let directory = crate::internals::catalog_directory(dir, &manifest)?;
    let native = manifest
        .native
        .as_ref()
        .ok_or("inject add needs a [native] mod")?;
    let internals = if native.module.is_empty() {
        crate::internals::load(dir, &native.horizon)?
    } else {
        horizon_mod_package::internals::read(&directory.join("internals.json"))?
    };
    let points = directory.join("points.json");
    let catalog: Catalog =
        serde_json::from_str(&crate::bounded::read(&points, crate::bounded::MAX_CATALOG)?)
            .map_err(|e| format!("{}: {e}", points.display()))?;
    if let Some(library) = library(dir, request.at, &catalog.wire)? {
        let candidate = append_mod_point(&original, &manifest, &request, &library)?;
        crate::internals::atomic_write_manifest(&path, candidate)?;
        println!("{}", mod_availability(&library, &request));
        return Ok(());
    }
    let candidate = append(&original, &manifest, &request, &internals, &catalog)?;
    crate::internals::atomic_write_manifest(&path, candidate)?;
    println!("{}", availability(&catalog, &request));
    Ok(())
}

/// Which access tiers can listen at the request's place: `access = "api"` (native and wasm) at a
/// point whose every parameter has an API type, at its head or return; `access = "internals"`
/// on native mods only.
fn availability(catalog: &Catalog, request: &Request<'_>) -> String {
    use horizon_mod_point_emit::plain_path;
    let api = if !matches!(request.kind, Kind::Head | Kind::Return | Kind::Replace) {
        format!("refused, {} is internals only", request.kind_name)
    } else {
        match catalog
            .points
            .iter()
            .find(|point| plain_path(&point.target) == plain_path(request.at))
        {
            Some(Point {
                api_refusal: None, ..
            }) => "available (native and wasm)".to_owned(),
            Some(Point {
                api_refusal: Some(why),
                ..
            }) => format!("refused, {why}"),
            None => format!("refused, {} is not a point", request.at),
        }
    };
    format!("api: {api}")
}

/// The points of the dependency whose crate owns `at` (#2339), listed from its source as its
/// build lists them; none when `at` is not another mod's function.
fn library(
    dir: &Path,
    at: &str,
    wire: &std::collections::BTreeSet<String>,
) -> Result<Option<ModPoints>, String> {
    let krate = at.split("::").next().unwrap_or_default();
    // Metadata needs no particular toolchain: the cargo running this subcommand lists it.
    let mut command =
        std::process::Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command
        .args(["metadata", "--format-version", "1"])
        .current_dir(dir);
    let metadata: serde_json::Value =
        serde_json::from_str(&crate::wasm::output(command)?).map_err(|e| e.to_string())?;
    let owner = metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|package| {
            package["targets"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|target| {
                    target["kind"]
                        .as_array()
                        .is_some_and(|kinds| kinds.iter().any(|kind| kind == "lib"))
                        && target["name"].as_str().map(|name| name.replace('-', "_"))
                            == Some(krate.to_owned())
                })
        });
    let Some(manifest) = owner.and_then(|package| package["manifest_path"].as_str()) else {
        return Ok(None);
    };
    let root = Path::new(manifest)
        .parent()
        .ok_or("a manifest has a folder")?;
    if !root.join(MANIFEST).is_file() {
        return Ok(None);
    }
    horizon_mod_api_gen::points::mod_crate::mod_points(root, wire).map(Some)
}

type ModPoints = point_shape::catalog::Catalog;

/// The `[[inject]]` for a mod point: its head or return, pinned to the function's hash.
fn append_mod_point(
    text: &str,
    manifest: &Manifest,
    request: &Request<'_>,
    library: &ModPoints,
) -> Result<String, String> {
    use horizon_mod_package::manifest::inject::mod_points::Refusal;
    let id = format!("{}:{}", manifest.id, request.with);
    let Some(point) = library
        .points
        .iter()
        .find(|point| point.target == request.at)
    else {
        return Err(if library.opted_out.iter().any(|path| path == request.at) {
            Refusal::OptedOut {
                id,
                at: request.at.into(),
            }
            .to_string()
        } else {
            InjectError::Missing {
                id,
                at: request.at.into(),
            }
            .to_string()
        });
    };
    let mut doc: DocumentMut = text.parse().map_err(|e| format!("{MANIFEST}: {e}"))?;
    if doc.get("inject").is_none() {
        doc["inject"] = toml_edit::Item::ArrayOfTables(toml_edit::ArrayOfTables::new());
    }
    let mut table = Table::new();
    table["id"] = value(request.with);
    table["at"] = value(&point.target);
    table["kind"] = value(request.kind_name);
    table["with"] = value(request.with);
    table["hash"] = value(&point.hash);
    doc["inject"]
        .as_array_of_tables_mut()
        .ok_or("inject add requires [[inject]] tables")?
        .push(table);
    let result = doc.to_string();
    Manifest::parse(&result)?;
    Ok(result)
}

/// Which access tiers can listen at a mod point: `api` when every parameter has a `Wire` form,
/// `internals` from the library's own target.
fn mod_availability(library: &ModPoints, request: &Request<'_>) -> String {
    let api = match library
        .points
        .iter()
        .find(|point| point.target == request.at)
        .and_then(|point| point.api_refusal.as_ref())
    {
        None => "available".to_owned(),
        Some(why) => format!("refused, {why}"),
    };
    format!("api: {api}")
}

fn append(
    text: &str,
    manifest: &Manifest,
    request: &Request<'_>,
    internals: &[ListedInternal],
    catalog: &Catalog,
) -> Result<String, String> {
    let missing = || {
        InjectError::NoSlot {
            id: manifest.id.clone(),
            reason: horizon_mod_package::manifest::inject::NoSlotReason::NoApiRecord,
            declaration: request.at.into(),
        }
        .to_string()
    };
    let item = internals
        .iter()
        .find(|i| {
            i.path == request.at && !i.shape_only && matches!(i.kind.as_str(), "fn" | "method")
        })
        .ok_or_else(missing)?;
    let item = horizon_mod_package::internals::resolve(
        internals,
        &manifest.id,
        request.at,
        Some(&item.kind),
    )
    .map_err(|e| e.to_string())?;
    if !catalog
        .points
        .iter()
        .any(|point| point.target == request.at)
    {
        return Err(missing());
    }
    let mut doc: DocumentMut = text.parse().map_err(|e| format!("{MANIFEST}: {e}"))?;
    if doc.get("inject").is_none() {
        doc["inject"] = toml_edit::Item::ArrayOfTables(toml_edit::ArrayOfTables::new());
    }
    let array = doc["inject"]
        .as_array_of_tables_mut()
        .ok_or("inject add requires [[inject]] tables")?;
    let mut table = Table::new();
    table["id"] = value(request.with);
    table["at"] = value(&item.path);
    table["kind"] = value(request.kind_name);
    table["with"] = value(request.with);
    table["hash"] = value(&item.hash);
    array.push(table);
    let result = doc.to_string();
    Manifest::parse(&result)?;
    Ok(result)
}
