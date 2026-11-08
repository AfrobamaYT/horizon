use mod_api::RowWindow;
use mod_points::{
    locators::{Locators, Step},
    native_read::read,
};

enum Choice {
    Empty,
    One(u32),
    Named { value: String },
}

struct Pair {
    a: u32,
    b: u32,
}
enum Later {
    Both { a: u32, b: u32 },
}

#[test]
fn emitted_native_projections_compile_and_read() {
    let mut paths = Locators::default();
    let root = paths.intern(None, Step::Root(0)).unwrap();
    let request = RowWindow {
        node: root,
        start: 0,
        rows: 16,
        depth: 3,
    };
    let value = vec![1u32, 2];
    let node = include!(concat!(env!("OUT_DIR"), "/native_window_vec.rs"));
    assert_eq!(read(&node, root, &mut paths, request).unwrap().len(), 3);
    let value = Some(String::from("x"));
    let node = include!(concat!(env!("OUT_DIR"), "/native_window_option.rs"));
    assert_eq!(read(&node, root, &mut paths, request).unwrap().len(), 4);
    let value = (1u32, String::from("x"));
    let node = include!(concat!(env!("OUT_DIR"), "/native_window_tuple.rs"));
    assert_eq!(read(&node, root, &mut paths, request).unwrap().len(), 4);
    let value = std::collections::BTreeMap::from([(1u32, String::from("x"))]);
    let node = include!(concat!(env!("OUT_DIR"), "/native_window_map.rs"));
    assert_eq!(read(&node, root, &mut paths, request).unwrap().len(), 5);
    let value = Choice::One(7);
    let node = include!(concat!(env!("OUT_DIR"), "/native_window_choice.rs"));
    let rows = read(&node, root, &mut paths, request).unwrap();
    let old_variant = rows[1].0;
    let value = Choice::Empty;
    let node = include!(concat!(env!("OUT_DIR"), "/native_window_choice.rs"));
    assert!(matches!(
        read(
            &node,
            root,
            &mut paths,
            RowWindow {
                node: old_variant,
                ..request
            }
        ),
        Err(mod_api::RowError::Bounds)
    ));
    for value in [
        Choice::Empty,
        Choice::One(7),
        Choice::Named { value: "x".into() },
    ] {
        let node = include!(concat!(env!("OUT_DIR"), "/native_window_choice.rs"));
        assert!(read(&node, root, &mut paths, request).unwrap().len() >= 2);
    }
    let value = Pair { a: 1, b: 2 };
    let node = include!(concat!(env!("OUT_DIR"), "/native_window_pair.rs"));
    let by = |start| RowWindow { start, ..request };
    let rows = read(&node, root, &mut paths, by(1)).unwrap();
    assert_eq!((rows.len(), rows[1].2), (2, 1));
    assert_eq!(mod_api::wire::Reader::decode::<u32>(&rows[1].4), Ok(2));
    assert_eq!(
        read(&node, root, &mut paths, by(0)).map(|r| r.len()),
        Err(mod_api::RowError::Bounds)
    );
    assert_eq!(read(&node, root, &mut paths, by(2)).unwrap().len(), 1);
    let value = Later::Both { a: 1, b: 2 };
    let node = include!(concat!(env!("OUT_DIR"), "/native_window_later.rs"));
    let rows = read(&node, root, &mut paths, request).unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[2].2, 1);
    assert_eq!(mod_api::wire::Reader::decode::<u32>(&rows[2].4), Ok(2));
}
