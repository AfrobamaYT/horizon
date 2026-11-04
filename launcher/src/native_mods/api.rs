//! The launcher's generated executable for trait mods.
use super::Bundle;
use std::fs;
use std::path::Path;

pub(super) fn read(path: &Path) -> Result<toml::Table, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    text.parse().map_err(|e| format!("{}: {e}", path.display()))
}

fn write(path: &Path, value: &toml::Table) -> Result<(), String> {
    let text = toml::to_string(value).map_err(|e| format!("{}: {e}", path.display()))?;
    fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

pub(super) const PACKAGE: &str = "horizon-native-game";
/// Where the generated executable's package is staged, in the workspace.
const GAME: &str = "crates/native-game";

fn path_dep(path: &str) -> toml::Table {
    toml::Table::from_iter([("path".into(), path.into())])
}

const MAIN_TEMPLATE: &str = "crates/horizon-client/src/native_main.rs.template";

fn main_rs(template: &str, mods: &[&horizon_mods::Manifest]) -> String {
    let entries = mods
        .iter()
        .enumerate()
        .map(|(i, m)| format!("        {:?} => mod_{i}::hooks,\n", m.id))
        .collect::<String>();
    let inventory = mods.iter().map(|m| format!(
        "    horizon_mods::runtime::BuiltMod {{ id: {:?}, version: {:?}, side: horizon_mods::Side::{:?} }},\n",
        m.id, m.version, m.side,
    )).collect::<String>();
    template
        .replace("/* MOD_ENTRIES */", &entries)
        .replace("/* BUILT_MODS */", &inventory)
}

/// Parsed once from the pipeline's immutable incoming copy, then reused for staging.
pub(super) struct Project<'a> {
    bundle: &'a Bundle,
    cargo: toml::Table,
    package: String,
    client_deps: Vec<ClientDependency>,
}

struct ClientDependency {
    target: Option<String>,
    key: String,
}

impl ClientDependency {
    fn value<'a>(&self, cargo: &'a toml::Table) -> &'a toml::Value {
        match &self.target {
            Some(target) => &cargo["target"][target.as_str()]["dependencies"][self.key.as_str()],
            None => &cargo["dependencies"][self.key.as_str()],
        }
    }

    fn value_mut<'a>(&self, cargo: &'a mut toml::Table) -> &'a mut toml::Value {
        match &self.target {
            Some(target) => {
                &mut cargo["target"][target.as_str()]["dependencies"][self.key.as_str()]
            }
            None => &mut cargo["dependencies"][self.key.as_str()],
        }
    }
}

#[derive(Debug)]
pub(super) struct ClientFeatureRequest {
    field: &'static str,
    value: toml::Value,
}

#[derive(Debug)]
#[non_exhaustive]
pub(super) enum ProjectError {
    Cargo {
        mod_id: String,
        reason: String,
    },
    PackageName {
        mod_id: String,
    },
    ClientDependency {
        mod_id: String,
    },
    ClientFeatures {
        mod_id: String,
        requested: Vec<ClientFeatureRequest>,
    },
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::i18n::trf;
        let message = match self {
            Self::Cargo { mod_id, reason } => {
                trf("{mod}: {error}", &[("mod", mod_id), ("error", reason)])
            }
            Self::PackageName { mod_id } => {
                trf("{mod}: Cargo.toml has no package name.", &[("mod", mod_id)])
            }
            Self::ClientDependency { mod_id } => trf(
                "{mod}: Cargo.toml has no horizon-client dependency.",
                &[("mod", mod_id)],
            ),
            Self::ClientFeatures { mod_id, requested } => {
                let fields = requested
                    .iter()
                    .map(|r| format!("{} = {}", r.field, r.value))
                    .collect::<Vec<_>>()
                    .join(", ");
                trf("{mod}: horizon-client requests {fields}. Server builds are headless; the launcher owns horizon-client features. Remove features and use default-features = false or omit it.",
                    &[("mod", mod_id), ("fields", &fields)])
            }
        };
        f.write_str(&message)
    }
}
impl std::error::Error for ProjectError {}

