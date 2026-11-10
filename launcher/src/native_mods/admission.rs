//! Admission precedes offline resolution, even when Cargo's cache is empty.
use super::{crates::Wanted, refuse_unlocked};
use std::{collections::BTreeSet, path::Path};

pub(super) fn resolve(
    source: &Path,
    wanted: &[Wanted],
    has_api: bool,
    metadata: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    refuse_unlocked(source, wanted)?;
    if has_api {
        metadata()?;
    }
    Ok(())
}

/// Lexically sorted unique versions, capped at four; +N counts omitted versions.
pub(super) fn versions<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let values: BTreeSet<_> = values.collect();
    let mut text = values
        .iter()
        .take(4)
        .copied()
        .collect::<Vec<_>>()
        .join(", ");
    if values.len() > 4 {
        text.push_str(&format!(", +{}", values.len() - 4));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusal_precedes_the_metadata_runner() {
        let source = tempfile::tempdir().unwrap();
        std::fs::write(
            source.path().join("Cargo.lock"),
            "[[package]]\nname = \"zip\"\nversion = \"9.1.3\"\nsource = \"registry+x\"\n",
        )
        .unwrap();
        let wanted = vec![Wanted {
            mod_id: "example".into(),
            name: "zip".into(),
            version: "2".into(),
        }];
        let error = resolve(source.path(), &wanted, true, || {
            panic!("offline metadata ran before refusal")
        })
        .unwrap_err();
        assert!(error.contains("example needs zip 2"), "{error}");
        let called = std::cell::Cell::new(false);
        resolve(source.path(), &[], true, || {
            called.set(true);
            Ok(())
        })
        .unwrap();
        assert!(called.get());
    }

    #[test]
    fn diagnostic_versions_are_unique_sorted_and_bounded() {
        assert_eq!(versions(["2", "1", "2"].into_iter()), "1, 2");
        assert_eq!(
            versions(["6", "5", "4", "3", "2", "1", "6"].into_iter()),
            "1, 2, 3, 4, +2"
        );
    }
}
