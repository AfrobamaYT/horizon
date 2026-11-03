use super::*;
fn id(value: &str) -> Identifier {
    Identifier::parse(value).unwrap()
}
fn files() -> FileToIdConverter {
    FileToIdConverter::json("machines").unwrap()
}

#[test]
fn registry_views_and_explicit_order_before_vanilla() {
    let mut listeners = ServerReloadListeners::new();
    let first = Arc::new(Boundary);
    let second = Arc::new(Boundary);
    listeners
        .add_listener(id("test_mod:first"), files(), first.clone())
        .unwrap();
    listeners
        .add_listener(id("test_mod:second"), files(), second.clone())
        .unwrap();
    assert_eq!(
        listeners.get_name_lookup(first.as_ref()),
        Some(&id("test_mod:first"))
    );
    assert_eq!(listeners.get_name_lookup(&Boundary), None);
    assert_eq!(
        listeners.get_name_lookup(listeners.get_last_vanilla_listener()),
        Some(&VanillaServerListeners::functions())
    );
    assert_eq!(
        listeners
            .get_registry()
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>(),
        [
            VanillaServerListeners::first(),
            VanillaServerListeners::functions(),
            VanillaServerListeners::last(),
            id("test_mod:first"),
            id("test_mod:second"),
        ]
    );
    listeners
        .add_dependency(id("test_mod:second"), VanillaServerListeners::functions())
        .unwrap();
    assert!(
        listeners
            .get_graph()
            .any(|edge| edge == (&id("test_mod:second"), &VanillaServerListeners::functions()))
    );
    let sorted = listeners.finish().unwrap();
    assert_eq!(
        sorted.into_iter().map(|entry| entry.id).collect::<Vec<_>>(),
        [
            VanillaServerListeners::first(),
            id("test_mod:second"),
            VanillaServerListeners::functions(),
            id("test_mod:first"),
            VanillaServerListeners::last(),
        ]
    );
}

#[test]
fn failed_registration_is_not_silenced_by_ignoring_the_result() {
    let mut listeners = ServerReloadListeners::new();
    assert_eq!(
        listeners
            .add_dependency(id("mistyped:listener"), VanillaServerListeners::functions())
            .unwrap_err(),
        ReloadOrderError::UnknownListener {
            id: id("mistyped:listener")
        }
    );
    assert!(matches!(
        listeners.finish(),
        Err(ReloadOrderError::UnknownListener { .. })
    ));
}

#[test]
fn duplicate_objects_cannot_make_the_reverse_lookup_ambiguous() {
    let mut listeners = ServerReloadListeners::new();
    let listener = Arc::new(Boundary);
    listeners
        .add_listener(id("one:listener"), files(), listener.clone())
        .unwrap();
    assert_eq!(
        listeners.add_listener(id("two:listener"), files(), listener),
        Err(ReloadOrderError::DuplicateObject {
            id: id("two:listener"),
            registered_as: id("one:listener"),
        })
    );
}

#[test]
fn add_listener_attributes_its_calls_to_the_registering_mod() {
    struct Noop;
    impl ServerReloadListener for Noop {
        fn prepare(&self, _: &ServerReloadContext) -> Result<ServerReloadApply, ServerReloadError> {
            Ok(ServerReloadApply::new(|| {}))
        }
    }
    let guard = crate::crash::enter_native_mod(crate::crash::NativeModCall {
        id: "test_mod",
        hook: "add_server_reload_listeners",
        part: None,
    });
    let mut owned = ServerReloadListeners::new();
    owned
        .add_listener(id("test_mod:data"), files(), Arc::new(Noop))
        .unwrap();
    drop(guard);
    let mut unowned = ServerReloadListeners::new();
    unowned
        .add_listener(id("game:data"), files(), Arc::new(Noop))
        .unwrap();
    let listener = owned
        .finish()
        .unwrap()
        .into_iter()
        .find(|listener| listener.id == id("test_mod:data"))
        .unwrap();
    assert_eq!(listener.owner, Some("test_mod"));
    let listener = unowned
        .finish()
        .unwrap()
        .into_iter()
        .find(|listener| listener.id == id("game:data"))
        .unwrap();
    assert_eq!(listener.owner, None);
}