impl<'a> Project<'a> {
    pub(super) fn read(bundle: &'a Bundle, target: super::Target) -> Result<Self, ProjectError> {
        let cargo = read(&bundle.dir.join("Cargo.toml")).map_err(|reason| ProjectError::Cargo {
            mod_id: bundle.manifest.id.clone(),
            reason,
        })?;
        let package = cargo
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(toml::Value::as_str)
            .ok_or_else(|| ProjectError::PackageName {
                mod_id: bundle.manifest.id.clone(),
            })?
            .to_owned();
        let dependencies = cargo
            .get("dependencies")
            .and_then(toml::Value::as_table)
            .ok_or_else(|| ProjectError::ClientDependency {
                mod_id: bundle.manifest.id.clone(),
            })?;
        let is_client = |key: &str, value: &toml::Value| {
            key == "horizon-client"
                || value.get("package").and_then(toml::Value::as_str) == Some("horizon-client")
        };
        let mut client_deps: Vec<_> = dependencies
            .iter()
            .filter(|(key, value)| is_client(key, value))
            .map(|(key, _)| ClientDependency {
                target: None,
                key: key.clone(),
            })
            .collect();
        if client_deps.is_empty() {
            return Err(ProjectError::ClientDependency {
                mod_id: bundle.manifest.id.clone(),
            });
        }
        if target == super::Target::Server {
            if let Some(targets) = cargo.get("target").and_then(toml::Value::as_table) {
                for (target, config) in targets {
                    if let Some(dependencies) =
                        config.get("dependencies").and_then(toml::Value::as_table)
                    {
                        client_deps.extend(
                            dependencies
                                .iter()
                                .filter(|(key, value)| is_client(key, value))
                                .map(|(key, _)| ClientDependency {
                                    target: Some(target.clone()),
                                    key: key.clone(),
                                }),
                        );
                    }
                }
            }
        } else {
            client_deps.truncate(1);
        }
        if target == super::Target::Server {
            let mut requested = Vec::new();
            for client in &client_deps {
                let dependency = client.value(&cargo);
                if let Some(value) = dependency
                    .get("default-features")
                    .filter(|value| value.as_bool() != Some(false))
                {
                    requested.push(ClientFeatureRequest {
                        field: "default-features",
                        value: value.clone(),
                    });
                }
                if let Some(value) = dependency.get("features") {
                    requested.push(ClientFeatureRequest {
                        field: "features",
                        value: value.clone(),
                    });
                }
            }
            if let Some(features) = cargo.get("features").and_then(toml::Value::as_table) {
                for value in features.values() {
                    if value.as_array().is_some_and(|values| {
                        values.iter().any(|value| {
                            value
                                .as_str()
                                .and_then(|value| value.split_once('/'))
                                .is_some_and(|(name, _)| {
                                    client_deps
                                        .iter()
                                        .any(|client| client.key == name.trim_end_matches('?'))
                                })
                        })
                    }) {
                        requested.push(ClientFeatureRequest {
                            field: "features",
                            value: value.clone(),
                        });
                    }
                }
            }
            if !requested.is_empty() {
                return Err(ProjectError::ClientFeatures {
                    mod_id: bundle.manifest.id.clone(),
                    requested,
                });
            }
        }
        Ok(Self {
            bundle,
            cargo,
            package,
            client_deps,
        })
    }
}

