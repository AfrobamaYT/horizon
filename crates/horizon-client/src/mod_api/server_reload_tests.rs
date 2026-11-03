use super::*;

fn id(value: &str) -> Identifier {
    Identifier::parse(value).unwrap()
}

#[test]
fn registry_views_and_explicit_order_before_vanilla() {
    let mut listeners = ServerReloadListeners::new();
    listeners.set_owner("test_mod");
    let first = Arc::new(Boundary);
    let second = Arc::new(Boundary);
    listeners
        .add_listener(
            id("test_mod:first"),
            id("test_mod:machines/"),
            first.clone(),
        )
        .unwrap();
    listeners
        .add_listener(
            id("test_mod:second"),
            id("test_mod:machines/"),
            second.clone(),
        )
        .unwrap();
    assert_eq!(
        listeners.get_name_lookup(first.as_ref()),
        Some("test_mod:first")
    );
    assert_eq!(listeners.get_name_lookup(&Boundary), None);
    assert_eq!(
        listeners.get_name_lookup(listeners.get_last_vanilla_listener()),
        Some(VanillaServerListeners::FUNCTIONS)
    );
    assert_eq!(
        listeners
            .get_registry()
            .map(|(name, _)| name)
            .collect::<Vec<_>>(),
        [
            VanillaServerListeners::FIRST,
            VanillaServerListeners::FUNCTIONS,
            VanillaServerListeners::LAST,
            "test_mod:first",
            "test_mod:second",
        ]
    );
    listeners
        .add_dependency("test_mod:second", VanillaServerListeners::FUNCTIONS)
        .unwrap();
    assert!(
        listeners
            .get_graph()
            .any(|edge| edge == ("test_mod:second", VanillaServerListeners::FUNCTIONS))
    );
    let sorted = listeners.finish().unwrap();
    assert_eq!(
        sorted
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        [
            VanillaServerListeners::FIRST,
            "test_mod:second",
            VanillaServerListeners::FUNCTIONS,
            "test_mod:first",
            VanillaServerListeners::LAST,
        ]
    );
}

#[test]
fn failed_registration_is_not_silenced_by_ignoring_the_result() {
    let mut listeners = ServerReloadListeners::new();
    listeners.set_owner("mistaken");
    let error = listeners
        .add_dependency("missing:listener", VanillaServerListeners::FUNCTIONS)
        .unwrap_err();
    assert_eq!(
        error,
        ReloadOrderError::UnknownListener {
            id: "missing:listener".into(),
            owner: "mistaken".into()
        }
    );
    assert!(matches!(
        listeners.finish(),
        Err(ReloadOrderError::UnknownListener { .. })
    ));
}

#[test]
fn directory_prefixes_reject_traversal_but_allow_another_namespace() {
    for prefix in [
        "test_mod:/",
        "test_mod:../machines/",
        "test_mod:machines/../",
        "test_mod:machines//nested/",
        ".:machines/",
        "test_mod:machines",
    ] {
        let mut listeners = ServerReloadListeners::new();
        listeners.set_owner("test_mod");
        assert!(
            matches!(
                listeners.add_listener(id("test_mod:test"), id(prefix), Arc::new(Boundary)),
                Err(ReloadOrderError::InvalidPrefix { .. })
            ),
            "{prefix}"
        );
    }
    let mut listeners = ServerReloadListeners::new();
    listeners
        .add_listener(
            id("test_mod:test"),
            id("another_mod:machines/"),
            Arc::new(Boundary),
        )
        .unwrap();
}
