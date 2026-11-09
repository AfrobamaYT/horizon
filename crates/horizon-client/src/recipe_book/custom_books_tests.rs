//! The design's storage, persistence, copy and replace acceptance cases (#2415).
use super::*;
use crate::recipe_book::{BookType, Settings};

const POT: &str = "demo:cooking_pot";
const OVEN: &str = "demo:oven";
const OPEN: BookFlags = BookFlags {
    open: true,
    filtering: false,
};
const FILTERING: BookFlags = BookFlags {
    open: false,
    filtering: true,
};

/// A saved `recipeBook` compound: the vanilla lists, the furnace book open and `extension`.
fn saved(extension: Option<Tag>) -> Tag {
    let mut entries = vec![
        ("recipes", Tag::List(vec![])),
        ("toBeDisplayed", Tag::List(vec![])),
        ("isFurnaceGuiOpen", Tag::Byte(1)),
    ];
    entries.extend(extension.map(|tag| (EXTENSION, tag)));
    Tag::compound(entries)
}

fn entry(open: Tag, filtering: Tag) -> Tag {
    Tag::compound([("open", open), ("filtering", filtering)])
}

#[test]
fn a_fresh_book_reads_closed_and_one_book_leaves_another_alone() {
    let mut books = CustomBooks::default();
    assert_eq!(books.get(POT), BookFlags::default());
    assert_eq!(books.get(OVEN), BookFlags::default());
    assert_eq!(books.set(POT, OPEN), Ok(true));
    assert_eq!(books.get(POT), OPEN);
    assert_eq!(books.get(OVEN), BookFlags::default());
    assert_eq!(books.set(POT, OPEN), Ok(false), "an unchanged pair");
    assert_eq!(books.set(OVEN, BookFlags::default()), Ok(false));
    assert_eq!(books.len(), 1, "a default pair adds no entry");
}

#[test]
fn a_saved_book_loads_exactly_and_the_vanilla_pairs_stay() {
    let mut settings = Settings::read(Some(&saved(None)));
    settings.filtering[BookType::Crafting.index()] = true;
    settings.custom.set(POT, OPEN).unwrap();
    let vanilla = (settings.open, settings.filtering);
    let loaded = Settings::read(Some(&settings.write()));
    assert_eq!(loaded.custom.get(POT), OPEN);
    assert_eq!((loaded.open, loaded.filtering), vanilla);
    assert_eq!(loaded.custom, settings.custom);
}

#[test]
fn both_flags_are_written_zero_too_in_id_order() {
    let mut books = CustomBooks::default();
    books.set(POT, FILTERING).unwrap();
    books.set("demo:anvil", OPEN).unwrap();
    let Some(Tag::Compound(written)) = books.write() else {
        panic!("an extension");
    };
    let ids: Vec<String> = written.keys().map(JavaString::to_string_lossy).collect();
    assert_eq!(ids, ["demo:anvil", POT]);
    assert_eq!(
        written.get(&JavaString::from(POT)),
        Some(&entry(Tag::Byte(0), Tag::Byte(1)))
    );
    assert_eq!(
        CustomBooks::default().write(),
        None,
        "no books, no extension"
    );
}

#[test]
fn a_book_without_its_mod_is_kept_through_save_and_load() {
    // Nothing is registered here: storage keeps every well-formed entry, so the pot's flags
    // are back when its mod is installed again.
    let tag = saved(Some(Tag::compound([(
        POT,
        entry(Tag::Byte(1), Tag::Byte(0)),
    )])));
    let loaded = Settings::read(Some(&tag));
    assert_eq!(loaded.custom.get(POT), OPEN);
    let again = Settings::read(Some(&loaded.write()));
    assert_eq!(again.custom.get(POT), OPEN);
}

#[test]
fn a_copy_is_independent_and_a_replace_drops_what_only_the_destination_had() {
    let mut a = Settings::default();
    a.custom.set(POT, OPEN).unwrap();
    let b = a.clone();
    a.custom.set(POT, FILTERING).unwrap();
    assert_eq!(b.custom.get(POT), OPEN, "the copy is a value");
    let mut destination = Settings::default();
    destination
        .custom
        .set(
            OVEN,
            BookFlags {
                open: true,
                filtering: true,
            },
        )
        .unwrap();
    destination = b.clone();
    assert_eq!(destination.custom.iter().collect::<Vec<_>>(), [(POT, OPEN)]);
    assert_eq!(destination.custom.get(OVEN), BookFlags::default());
}

#[test]
fn a_book_without_the_extension_reads_empty() {
    assert!(Settings::read(Some(&saved(None))).custom.is_empty());
    assert!(Settings::read(None).custom.is_empty());
    assert!(
        !matches!(Settings::default().write().get(EXTENSION), Some(_)),
        "a vanilla-only book writes what it wrote before"
    );
}

