//! `docs/modding/src/reference/hook-notes.md` from each declaration's `notes` attribute.
//!
//! A hook or helper carries its prose on its own declaration, so two PRs that add different
//! hooks never edit the same page. `cargo run -p xtask -- mod-api notes` writes the page; the
//! generator's tests fail when the checked-in page is stale.
use crate::catalog::Catalog;
use std::path::Path;

/// The page's title, before the declarations' sections.
const HEADER: &str = "# Hook notes\n";

/// Renders the page from the declarations' notes, in catalog order: hooks first, then helpers.
pub fn render(catalog: &Catalog) -> String {
    let mut page = String::from(HEADER);
    for notes in catalog
        .general_notes
        .as_deref()
        .into_iter()
        .chain(
            catalog
                .hooks
                .iter()
                .filter_map(|hook| hook.notes.as_deref()),
        )
        .chain(
            catalog
                .helpers
                .iter()
                .filter_map(|helper| helper.notes.as_deref()),
        )
    {
        page.push('\n');
        page.push_str(notes.trim_end());
        page.push('\n');
    }
    page
}

/// Reads the checkout's catalog and renders the page.
pub fn generate(root: &Path) -> Result<String, String> {
    let catalog = Catalog::load(root).map_err(|error| error.to_string())?;
    Ok(render(&catalog))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn the_checked_in_page_matches_the_declarations() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let page = super::generate(&root).unwrap();
        let path = root.join("docs/modding/src/reference/hook-notes.md");
        let checked_in = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            checked_in, page,
            "run `cargo run -p xtask -- mod-api notes`"
        );
    }
}
