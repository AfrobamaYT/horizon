//! Compiler-driven discovery uses an isolated, per-pin source and metadata cache.
use super::{api, dependencies, internals, rust_toolchain};
use crate::i18n::trf;
use horizon_mods::internals::{self as catalog, ListedInternal};
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Stdio,
};

#[derive(Debug)]
pub(crate) enum Error {
    SourceUnavailable { revision: String, reason: String },
    SourceInjection,
    Check(String),
}

impl From<String> for Error {
    fn from(reason: String) -> Self {
        Self::Check(reason)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceUnavailable { revision, reason } => f.write_str(&trf(
                "E_INTERNAL_SOURCE_UNAVAILABLE: internals sync needs the source for {revision}. It is not cached and could not be downloaded: {error}",
                &[("revision", revision), ("error", reason)],
            )),
            Self::SourceInjection => f.write_str(crate::i18n::tr("E_INTERNAL_SOURCE_INJECTION: internals sync checks library code and raw listeners, not source-spliced helpers or modules. Use cargo horizon-mod internals add for this mod.")),
            Self::Check(reason) => f.write_str(reason),
        }
    }
}
impl std::error::Error for Error {}

/// No normal build reads this directory or its target. In particular, neither the attributes
/// nor the warning-bearing metadata can reach the prebuilt or source build caches.
pub(crate) fn check(folder: &Path) -> Result<Vec<ListedInternal>, Error> {
    let _lease = crate::native_cache::Cache::acquire(&crate::backend::build_dir())
        .map_err(|error| Error::Check(super::cache_error(error)))?;
    super::clear_log()?;
    discover(folder)
}

fn discover(folder: &Path) -> Result<Vec<ListedInternal>, Error> {
    let bundle = super::bundle_in(folder)?.ok_or_else(|| {
        trf(
            "{path} is not a native mod project.",
            &[("path", &folder.display().to_string())],
        )
    })?;
    if bundle.needs_mixins() {
        return Err(Error::SourceInjection);
    }
    let projects = [api::Project::read(&bundle).map_err(|e| e.to_string())?];
    let declared = [dependencies::from_manifest(&bundle.manifest.dependencies)];
    let inputs = [dependencies::Mod {
        id: &bundle.manifest.id,
        version: &bundle.manifest.version,
        dependencies: &declared[0],
        has_api: true,
    }];
    let resolution = dependencies::resolve(&inputs).map_err(|e| e.to_string())?;
    let revision = &bundle
        .manifest
        .native
        .as_ref()
        .expect("native bundle")
        .horizon;
    let source = super::source(revision).map_err(|reason| Error::SourceUnavailable {
        revision: revision.clone(),
        reason,
    })?;
    let cache = super::root().join("internals-sync").join(revision);
    let game = cache.join("game");
    if !game.join(".discovery-ready").is_file() {
        let fresh = cache.join("game.new");
        if fresh.exists() {
            fs::remove_dir_all(&fresh).map_err(|e| e.to_string())?;
        }
        super::sync(&source, &fresh)?;
        horizon_mod_api_gen::staging::stage(
            &fresh,
            &fresh.join(super::inject::POINTS),
            &fresh.join(internals::INTERNALS),
            horizon_mod_api_gen::staging::Mode::Discovery,
        )?;
        fs::write(fresh.join(".discovery-ready"), "").map_err(|e| e.to_string())?;
        fs::rename(&fresh, &game).map_err(|e| e.to_string())?;
    }
    let stage = cache.join("stage");
    super::sync(&game, &stage)?;
    let mut listed = internals::Catalog::read(&stage)?;
    listed.discovery = true;
    api::stage(&stage, &projects, &declared, &resolution, &listed)?;
    let rust = rust_toolchain::install(|_| {})?;
    let mod_dir = stage.join("native-api").join(&bundle.manifest.id);
    let mut cmd = super::cargo(&rust, &stage, "check");
    cmd.args(["--message-format=json", "-p", &projects[0].package, "--lib"])
        .env("CARGO_TARGET_DIR", cache.join("target"))
        .env(
            "RUSTC_WRAPPER",
            std::env::current_exe().map_err(|e| e.to_string())?,
        )
        .env("HORIZON_INTERNALS_CHECK_MOD", &mod_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let mut used = BTreeSet::new();
    let mut errors = String::new();
    for line in BufReader::new(child.stdout.take().expect("piped")).lines() {
        let line = line.map_err(|e| e.to_string())?;
        if let Some(message) = parse(&line, &mod_dir, &stage) {
            match message {
                Message::Used(kind, path) => {
                    used.insert((kind, path));
                }
                Message::Failure(text) => errors.push_str(&text),
            }
        }
    }
    if !child.wait().map_err(|e| e.to_string())?.success() {
        return Err(trf(
            "Internals discovery failed; the manifest was not changed. {error}",
            &[("error", &errors)],
        )
        .into());
    }
    used.into_iter()
        .filter(|(kind, _)| kind != "mod")
        .map(|(kind, path)| {
            catalog::resolve(&listed.internals, &bundle.manifest.id, &path, Some(&kind))
                .cloned()
                .map_err(|error| Error::Check(error.to_string()))
        })
        .collect()
}

/// Entry point used only when Cargo launches this executable as the discovery rustc wrapper.
/// Force the lint for the selected mod, including macro expansions and local allow/deny
/// attributes, without enabling warnings in the game's own crates or other dependencies.
pub(crate) fn rustc_wrapper() -> Option<i32> {
    let selected = std::env::var_os("HORIZON_INTERNALS_CHECK_MOD")?;
    let mut args = std::env::args_os().skip(1);
    let compiler = args.next()?;
    if Some(&compiler) != std::env::var_os("RUSTC").as_ref() {
        return None;
    }
    let mut cmd = std::process::Command::new(compiler);
    cmd.args(args);
    if std::env::var_os("CARGO_MANIFEST_DIR").as_ref() == Some(&selected) {
        cmd.arg("--force-warn=deprecated");
    }
    Some(match cmd.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            eprintln!("rustc: {error}");
            1
        }
    })
}

