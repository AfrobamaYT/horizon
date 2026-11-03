use super::*;
fn id(value: &str) -> Identifier { Identifier::parse(value).unwrap() }
fn files() -> FileToIdConverter { FileToIdConverter::json("machines").unwrap() }

#[test]
fn registry_views_and_explicit_order_before_vanilla() {
    let mut listeners = ServerReloadListeners::new();
    let first = Arc::new(Boundary);
    let second = Arc::new(Boundary);
    listeners.add_listener(id("test_mod:first"), files(), first.clone()).unwrap();
    listeners.add_listener(id("test_mod:second"), files(), second.clone()).unwrap();
    assert_eq!(listeners.get_name_lookup(first.as_ref()), Some(&id("test_mod:first")));
    assert_eq!(listeners.get_name_lookup(&Boundary), None);
    assert_eq!(listeners.get_name_lookup(listeners.get_last_vanilla_listener()), Some(&VanillaServerListeners::functions()));
    assert_eq!(listeners.get_registry().map(|(name, _)| name.clone()).collect::<Vec<_>>(), [
        VanillaServerListeners::first(), VanillaServerListeners::functions(), VanillaServerListeners::last(),
        id("test_mod:first"), id("test_mod:second"),
    ]);
    listeners.add_dependency(id("test_mod:second"), VanillaServerListeners::functions()).unwrap();
    assert!(listeners.get_graph().any(|edge| edge == (&id("test_mod:second"), &VanillaServerListeners::functions())));
    let sorted = listeners.finish().unwrap();
    assert_eq!(sorted.into_iter().map(|entry| entry.id).collect::<Vec<_>>(), [
        VanillaServerListeners::first(), id("test_mod:second"), VanillaServerListeners::functions(),
        id("test_mod:first"), VanillaServerListeners::last(),
    ]);
}

#[test]
fn failed_registration_is_not_silenced_by_ignoring_the_result() {
    let mut listeners = ServerReloadListeners::new();
    assert_eq!(listeners.add_dependency(id("mistyped:listener"), VanillaServerListeners::functions()).unwrap_err(),
        ReloadOrderError::UnknownListener { id: id("mistyped:listener") });
    assert!(matches!(listeners.finish(), Err(ReloadOrderError::UnknownListener { .. })));
}

#[test]
fn duplicate_objects_cannot_make_the_reverse_lookup_ambiguous() {
    let mut listeners = ServerReloadListeners::new();
    let listener = Arc::new(Boundary);
    listeners.add_listener(id("one:listener"), files(), listener.clone()).unwrap();
    assert_eq!(listeners.add_listener(id("two:listener"), files(), listener), Err(ReloadOrderError::DuplicateObject {
        id: id("two:listener"), registered_as: id("one:listener"),
    }));
}