/// Add API packages and the generated executable to the fresh staged workspace.
pub(super) fn stage(
    stage: &Path,
    projects: &[Project<'_>],
    declared: &[Vec<super::dependencies::Dependency>],
    resolution: &super::dependencies::Resolution,
) -> Result<bool, String> {
    let mods: Vec<_> = resolution
        .order
        .iter()
        .map(|&i| (i, &projects[i]))
        .collect();
    if mods.is_empty() {
        return Ok(false);
    }
    let template = fs::read_to_string(stage.join(MAIN_TEMPLATE)).ok()
        .filter(|template| ["/* MOD_ENTRIES */", "/* BUILT_MODS */"].iter()
            .all(|marker| template.matches(marker).count() == 1))
        .ok_or_else(|| crate::i18n::tr("This game source has no native inventory entry template. Update Horizon and rebuild the mods against this game.").to_string())?;
    let packages: std::collections::HashMap<_, _> = mods
        .iter()
        .map(|(_, project)| (project.bundle.manifest.id.as_str(), project.package.clone()))
        .collect();
    let mut deps = toml::Table::new();
    // The generated game's features own sound/controllers, including the server's
    // `--no-default-features`; its client dependency never enables them independently.
    for name in ["horizon-client", "horizon-threads", "horizon-mods"] {
        let mut dependency = path_dep(&format!("../{name}"));
        if name == "horizon-client" {
            dependency.insert("default-features".into(), false.into());
        }
        deps.insert(name.into(), toml::Value::Table(dependency));
    }
    let mut members = Vec::new();
    for (i, (index, project)) in mods.iter().enumerate() {
        let bundle = project.bundle;
        let relative = format!("native-api/{}", bundle.manifest.id);
        let dest = stage.join(&relative);
        super::copy_bundle(bundle, &dest)?;
        let path = dest.join("Cargo.toml");
        let mut manifest = project.cargo.clone();
        let package = project.package.clone();
        for client in &project.client_deps {
            let dependency = client.value_mut(&mut manifest);
            let mut rewritten = path_dep("../../crates/horizon-client");
            if let Some(old) = dependency.as_table() {
                // A renamed dependency also needs its package name.
                for key in ["package", "features"] {
                    if let Some(value) = old.get(key) {
                        rewritten.insert(key.into(), value.clone());
                    }
                }
            }
            rewritten.insert("default-features".into(), false.into());
            *dependency = toml::Value::Table(rewritten);
        }
        let targets: Vec<_> = declared[*index]
            .iter()
            .map(|dependency| super::dependencies::Package {
                dependency,
                package: packages.get(dependency.id.as_str()).map(String::as_str),
            })
            .collect();
        let enabled = super::dependencies::rewrite(&bundle.manifest.id, &mut manifest, &targets)
            .map_err(|e| e.to_string())?;
        write(&path, &manifest)?;
        let mut dep = toml::Table::new();
        dep.insert("path".into(), format!("../../{relative}").into());
        dep.insert("package".into(), package.into());
        if !enabled.is_empty() {
            dep.insert(
                "features".into(),
                toml::Value::Array(enabled.into_iter().map(toml::Value::String).collect()),
            );
        }
        deps.insert(format!("mod_{i}"), toml::Value::Table(dep));
        members.push(relative);
    }
    // Beside horizon-client, so its build script (the Windows stack size and icon, for the
    // `horizon-client` binary this package owns now) finds `packaging/` where it looks.
    let game = stage.join(GAME);
    fs::create_dir_all(game.join("src")).map_err(|e| e.to_string())?;
    let mut manifest = toml::Table::new();
    manifest.insert(
        "package".into(),
        toml::Value::Table(toml::Table::from_iter([
            ("name".into(), PACKAGE.into()),
            ("version".into(), "0.1.0".into()),
            ("edition".into(), "2024".into()),
            ("publish".into(), false.into()),
            ("build".into(), "../horizon-client/build.rs".into()),
        ])),
    );
    // This sibling package uses the client's build.rs, so its build dependencies apply too;
    // relative paths still point to the same crates from crates/native-game.
    let client = read(&stage.join("crates/horizon-client/Cargo.toml"))?;
    if let Some(dependencies) = client.get("build-dependencies") {
        manifest.insert("build-dependencies".into(), dependencies.clone());
    }
    let windows = toml::Table::from_iter([(
        "build-dependencies".into(),
        toml::Value::Table(toml::Table::from_iter([(
            "winresource".into(),
            "0.1".into(),
        )])),
    )]);
    manifest.insert(
        "target".into(),
        toml::Value::Table(toml::Table::from_iter([(
            "cfg(windows)".into(),
            toml::Value::Table(windows),
        )])),
    );
    manifest.insert(
        "bin".into(),
        toml::Value::Array(vec![toml::Value::Table(toml::Table::from_iter([
            ("name".into(), "horizon-client".into()),
            ("path".into(), "src/main.rs".into()),
        ]))]),
    );
    manifest.insert("dependencies".into(), toml::Value::Table(deps));
    manifest.insert(
        "features".into(),
        toml::Value::Table(toml::Table::from_iter([(
            "default".into(),
            toml::Value::Array(vec!["horizon-client/default".into()]),
        )])),
    );
    write(&game.join("Cargo.toml"), &manifest)?;
    let manifests: Vec<_> = mods
        .iter()
        .map(|(_, project)| &project.bundle.manifest)
        .collect();
    fs::write(game.join("src/main.rs"), main_rs(&template, &manifests))
        .map_err(|e| e.to_string())?;
    let root = stage.join("Cargo.toml");
    let mut workspace = read(&root)?;
    let list = workspace
        .get_mut("workspace")
        .and_then(|w| w.get_mut("members"))
        .and_then(toml::Value::as_array_mut)
        .ok_or("workspace has no members")?;
    members.push(GAME.into());
    list.extend(members.into_iter().map(toml::Value::String));
    write(&root, &workspace)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> std::path::PathBuf {
        let name = std::thread::current().name().unwrap().replace(':', "-");
        let dir = super::super::tests::dir(&name);
        super::super::tests::write_bundle(&dir, "example", "");
        fs::write(dir.join("Cargo.toml"), "[package]\nname='example-mod'\nversion='9.0.0'\nedition='2024'\n[dependencies]\nhorizon-client='0.1'\n").unwrap();
        dir
    }

    #[test]
    fn server_projects_refuse_explicit_client_features_by_mod_and_field() {
        let dir = fixture();
        for (field, setting) in [
            ("default-features", "default-features = true"),
            ("features", "features = ['audio']"),
            ("features", "features = []"),
            ("features", "default-features = false\n[target.'cfg(unix)'.dependencies.platform_client]\npackage='horizon-client'\nversion='0.1'\nfeatures=[]"),
            ("default-features", "default-features = false\n[target.'cfg(unix)'.dependencies.platform_client]\npackage='horizon-client'\nversion='0.1'\ndefault-features=true"),
            ("features", "default-features = false\n[dependencies.z_alias]\npackage='horizon-client'\nversion='0.1'\nfeatures=['audio']"),
        ] {
            for dependency in ["horizon-client", "client_alias"] {
                let text = format!("[package]\nname='different_crate'\nversion='9.9.9'\n[dependencies.{dependency}]\npackage='horizon-client'\nversion='0.1'\n{setting}\n");
                fs::write(dir.join("Cargo.toml"), text).unwrap();
                let bundle = super::super::bundle_in(&dir).unwrap().unwrap();
                let error = Project::read(&bundle, super::super::Target::Server)
                    .err()
                    .unwrap();
                assert!(
                    matches!(&error, ProjectError::ClientFeatures { mod_id, requested } if mod_id == "example" && requested.len() == 1 && requested[0].field == field)
                );
                let message = error.to_string();
                for expected in ["example", field, "headless"] {
                    assert!(message.contains(expected), "{message}");
                }
                assert!(Project::read(&bundle, super::super::Target::Client).is_ok());
            }
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn server_projects_refuse_sdk_requests_through_features_tables() {
        let dir = fixture();
        for request in ["client_alias/audio", "client_alias?/controllers"] {
            fs::write(dir.join("Cargo.toml"), format!("[package]\nname='example-mod'\nversion='1.0.0'\n[dependencies.client_alias]\npackage='horizon-client'\nversion='0.1'\ndefault-features=false\n[features]\noptional_graphics=['{request}']\n")).unwrap();
            let bundle = super::super::bundle_in(&dir).unwrap().unwrap();
            let error = match Project::read(&bundle, super::super::Target::Server) {
                Err(error) => error,
                Ok(_) => {
                    panic!("SDK feature requests must not enable client libraries on a server")
                }
            };
            assert!(matches!(&error, ProjectError::ClientFeatures { .. }));
            assert!(error.to_string().contains(request));
            assert!(Project::read(&bundle, super::super::Target::Client).is_ok());
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn server_projects_accept_false_or_absent_default_features() {
        let dir = fixture();
        for dependency in [
            "horizon-client = '0.1'",
            "horizon-client = { version='0.1' }",
            "horizon-client = { version='0.1', default-features=false }",
        ] {
            fs::write(
                dir.join("Cargo.toml"),
                format!(
                    "[package]\nname='example'\nversion='1.0.0'\n[dependencies]\n{dependency}\n"
                ),
            )
            .unwrap();
            let bundle = super::super::bundle_in(&dir).unwrap().unwrap();
            assert!(Project::read(&bundle, super::super::Target::Server).is_ok());
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn generates_only_tool_contract_tables_without_mutating_package() {
        let dir = fixture();
        let path = dir.join(horizon_mods::manifest::MANIFEST);
        let text = fs::read_to_string(&path).unwrap()
            + "[[native.injection]]\nid='test'\ntarget='horizon_client::game::tick'\nkind='HEAD'\nhelper='src/lib.rs'\n";
        fs::write(&path, &text).unwrap();
        let bundle = super::super::bundle_in(&dir).unwrap().unwrap();
        assert!(bundle.needs_mixins());
        super::super::package::tool_manifest(&bundle).unwrap();
        let value = read(&dir.join(super::super::MANIFEST)).unwrap();
        assert!(!value.contains_key("api"));
        assert!(!value.contains_key("native"));
        assert!(value.contains_key("injection"));
        assert_eq!(value["mod"]["id"].as_str(), Some("example"));
        assert_eq!(fs::read_to_string(path).unwrap(), text);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn generated_main_keeps_mod_order() {
        let template = include_str!("../../../crates/horizon-client/src/native_main.rs.template");
        let dir = fixture();
        let mut base = horizon_mods::Package::read(&dir).unwrap().manifest;
        base.id = "base".into();
        base.version = "1.2.3".into();
        base.side = horizon_mods::Side::Both;
        let mut extra = base.clone();
        extra.id = "extra".into();
        extra.version = "2.0.0".into();
        extra.side = horizon_mods::Side::Client;
        let mods = [&base, &extra];
        for count in 0..=2 {
            let text = main_rs(template, &mods[..count]);
            let facts: Vec<_> = text
                .lines()
                .filter(|line| line.contains(" => mod_"))
                .collect();
            assert_eq!(
                facts,
                &[
                    "        \"base\" => mod_0::hooks,",
                    "        \"extra\" => mod_1::hooks,",
                ][..count]
            );
            let inventory: Vec<_> = text
                .lines()
                .filter(|line| line.contains("BuiltMod {"))
                .collect();
            assert_eq!(inventory, &[
                "    horizon_mods::runtime::BuiltMod { id: \"base\", version: \"1.2.3\", side: horizon_mods::Side::Both },",
                "    horizon_mods::runtime::BuiltMod { id: \"extra\", version: \"2.0.0\", side: horizon_mods::Side::Client },",
            ][..count]);
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn stages_api_without_a_dependency_cycle() {
        let dir = fixture();
        let cargo = fs::read_to_string(dir.join("Cargo.toml")).unwrap()
            + "[target.'cfg(unix)'.dependencies]\nhorizon-client = '0.1'\n";
        fs::write(dir.join("Cargo.toml"), cargo).unwrap();
        let bundle = super::super::bundle_in(&dir).unwrap().unwrap();
        let stage_dir = dir.with_extension("workspace");
        let _ = fs::remove_dir_all(&stage_dir);
        fs::create_dir_all(stage_dir.join("crates/horizon-client/src")).unwrap();
        fs::write(stage_dir.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
        fs::write(
            stage_dir.join("crates/horizon-client/Cargo.toml"),
            "[package]\nname = 'horizon-client'\nversion = '0.1.0'\n",
        )
        .unwrap();
        let resolution = super::super::dependencies::Resolution { order: vec![0] };
        let project = Project::read(&bundle, super::super::Target::Server).unwrap();
        let err = stage(
            &stage_dir,
            std::slice::from_ref(&project),
            &[vec![]],
            &resolution,
        )
        .unwrap_err();
        assert!(err.contains("Update Horizon"), "{err}");
        fs::write(
            stage_dir.join(MAIN_TEMPLATE),
            "fn main() { /* MOD_ENTRIES */ }",
        )
        .unwrap();
        assert!(stage(
            &stage_dir,
            std::slice::from_ref(&project),
            &[vec![]],
            &resolution
        )
        .is_err());
        assert!(!stage_dir.join("native-api").exists());
        fs::write(
            stage_dir.join(MAIN_TEMPLATE),
            include_str!("../../../crates/horizon-client/src/native_main.rs.template"),
        )
        .unwrap();
        assert!(stage(
            &stage_dir,
            std::slice::from_ref(&project),
            &[vec![]],
            &resolution
        )
        .unwrap());
        let root = read(&stage_dir.join("Cargo.toml")).unwrap();
        let members = root["workspace"]["members"].as_array().unwrap();
        assert!(members
            .iter()
            .any(|v| v.as_str() == Some("native-api/example")));
        assert!(members.iter().any(|v| v.as_str() == Some(GAME)));
        let engine = read(&stage_dir.join("native-api/example/Cargo.toml")).unwrap();
        assert_eq!(
            engine["dependencies"]["horizon-client"]["path"].as_str(),
            Some("../../crates/horizon-client")
        );
        let platform = &engine["target"]["cfg(unix)"]["dependencies"]["horizon-client"];
        assert_eq!(
            platform["path"].as_str(),
            Some("../../crates/horizon-client")
        );
        assert_eq!(platform["default-features"].as_bool(), Some(false));
        let game = read(&stage_dir.join(GAME).join("Cargo.toml")).unwrap();
        assert_eq!(
            game["dependencies"]["mod_0"]["package"].as_str(),
            Some("example-mod")
        );
        assert_eq!(
            game["dependencies"]["mod_0"]["path"].as_str(),
            Some("../../native-api/example")
        );
        assert_eq!(
            game["package"]["build"].as_str(),
            Some("../horizon-client/build.rs")
        );
        assert!(stage_dir
            .join(GAME)
            .join(
                game["dependencies"]["horizon-client"]["path"]
                    .as_str()
                    .unwrap()
            )
            .ends_with("crates/native-game/../horizon-client"));
        // The game's default features are the generated package's alone, so a server build's
        // `--no-default-features` reaches horizon-client.
        assert_eq!(
            engine["dependencies"]["horizon-client"]["default-features"].as_bool(),
            Some(false)
        );
        assert_eq!(
            game["dependencies"]["horizon-client"]["default-features"].as_bool(),
            Some(false)
        );
        assert_eq!(
            game["features"]["default"][0].as_str(),
            Some("horizon-client/default")
        );
        let main = || fs::read_to_string(stage_dir.join(GAME).join("src/main.rs")).unwrap();
        assert!(main().contains(r#""example" => mod_0::hooks,"#));
        fs::write(stage_dir.join("crates/horizon-client/Cargo.toml"), "[package]\nname = 'horizon-client'\nversion = '0.1.0'\n[build-dependencies]\nhorizon-mod-api-gen = { path = '../horizon-mod-api-gen' }\n").unwrap();
        let bundle = super::super::bundle_in(&dir).unwrap().unwrap();
        let project = Project::read(&bundle, super::super::Target::Client).unwrap();
        stage(&stage_dir, &[project], &[vec![]], &resolution).unwrap();
        assert!(main().contains(r#""example" => mod_0::hooks,"#));
        let game = read(&stage_dir.join(GAME).join("Cargo.toml")).unwrap();
        assert_eq!(
            game["build-dependencies"]["horizon-mod-api-gen"]["path"].as_str(),
            Some("../horizon-mod-api-gen")
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn named_source_without_an_entry_template_is_refused_before_staging() {
        let dir = fixture();
        let stage_dir = dir.with_extension("workspace");
        let _ = fs::remove_dir_all(&stage_dir);
        let source = stage_dir.join("crates/horizon-client/src");
        fs::create_dir_all(&source).unwrap();
        let resolution = super::super::dependencies::Resolution { order: vec![0] };
        for text in ["", "pub struct ModEntry { id: &'static str }"] {
            fs::write(source.join("mod_api.rs"), text).unwrap();
            let bundle = super::super::bundle_in(&dir).unwrap().unwrap();
            let project = Project::read(&bundle, super::super::Target::Client).unwrap();
            let error = stage(&stage_dir, &[project], &[vec![]], &resolution).unwrap_err();
            assert!(error.contains("Update Horizon and rebuild"), "{error}");
            assert!(!stage_dir.join(GAME).exists());
            assert!(!stage_dir.join("native-api").exists());
        }
        assert!(!stage(
            &stage_dir,
            &[],
            &[],
            &super::super::dependencies::Resolution { order: vec![] },
        )
        .unwrap());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn api_only_skips_mixins() {
        let dir = fixture();
        let bundle = super::super::bundle_in(&dir).unwrap().unwrap();
        assert!(!bundle.needs_mixins());
        fs::remove_dir_all(dir).unwrap();
    }
}
