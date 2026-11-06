#[test]
fn item_model_property_dispatches_borrowed_stack_and_preserves_unhandled_answers() {
    struct Property;
    #[mod_api::__subscriptions]
    impl Hooks for Property {
        fn item_model_property(&self, _: &dyn ClientHost, event: &mut ItemModelPropertyEvent<'_>) {
            if event.property == "example:charged" {
                event.value = event.stack.count > 1;
            }
        }
    }
    let mods: Vec<ModEntry> = vec![ModEntry::new("property-8", Box::new(Property))];
    let mods: Arc<[ModEntry]> = mods.into();
    install(mods.clone());
    let host = TestHost;
    let mut value = false;
    dispatch_item_model_property(
        &host,
        "example:charged",
        &StackView { count: 2 },
        &mut value,
    );
    assert!(value);
    dispatch_item_model_property(&host, "other:property", &StackView { count: 1 }, &mut value);
    assert!(value);
    dispatch_item_model_property(
        &host,
        "example:charged",
        &StackView { count: 1 },
        &mut value,
    );
    assert!(!value);
    install(Arc::from([]));
    dispatch_item_model_property(
        &host,
        "example:charged",
        &StackView { count: 2 },
        &mut value,
    );
    assert!(!value);
}
