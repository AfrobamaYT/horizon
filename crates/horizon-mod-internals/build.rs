use horizon_mod_api_gen::internals_view::{self, Declaration, Injection};
use std::{env, fs, path::PathBuf};

fn generate() -> Result<String, String> {
    const MANIFEST: &str = "HORIZON_MOD_INTERNALS_MANIFEST";
    const CATALOG: &str = "HORIZON_MOD_INTERNALS_CATALOG";
    println!("cargo:rerun-if-env-changed={MANIFEST}");
    println!("cargo:rerun-if-env-changed={CATALOG}");
    let (manifest, catalog) = match (env::var_os(MANIFEST), env::var_os(CATALOG)) {
        (None, None) => return Ok("pub mod view {}".into()),
        (Some(m), Some(c)) => (PathBuf::from(m), PathBuf::from(c)),
        _ => return Err(format!("{MANIFEST} and {CATALOG} must both be set")),
    };
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rerun-if-changed={}", catalog.display());
    #[derive(serde::Deserialize, Default)]
    struct Native {
        #[serde(default)]
        internals: Vec<Internal>,
        #[serde(default)]
        injection: Vec<Raw>,
    }
    #[derive(serde::Deserialize)]
    struct Internal {
        path: String,
        hash: String,
        kind: Option<String>,
    }
    #[derive(serde::Deserialize)]
    struct Raw {
        target: Option<String>,
        kind: String,
        raw: Option<String>,
    }
    #[derive(serde::Deserialize)]
    struct Manifest {
        #[serde(default)]
        native: Native,
    }
    let text = fs::read_to_string(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
    let manifest: Manifest = toml::from_str(&text).map_err(|e| e.to_string())?;
    let catalog =
        fs::read_to_string(&catalog).map_err(|e| format!("{}: {e}", catalog.display()))?;
    let entries = internals_view::catalog(&catalog)?;
    let declared = manifest
        .native
        .internals
        .into_iter()
        .map(|i| Declaration {
            path: i.path,
            hash: i.hash,
            kind: i.kind,
        })
        .collect::<Vec<_>>();
    let injections = manifest
        .native
        .injection
        .into_iter()
        .filter_map(|i| i.raw.map(|raw| (i.target, i.kind, raw)))
        .map(|(target, kind, raw)| {
            Ok(Injection {
                target: target.ok_or("a raw injection needs a target")?,
                at_return: match kind.as_str() {
                    "HEAD" => false,
                    "RETURN" => true,
                    _ => return Err("a raw injection needs HEAD or RETURN".to_owned()),
                },
                listener: raw,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let points = horizon_mod_api_gen::points::game_points(&root)?;
    // The shared selector may read a function body or point declaration as well as the catalog.
    for path in horizon_mod_api_gen::points::tracked(&root)? {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    internals_view::generate(&entries, &declared, &injections, &points)
}

fn main() {
    let code = generate().expect("generating the mod's declared internals view");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("view.rs"),
        code,
    )
    .unwrap();
}