#[test]
fn lists_that_do_not_decode_keep_the_mods_flags() {
    let tag = Tag::compound([(
        EXTENSION,
        Tag::compound([(POT, entry(Tag::Byte(1), Tag::Byte(0)))]),
    )]);
    let loaded = Settings::read(Some(&tag));
    assert!(loaded.recipes.is_empty(), "no lists: vanilla reads nothing");
    assert_eq!(loaded.custom.get(POT), OPEN);
}

#[test]
fn a_malformed_entry_is_skipped_and_the_rest_read() {
    let extension = Tag::compound([
        (POT, entry(Tag::Byte(1), Tag::Byte(0))),
        ("demo:two", entry(Tag::Byte(2), Tag::Byte(0))),
        ("demo:int", entry(Tag::Int(1), Tag::Byte(0))),
        ("demo:half", Tag::compound([("open", Tag::Byte(1))])),
        ("demo:flat", Tag::Byte(1)),
        ("Demo:Upper", entry(Tag::Byte(1), Tag::Byte(1))),
        ("no_namespace", entry(Tag::Byte(1), Tag::Byte(1))),
    ]);
    let books = CustomBooks::read(Some(&saved(Some(extension))));
    assert_eq!(books.iter().collect::<Vec<_>>(), [(POT, OPEN)]);
}

#[test]
fn an_extension_over_a_limit_reads_empty_and_vanilla_still_reads() {
    let books = |n: usize| {
        Tag::Compound(
            (0..n)
                .map(|i| {
                    (
                        JavaString::from(format!("demo:book_{i}").as_str()),
                        entry(Tag::Byte(1), Tag::Byte(0)),
                    )
                })
                .collect(),
        )
    };
    assert_eq!(
        CustomBooks::read(Some(&saved(Some(books(MAX_BOOKS))))).len(),
        MAX_BOOKS
    );
    let over = saved(Some(books(MAX_BOOKS + 1)));
    let settings = Settings::read(Some(&over));
    assert!(settings.custom.is_empty(), "too many books");
    assert!(
        settings.open[BookType::Furnace.index()],
        "vanilla pairs still read"
    );

    let long = format!("demo:{}", "a".repeat(MAX_ID_BYTES));
    let tag = Tag::compound([
        (POT, entry(Tag::Byte(1), Tag::Byte(0))),
        (long.as_str(), entry(Tag::Byte(1), Tag::Byte(0))),
    ]);
    assert!(
        CustomBooks::read(Some(&saved(Some(tag)))).is_empty(),
        "an id too long"
    );

    let padding = Tag::ByteArray(vec![0; MAX_ENCODED_BYTES]);
    let tag = Tag::compound([
        (POT, entry(Tag::Byte(1), Tag::Byte(0))),
        ("demo:padded", Tag::compound([("pad", padding)])),
    ]);
    assert!(
        CustomBooks::read(Some(&saved(Some(tag)))).is_empty(),
        "too many bytes"
    );

    assert!(
        CustomBooks::read(Some(&saved(Some(Tag::Byte(1))))).is_empty(),
        "not a compound"
    );
}

#[test]
fn a_full_map_refuses_a_new_book_without_dropping_one() {
    let mut books = CustomBooks::default();
    for i in 0..MAX_BOOKS {
        books.set(&format!("demo:book_{i}"), OPEN).unwrap();
    }
    assert_eq!(books.set(POT, OPEN), Err(BookError::Full));
    assert_eq!(books.len(), MAX_BOOKS);
    assert_eq!(books.get(POT), BookFlags::default());
    assert_eq!(
        books.set("demo:book_0", FILTERING),
        Ok(true),
        "a saved book still changes"
    );
}

#[test]
fn a_book_id_is_a_namespaced_resource_id() {
    for id in [POT, "a:b", "my-mod.x:path/to_book", "minecraft:crafting"] {
        assert!(valid_book_id(id), "{id}");
    }
    let long = format!("demo:{}", "a".repeat(MAX_ID_BYTES));
    for id in [
        "",
        "demo",
        ":pot",
        "demo:",
        "Demo:pot",
        "demo:Pot",
        "de/mo:pot",
        "demo:pot!",
        long.as_str(),
    ] {
        assert!(!valid_book_id(id), "{id:?}");
    }
    assert_eq!(
        CustomBooks::default().set("demo", OPEN),
        Err(BookError::InvalidId("demo".into()))
    );
}

#[test]
fn the_encoded_size_counts_as_nbt_writes_it() {
    let tag = Tag::compound([(POT, entry(Tag::Byte(1), Tag::Byte(0)))]);
    let bytes = horizon_core::nbt::write_network(&tag, horizon_core::nbt::Limits::saved()).unwrap();
    // `write_network` adds the root's own type byte.
    let size = bytes.len() - 1;
    assert!(encoded_within(&tag, size));
    assert!(!encoded_within(&tag, size - 1));
}
