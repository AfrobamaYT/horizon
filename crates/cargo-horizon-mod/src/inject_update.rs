//! Transactional freshness updates; never changes the game pin.
use super::*;
use horizon_mod_api_gen::sites::identity::{CallIdentity, Selection, select};

pub(super) fn run(dir: &Path, args: &[String]) -> Result<(), String> {
    let mut accept = Vec::new();
    for pair in args.chunks(2) {
        match pair {
            [flag, id] if flag == "--accept" => accept.push(id.as_str()),
            _ => return Err("usage: cargo horizon-mod inject update [--accept <id>]…".into()),
        }
    }
    let path = dir.join(MANIFEST);
    let original = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let manifest = Manifest::parse(&original)?;
    let native = manifest
        .native
        .as_ref()
        .ok_or("inject update needs a [native] mod")?;
    let internals = crate::internals::load(dir, &native.horizon)?;
    let cache = dir.join("target/horizon-mod/prebuilt");
    let points = cache.join(&native.horizon).join("points.json");
    let catalog: Catalog =
        serde_json::from_str(&fs::read_to_string(points).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    // Older cached snapshots provide the signature behind a declaration's old hash.
    let mut old = Vec::new();
    for entry in fs::read_dir(cache).map_err(|e| e.to_string())? {
        let path = entry
            .map_err(|e| e.to_string())?
            .path()
            .join("internals.json");
        if path.is_file() {
            let snapshot: Vec<ListedInternal> =
                serde_json::from_str(&fs::read_to_string(path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            old.extend(snapshot);
        }
    }
    let (candidate, reports, refused) =
        plan(&original, &manifest, &internals, &catalog, &old, &accept)?;
    for report in reports {
        println!("{report}");
    }
    if refused {
        return Err("inject update refused; no files changed".into());
    }
    if candidate != original {
        crate::internals::atomic_write(&path, candidate)?;
    }
    Ok(())
}

fn plan(
    text: &str,
    manifest: &Manifest,
    internals: &[ListedInternal],
    catalog: &Catalog,
    old: &[ListedInternal],
    accept: &[&str],
) -> Result<(String, Vec<String>, bool), String> {
    let mut doc: DocumentMut = text.parse().map_err(|e| format!("{MANIFEST}: {e}"))?;
    let mut reports = Vec::new();
    let mut refused = false;
    for id in accept {
        if !manifest.inject.iter().any(|i| i.id == *id) {
            reports.push(format!("unknown inject id {id}"));
            refused = true;
        }
    }
    for (index, inject) in manifest.inject.iter().enumerate() {
        let id = format!("{}:{}", manifest.id, inject.id);
        let kind = if inject.kind == Kind::Field {
            "struct"
        } else {
            "fn"
        };
        let item = internals.iter().find(|i| {
            i.path == inject.at
                && !i.shape_only
                && (i.kind == kind || (kind == "fn" && i.kind == "method"))
        });
        let Some(item) = item else {
            reports.push(
                InjectError::Missing {
                    id,
                    at: inject.at.clone(),
                }
                .to_string(),
            );
            refused = true;
            continue;
        };
        let item = match horizon_mod_package::internals::resolve(
            internals,
            &id,
            &inject.at,
            Some(&item.kind),
        ) {
            Ok(item) => item,
            Err(error) => {
                reports.push(error.to_string());
                refused = true;
                continue;
            }
        };
        let mut nth = None;
        if inject.ctx.is_some() {
            let identities: Vec<_> = catalog
                .sites
                .iter()
                .filter(|s| s.target == inject.at)
                .map(|s| CallIdentity {
                    call: &s.call,
                    nth: s.ordinal,
                    ctx: &s.ctx,
                })
                .collect();
            match select(
                &identities,
                inject.call.as_deref().unwrap_or(""),
                inject.nth.unwrap_or(0),
                inject.ctx.as_deref().unwrap_or(""),
            ) {
                Selection::Selected(_) => (),
                Selection::Moved(n) => {
                    reports.push(
                        InjectError::Moved {
                            id: id.clone(),
                            nth: n,
                        }
                        .to_string(),
                    );
                    nth = Some(n);
                }
                Selection::Lost | Selection::Ambiguous => {
                    reports.push(
                        InjectError::Lost {
                            id,
                            at: inject.at.clone(),
                        }
                        .to_string(),
                    );
                    refused = true;
                    continue;
                }
            }
        }
        let changed = inject.hash != item.hash;
        if changed {
            let previous = old
                .iter()
                .find(|i| i.path == inject.at && i.hash == inject.hash)
                .map(|i| i.signature.as_str())
                .unwrap_or("signature unavailable in cached snapshots");
            reports.push(
                InjectError::Changed {
                    id,
                    at: inject.at.clone(),
                    old: format!("{} ({previous})", inject.hash),
                    new: format!("{} ({})", item.hash, item.signature),
                }
                .to_string(),
            );
            if !accept.contains(&inject.id.as_str()) {
                refused = true;
                continue;
            }
        }
        let table = doc
            .get_mut("inject")
            .and_then(|i| i.as_array_of_tables_mut())
            .and_then(|a| a.get_mut(index))
            .ok_or("missing inject table")?;
        // Replacing the scalar payload retains its existing comments and spacing.
        for (key, replacement) in [
            (
                "hash",
                changed.then(|| toml_edit::Value::from(item.hash.as_str())),
            ),
            (
                "nth",
                nth.map(|n| i64::try_from(n).map(toml_edit::Value::from))
                    .transpose()
                    .map_err(|_| "call ordinal exceeds TOML integer range")?,
            ),
        ] {
            if let Some(mut replacement) = replacement {
                if let Some(existing) = table.get(key).and_then(|i| i.as_value()) {
                    *replacement.decor_mut() = existing.decor().clone();
                }
                table[key] = toml_edit::Item::Value(replacement);
            }
        }
    }
    let candidate = if refused {
        text.to_owned()
    } else {
        doc.to_string()
    };
    Manifest::parse(&candidate)?;
    Ok((candidate, reports, refused))
}

#[cfg(test)]
mod tests {
    use super::super::tests::{TEXT, edit, fixture, reviewer_directory};
    use super::*;
    fn pinned() -> String {
        edit(
            TEXT,
            "add game::tick --kind wrap_call --call game::first --with wrap",
        )
        .unwrap()
    }
    fn check(
        text: &str,
        items: &[ListedInternal],
        catalog: &Catalog,
        accept: &[&str],
    ) -> (String, Vec<String>, bool) {
        plan(
            text,
            &Manifest::parse(text).unwrap(),
            items,
            catalog,
            &[],
            accept,
        )
        .unwrap()
    }
    #[test]
    fn inject_update_movement_and_format() {
        let text = pinned().replace("nth = 0", "nth = 0 # ordinal");
        let (items, mut catalog) = fixture();
        assert_eq!(check(&text, &items, &catalog, &[]).0, text);
        catalog.sites[0].ordinal = 7;
        let (result, reports, refused) = check(&text, &items, &catalog, &[]);
        assert!(!refused);
        assert_eq!(result, text.replace("nth = 0", "nth = 7"));
        assert!(reports[0].contains("E_INJECT_MOVED"));
        catalog.sites[0].ctx = "different".into();
        let (result, reports, refused) = check(&text, &items, &catalog, &["wrap"]);
        assert!(refused && reports[0].contains("E_INJECT_LOST"));
        assert_eq!(result, text);
        catalog.sites[0].ctx = "context-zero".into();
        catalog.sites.push(Call {
            target: "game::tick".into(),
            call: "game::first".into(),
            ordinal: 9,
            ctx: "context-zero".into(),
        });
        assert!(check(&text, &items, &catalog, &[]).2);
    }
    #[test]
    fn inject_update_signature_and_atomic_plan() {
        let text = pinned();
        let (mut items, catalog) = fixture();
        items[0].hash = "new-signature".into();
        items[0].signature = "unsafe fn tick(&self, value: bool) -> bool".into();
        let (result, reports, refused) = check(&text, &items, &catalog, &[]);
        assert!(refused);
        assert_eq!(result, text);
        assert!(
            reports[0].contains("E_INJECT_CHANGED")
                && reports[0].contains("unsafe fn tick(&self, value: bool) -> bool")
        );
        let (result, _, refused) = check(&text, &items, &catalog, &["wrap"]);
        assert!(!refused);
        assert_eq!(result, text.replace("signature-fixture", "new-signature"));
        assert!(check(&text, &[], &catalog, &["wrap"]).2);
        let multi = edit(&text, "add game::tick --kind head --with second").unwrap();
        let (result, _, refused) = check(&multi, &items, &catalog, &["wrap"]);
        assert!(refused);
        assert_eq!(result, multi);
        assert!(check(&text, &items, &catalog, &["unknown"]).2);
    }
    #[test]
    fn inject_update_noop_leaves_inode_and_mtime() {
        let dir = reviewer_directory("update-noop");
        let path = dir.join(MANIFEST);
        fs::write(&path, edit(TEXT, "add game::tick --kind head").unwrap()).unwrap();
        let before = fs::metadata(&path).unwrap().modified().unwrap();
        fs::hard_link(&path, dir.join("old")).unwrap();
        run(&dir, &[]).unwrap();
        assert_eq!(before, fs::metadata(&path).unwrap().modified().unwrap());
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            assert_eq!(
                fs::metadata(&path).unwrap().ino(),
                fs::metadata(dir.join("old")).unwrap().ino()
            );
        }
        fs::remove_dir_all(dir).unwrap();
    }
}
