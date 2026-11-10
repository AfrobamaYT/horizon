use super::*;
use crate::item::crafting::Shape;
use crate::world::snapshots::LootEntity;
use crate::{Ingredient, ItemPlace, RecipeSlotDisplay, Tag, Text};

fn round<T: Wire>(value: &T, initial: u16) {
    let mut out = Writer::new();
    out.put(value);
    assert_eq!(out.error(), None);
    let mut reader = Reader::new(out.bytes());
    reader.depth = initial;
    assert!(reader.take::<T>().is_ok());
    let mut reader = Reader::new(out.bytes());
    reader.depth = initial + 1;
    assert!(matches!(reader.take::<T>(), Err(WireError::Depth)));
}

#[test]
fn structural_families_fit_the_guest_shadow_stack() {
    // The guest reserves 256 KiB; retain half for callers and transport plumbing.
    std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            // A raised limit is not safe merely because the depth counter still works.
            assert_eq!(MAX_DEPTH, 32);
            let mut ingredient = Ingredient::of(vec!["minecraft:stone".into()]);
            let mut display = RecipeSlotDisplay::Empty;
            for _ in 1..32 {
                let mut parent = Ingredient::of(vec!["minecraft:stone".into()]);
                parent.children = Some((Shape::Compound, vec![ingredient]));
                ingredient = parent;
                display = RecipeSlotDisplay::Composite(vec![display]);
            }
            round(&ingredient, 0);
            round(&display, 0);
            let mut place = ItemPlace::ItemEntity { id: 1 };
            let mut entity = LootEntity::default();
            for _ in 0..15 {
                place = ItemPlace::Nested {
                    parent: Box::new(place),
                    slot: 0,
                };
                entity = LootEntity {
                    vehicle: Some(Box::new(entity)),
                    ..Default::default()
                };
            }
            round(&Box::new(place), 0);
            round(&Box::new(entity), 0);
            // These public SCC members no longer recurse structurally: every Text
            // edge crosses through a flat table. Their own envelope still guards depth.
            round(&crate::text::TextContent::Literal(String::new()), 31);
            round(&crate::text::TextStyle::default(), 31);
            round(
                &crate::text::TextHover::Item {
                    id: String::new(),
                    count: 1,
                    components: None,
                },
                31,
            );
            round(
                &crate::text::TextObject::Atlas {
                    atlas: String::new(),
                    sprite: String::new(),
                    fallback: None,
                },
                31,
            );
            let mut parent = Ingredient::of(vec![]);
            parent.children = Some((Shape::Compound, vec![ingredient]));
            let mut out = Writer::new();
            out.put(&parent);
            assert_eq!(out.error(), Some(WireError::Depth));
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn vanilla_trees_keep_their_depth_on_the_guest_shadow_stack() {
    std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            for compound in [false, true] {
                let mut tag = Tag::Int(7);
                for _ in 0..512 {
                    tag = if compound {
                        Tag::compound([("k", tag)])
                    } else {
                        Tag::List(vec![tag])
                    };
                }
                let mut out = Writer::new();
                out.put(&tag);
                assert_eq!(out.error(), None);
                let read = Reader::decode::<Tag>(out.bytes()).unwrap();
                horizon_core::nbt::discard_tag(read);
                let mut bytes = if compound {
                    vec![10, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 107, 0]
                } else {
                    vec![9, 0, 0, 0, 1, 0, 0, 0]
                };
                bytes.extend_from_slice(out.bytes());
                assert!(matches!(
                    Reader::decode::<Tag>(&bytes),
                    Err(WireError::Depth)
                ));
                horizon_core::nbt::discard_tag(tag);
            }
            let mut text = Text::literal("leaf");
            for _ in 1..512 {
                text = Text::literal("").append(text);
            }
            let mut out = Writer::new();
            out.put(&text);
            assert_eq!(out.error(), None);
            let mut form = crate::text::TextForm::try_from(&text).unwrap();
            let mut extra =
                crate::text::TextNode::new(crate::text::TextNodeContent::Literal(String::new()));
            extra.parent = Some(511);
            form.nodes.push(extra);
            let mut invalid = Writer::new();
            invalid.put(&form);
            assert!(matches!(
                Reader::decode::<Text>(invalid.bytes()),
                Err(WireError::Invalid(_))
            ));
            // Drop is outside Wire's ownership; intentionally discard a deep tree iteratively.
            let decoded = Reader::decode::<Text>(out.bytes()).unwrap();
            let text = Text::literal("").append(text);
            let mut too_deep = Writer::new();
            too_deep.put(&text);
            assert_eq!(too_deep.error(), Some(WireError::Depth));
            let mut pending = vec![text, decoded];
            while let Some(mut text) = pending.pop() {
                pending.append(&mut text.children);
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
