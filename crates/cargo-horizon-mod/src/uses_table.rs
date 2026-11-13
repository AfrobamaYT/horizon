//! What a code mod uses of the surface (design-fit.md 10.2): the scan of its source against the
//! pinned SDK's `surface.json`, the table `build` packs as `uses.json`, and `check`'s summary.
use crate::sdk_fetch::PinnedSdk;
use horizon_mod_api_gen::mod_uses::{self, ReadError};
use mod_surface::child::ChildEntry;
use mod_surface::uses::{Built, ManifestFacts, Origin, Uses, UsesTable, uses_contained};
use mod_surface::{Surface, leaf};
use std::collections::BTreeMap;
use std::path::Path;

/// A surface file read and checked.
pub(crate) fn read_surface(path: &Path) -> Result<Surface, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Surface::read(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// What the mod at `root` uses of `surface`; `None` for a mod made for exactly one build (a
/// `[native.*]` table or internals), where the question does not apply.
pub(crate) fn scan(
    entry: ChildEntry,
    root: &Path,
    surface: &Surface,
) -> Result<Option<Uses>, String> {
    let input = match mod_uses::read(root) {
        Ok(input) => input,
        Err(ReadError::Exact(_)) => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    // No manifest form names a hook version today (the schema refuses `hook_version`).
    let manifest = ManifestFacts {
        names: input.manifest_names,
        hook_versions: Default::default(),
    };
    uses_contained(
        entry,
        input.sources.iter().map(|(p, t)| (p.as_str(), t.as_str())),
        &manifest,
        surface,
    )
    .map(Some)
    .map_err(|e| e.to_string())
}

/// The table `build` packs for the mod at `root`, made for the pinned build; `None` when the
/// mod is made for exactly one build.
pub(crate) fn table(
    entry: ChildEntry,
    root: &Path,
    sdk: &PinnedSdk,
) -> Result<Option<UsesTable>, String> {
    let surface = read_surface(&sdk.surface)?;
    Ok(scan(entry, root, &surface)?.map(|uses| uses.table(sdk.build.clone())))
}

/// `Greeter uses 14 items of the API (6 hooks, 5 methods, 3 types). Made for nightly-….`, then
/// what the author should know about how it builds each struct.
pub(crate) fn summary(name: &str, uses: &Uses, sdk: &PinnedSdk) -> String {
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for used in &uses.items {
        *kinds.entry(kind_word(&used.entry.id)).or_default() += 1;
    }
    let counts: Vec<String> = kinds
        .iter()
        .map(|(word, count)| format!("{count} {}", plural(word, *count)))
        .collect();
    let mut text = format!(
        "{name} uses {} {} of the API ({}). Made for {}.\n",
        uses.items.len(),
        plural("item", uses.items.len()),
        counts.join(", "),
        sdk.name
    );
    for note in uses.items.iter().filter_map(|used| used.note.as_deref()) {
        text.push_str(note);
        text.push('\n');
    }
    text
}

/// `check --uses`: each item with the place that brought it in.
pub(crate) fn list(uses: &Uses) -> String {
    let mut text = String::new();
    for used in &uses.items {
        let built = match used.entry.built {
            Some(Built::Full) => "  built field by field",
            Some(Built::Base) => "  built with `..`",
            _ => "",
        };
        text.push_str(&format!(
            "{}  {}{built}\n",
            used.entry.id,
            place(&used.origin)
        ));
    }
    text
}

/// Where an item came into the list.
pub(crate) fn place(origin: &Origin) -> String {
    match origin {
        Origin::Source { file, line } => format!("{file}:{line}"),
        Origin::Manifest => horizon_mod_package::manifest::MANIFEST.to_owned(),
        Origin::Via(id) => format!("through {}", leaf(id)),
    }
}

/// The word for an id's kind, as the summary counts it.
fn kind_word(id: &str) -> &'static str {
    match id.split_once(':') {
        Some(("hook", rest)) if rest.contains('.') => "hook argument",
        Some(("hook", _)) => "hook",
        Some(("trait", _)) => "trait",
        Some(("method" | "handle", _)) => "method",
        Some(("assoc", _)) => "associated item",
        Some(("fn", _)) => "function",
        Some(("macro", _)) => "macro",
        Some(("part", _)) => "Part kind",
        Some(("capability", _)) => "capability",
        Some(("type", rest)) if rest.contains('.') => "field or variant",
        Some(("type", _)) => "type",
        _ => "item",
    }
}

fn plural(word: &str, count: usize) -> String {
    match (word, count) {
        (_, 1) => word.to_owned(),
        ("field or variant", _) => "fields or variants".to_owned(),
        ("associated item", _) => "associated items".to_owned(),
        ("hook argument", _) => "hook arguments".to_owned(),
        ("Part kind", _) => "Part kinds".to_owned(),
        _ => format!("{word}s"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_summary_counts_by_kind_in_words() {
        let used = |id: &str| mod_surface::uses::Used {
            entry: mod_surface::uses::Entry {
                id: id.into(),
                hash: "h".into(),
                impls: vec![],
                built: None,
                fields_hash: None,
                fields: vec![],
                built_at: None,
                paths: vec![],
                reached_from: vec![],
                version: None,
            },
            origin: Origin::Source {
                file: "src/lib.rs".into(),
                line: 3,
            },
            note: None,
        };
        let uses = Uses {
            items: vec![
                used("hook:player_logged_in"),
                used("hook:block_broken"),
                used("method:ServerHost::give_item"),
                used("type:ItemStack"),
            ],
        };
        let sdk = PinnedSdk {
            build: "a".repeat(40),
            name: "nightly-2026-11-13 (afe471c)".into(),
            dir: "sdk".into(),
            surface: "sdk/surface.json".into(),
        };
        assert_eq!(
            summary("Greeter", &uses, &sdk),
            "Greeter uses 4 items of the API (2 hooks, 1 method, 1 type). Made for nightly-2026-11-13 (afe471c).\n"
        );
        assert_eq!(
            list(&uses).lines().next(),
            Some("hook:player_logged_in  src/lib.rs:3")
        );
    }
}
