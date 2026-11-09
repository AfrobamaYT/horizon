//! Mod points (#2339) in the native build: each code mod's crate is staged with a point on every
//! eligible function, by the game's own rules, and an addon's `[[inject]]` at another mod's
//! function routes to that mod's slot table. Every native mod links into one binary, so every
//! mod point here is on the addon's target.
use super::{internals::Catalog, Bundle};
use crate::i18n::trf;
use horizon_mod_package::manifest::inject::mod_points::{
    self, Catalog as Points, Library, Refusal, Routed,
};
use horizon_mod_package::manifest::inject::{Access, Inject};
use horizon_mod_package::Manifest;
use std::collections::BTreeSet;
use std::path::Path;

/// A code mod's points, listed from its source.
pub(super) struct ModLibrary {
    pub(super) id: String,
    pub(super) version: String,
    pub(super) points: Points,
}

impl Catalog {
    /// This catalog with the points of every code mod in `bundles`, listed from their source.
    pub(super) fn with_mods<'a>(
        mut self,
        bundles: impl IntoIterator<Item = &'a Bundle>,
    ) -> Result<Self, String> {
        for bundle in bundles
            .into_iter()
            .filter(|bundle| bundle.manifest.native.is_some())
        {
            let points =
                horizon_mod_api_gen::points::mod_crate::mod_points(&bundle.dir, &self.wire)
                    .map_err(|error| format!("{}: {error}", bundle.manifest.id))?;
            self.mods.push(ModLibrary {
                id: bundle.manifest.id.clone(),
                version: bundle.manifest.version.clone(),
                points,
            });
        }
        Ok(self)
    }

    /// The mod whose crate owns `at`, a mod point's target.
    pub(super) fn mod_owner(&self, at: &str) -> Option<&ModLibrary> {
        self.mods.iter().find(|library| library.points.owns(at))
    }
}

/// Stages the mod crate copied to `dir`: its points, for the table its own build compiles.
pub(super) fn stage(dir: &Path, wire: &BTreeSet<String>) -> Result<(), String> {
    horizon_mod_api_gen::points::mod_crate::stage_mod(dir, wire).map(|_| ())
}

/// `manifest`'s injections at other mods' points, each with its library and place.
pub(super) fn routes<'a>(
    catalog: &'a Catalog,
    manifest: &'a Manifest,
) -> Result<Vec<(&'a ModLibrary, Routed<'a>, &'a Inject)>, Refusal> {
    manifest
        .inject
        .iter()
        .filter_map(|inject| {
            catalog
                .mod_owner(&inject.at)
                .map(|library| (library, inject))
        })
        .map(|(library, inject)| {
            let routed = mod_points::route(
                &manifest.id,
                inject,
                &Library {
                    id: &library.id,
                    version: &library.version,
                    same_target: true,
                    catalog: &library.points,
                },
            )?;
            Ok((library, routed, inject))
        })
        .collect()
}

/// The table module of mod `library` as the generated `main` names it: `mods` is the build's
/// order, in which `main` names each mod `mod_<index>`.
fn table(library: &ModLibrary, mods: &[&Manifest]) -> String {
    let index = mods
        .iter()
        .position(|manifest| manifest.id == library.id)
        .expect("a mod point's library is in the build");
    format!("mod_{index}::{}", library.points.table)
}

/// Every mod point as the shared API constructor emitter reads it, under its table.
pub(super) fn emitted(catalog: &Catalog, mods: &[&Manifest]) -> Vec<horizon_mod_point_emit::Point> {
    catalog
        .mods
        .iter()
        .flat_map(|library| {
            let table = table(library, mods);
            library
                .points
                .points
                .iter()
                .map(move |point| horizon_mod_point_emit::Point {
                    name: point.name.clone(),
                    target: point.target.clone(),
                    private: false,
                    at_return: point.at_return,
                    api_refusal: point.api_refusal.clone(),
                    returns: point.returns.clone(),
                    host: point.host.clone(),
                    raw_shape: None,
                    return_ty: Some(point.return_ty.clone()),
                    table: Some(table.clone()),
                })
        })
        .collect()
}

/// The generated `main`'s internals listeners at mod points, for mod `index` of `mods`.
pub(super) fn internals(catalog: &Catalog, mods: &[&Manifest], index: usize) -> String {
    let manifest = mods[index];
    routes(catalog, manifest)
        .expect("mod point routes checked")
        .into_iter()
        .filter(|(_, routed, _)| routed.access == Access::Internals)
        .map(|(library, routed, inject)| {
            let line = horizon_mod_point_emit::mod_internals(
                &table(library, mods),
                &routed.place,
                &manifest.id,
                &format!("mod_{index}::{}", inject.with),
                inject.priority,
                &inject.id,
            );
            format!("        {line},\n")
        })
        .collect()
}

/// The launcher's words for a mod point refusal: the package crate's `Display` is the
/// command-line form, this is the translated one players see.
pub(super) fn refusal(refusal: &Refusal) -> String {
    match refusal {
        Refusal::Inject(error) => super::inject_text::inject_error(error),
        Refusal::Changed {
            id,
            library,
            version,
            at,
            pinned,
            installed,
            signature,
        } => trf(
            "E_INJECT_CHANGED: {id}: {library} {version} changed {at}: pinned {pinned}, installed {installed} ({signature}); update the declaration with `cargo horizon-mod inject update` and rebuild against it",
            &[
                ("id", id),
                ("library", library),
                ("version", version),
                ("at", at),
                ("pinned", pinned),
                ("installed", installed),
                ("signature", signature),
            ],
        ),
        Refusal::OptedOut { id, at } => trf(
            "E_INJECT_MISSING: {id}: {at} has no point (opted out with #[mod_api::no_point])",
            &[("id", id), ("at", at)],
        ),
        Refusal::CrossTargetInternals { id, library } => trf(
            "{id}: internals access to {library} needs the same target: the same binary on native, the same dependency group on wasm",
            &[("id", id), ("library", library)],
        ),
        Refusal::CrossTargetNoWire {
            id,
            library,
            reason,
        } => trf(
            "{id}: an api listener at {library} on the other target crosses through the game, and {reason}: add #[derive(mod_api::Wire)] to it",
            &[("id", id), ("library", library), ("reason", reason)],
        ),
        Refusal::CrossTarget { id, library } => trf(
            "{id}: {library} is on the other target: mixed native and wasm dependencies aren't supported yet",
            &[("id", id), ("library", library)],
        ),
    }
}
