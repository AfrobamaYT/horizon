//! API engine point validation. No source or listed-call-site fallback exists.
use super::internals::Catalog;
use horizon_mod_package::{manifest::inject::Kind, Manifest};

pub(super) fn check(catalog: &Catalog, manifest: &Manifest) -> Result<(), String> {
    for entry in &manifest.inject {
        let id = format!("{}:{}", manifest.id, entry.id);
        let point = catalog
            .points
            .iter()
            .find(|point| {
                super::internals::plain_path(&point.target)
                    == super::internals::plain_path(&entry.at)
            })
            .ok_or_else(|| format!("E_INJECT_MISSING: {id}: {} has no API point", entry.at))?;
        let target = catalog
            .target(&entry.at)
            .ok_or_else(|| format!("E_INJECT_MISSING: {id}: {} has no signature", entry.at))?;
        if entry
            .hash
            .as_deref()
            .is_some_and(|hash| hash != target.hash)
        {
            return Err(format!(
                "E_INJECT_CHANGED: {id}: {}: pinned {}, installed {} ({})",
                entry.at,
                entry.hash.as_deref().unwrap_or("missing"),
                target.hash,
                target.signature
            ));
        }
        if let Some(reason) = &point.api_refusal {
            return Err(format!("{id}: no API listener at {}: {reason}", entry.at));
        }
        if entry.kind == Kind::Return && !point.at_return {
            return Err(format!("{id}: no API return point at {}", entry.at));
        }
        if entry.kind == Kind::Replace {
            horizon_mod_point_emit::replaceable(&id, &entry.at, &point.emitted())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
