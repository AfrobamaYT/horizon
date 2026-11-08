use mod_api::RowWindow;
use mod_points::{
    locators::{Locators, Step},
    native_read::read,
};

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
}