#[derive(Deserialize)]
struct Diagnostic {
    level: String,
    message: String,
    code: Option<Code>,
    spans: Vec<Span>,
    rendered: Option<String>,
}
#[derive(Deserialize)]
struct Code {
    code: String,
}
#[derive(Deserialize)]
struct Span {
    file_name: PathBuf,
    #[serde(default)]
    expansion: Option<Box<Expansion>>,
}
#[derive(Deserialize)]
struct Expansion {
    span: Span,
}
#[derive(Deserialize)]
struct CargoMessage {
    reason: String,
    message: Option<Diagnostic>,
    manifest_path: Option<PathBuf>,
}

enum Message {
    Used(String, String),
    Failure(String),
}

// Rustc paths are workspace-relative (or absolute). Macro expansions can point at the
// dependency's macro body: follow the expansion back to the mod's call site, never a string prefix.
fn owned(span: &Span, mod_dir: &Path, stage: &Path) -> bool {
    let path = if span.file_name.is_absolute() {
        span.file_name.clone()
    } else {
        stage.join(&span.file_name)
    };
    path.starts_with(mod_dir)
        || span
            .expansion
            .as_ref()
            .is_some_and(|expansion| owned(&expansion.span, mod_dir, stage))
}

fn parse(line: &str, mod_dir: &Path, stage: &Path) -> Option<Message> {
    let event: CargoMessage = serde_json::from_str(line).ok()?;
    if event.reason != "compiler-message" {
        return None;
    }
    let diagnostic = event.message?;
    if diagnostic.level == "error" {
        return Some(Message::Failure(
            diagnostic.rendered.unwrap_or(diagnostic.message),
        ));
    }
    if event
        .manifest_path
        .as_deref()
        .is_none_or(|path| path.parent() != Some(mod_dir))
    {
        return None;
    }
    if diagnostic.code?.code != "deprecated"
        || !diagnostic
            .spans
            .iter()
            .any(|span| owned(span, mod_dir, stage))
    {
        return None;
    }
    let (_, note) = diagnostic.message.rsplit_once("horizon-internal:")?;
    let (kind, path) = note.split_once(':')?;
    Some(Message::Used(kind.into(), path.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn discovery_filters_diagnostics_by_crate_span_and_lint_and_keeps_macro_calls() {
        let stage = Path::new("/cache/stage");
        let own = stage.join("native-api/example");
        let mut event = json!({"reason":"compiler-message", "manifest_path":own.join("Cargo.toml"), "message": {
            "level":"warning", "message":"use of deprecated method: horizon-internal:method:horizon_client::game::Thing::method",
            "code":{"code":"deprecated"}, "spans":[{"file_name":"native-api/example/src/lib.rs"}], "rendered":null
        }});
        assert!(
            matches!(parse(&event.to_string(), &own, stage), Some(Message::Used(kind, path)) if kind == "method" && path.ends_with("Thing::method"))
        );
        event["message"]["spans"][0] = json!({"file_name":"crates/dependency/src/lib.rs", "expansion":{"span":{"file_name":"native-api/example/src/lib.rs"}}});
        assert!(matches!(
            parse(&event.to_string(), &own, stage),
            Some(Message::Used(..))
        ));
        event["manifest_path"] = json!(stage.join("crates/dependency/Cargo.toml"));
        assert!(parse(&event.to_string(), &own, stage).is_none());
        event["manifest_path"] = json!(own.join("Cargo.toml"));
        event["message"]["spans"][0] = json!({"file_name":"native-api/example_other/src/lib.rs"});
        assert!(parse(&event.to_string(), &own, stage).is_none());
        event["message"]["spans"][0] = json!({"file_name":own.join("src/lib.rs")});
        event["message"]["code"]["code"] = json!("unused");
        assert!(parse(&event.to_string(), &own, stage).is_none());
        event["message"]["level"] = json!("error");
        assert!(matches!(
            parse(&event.to_string(), &own, stage),
            Some(Message::Failure(..))
        ));
    }
}
