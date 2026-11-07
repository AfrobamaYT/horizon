//! Read-only classification of a native mod set, without executing mod or game code.
use super::*;

/// The inputs owned by a preview or an actual build attempt.
pub enum PlanInput<'a> {
    /// Snapshot the enabled packages of this game home.
    Installed(&'a Path),
    /// Reuse an already-staged input folder, or a CLI project folder.
    Folder(&'a Path),
}

/// Plan native mods against their game. Worker-only filesystem and release reads.
pub fn routes(input: PlanInput<'_>, base: Option<&str>) -> Result<Routes, Failed> {
    require_consent()?;
    let lease = crate::native_cache::Cache::acquire(&backend::build_dir()).map_err(cache_error)?;
    let folder = match input {
        PlanInput::Installed(home) => {
            let Some(folder) = stage_installed(home, root().join("installed.routes"))? else {
                return Ok(Routes::default());
            };
            folder
        }
        PlanInput::Folder(folder) => folder.to_owned(),
    };
    let found = scan_bundles(&folder)?;
    let mut result = Routes::default();
    let mut projects = Vec::new();
    for bundle in &found {
        let route = match api::Project::read(bundle) {
            Ok(project) => {
                let route = prebuilt::project_refusal(&project)
                    .map_or(Route::Prebuilt, |reason| Route::Game { reason });
                projects.push(project);
                route
            }
            Err(error) => Route::Invalid {
                reason: error.to_string(),
            },
        };
        result.mods.push((bundle.manifest.id.clone(), route));
    }
    if projects.is_empty() {
        return Ok(result);
    }
    let base = base.ok_or_else(|| tr("Install Horizon first.").to_string())?;
    let revision = build_revision(base, checked_about(base)?.as_ref())?;
    if let Some(list) = mismatch(&found, &revision) {
        return Err(mismatch_error(&list).into());
    }
    // Trait mods need only the release listing; compiled slots need a checked SDK catalog.
    if projects
        .iter()
        .all(|project| project.bundle.manifest.inject.is_empty())
    {
        result.unavailable = prebuilt::prebuilt_availability(&revision, Target::Client);
    } else {
        match prebuilt::planning_catalog(&lease, &revision) {
            Ok(catalog) => {
                for (id, route) in prebuilt::slot_routes(&catalog, &projects) {
                    if route != Route::Prebuilt {
                        let entry = result
                            .mods
                            .iter_mut()
                            .find(|(mod_id, _)| *mod_id == id)
                            .expect("slot decisions cover the parsed projects");
                        entry.1 = route;
                    }
                }
            }
            Err(reason) => result.unavailable = Some(reason),
        }
    }
    Ok(result)
}
