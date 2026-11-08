#[test]
fn state_factory_ownership_preserves_listen_order_and_existing_answers() {
    struct Factory(u32, Arc<Mutex<Vec<u32>>>);
    #[mod_api::__subscriptions]
    impl Hooks for Factory {
        fn stateful_block_entity(&self, _: &dyn Host, event: &mut StatefulBlockEntityEvent<'_>) {
            assert_eq!(event.block, "fixture:heater");
            self.1.lock().unwrap().push(self.0);
            if self.0 == 2 {
                event.stateful = true;
            }
        }
    }
    let calls = Arc::new(Mutex::new(Vec::new()));
    install(
        (1..=3)
            .map(|id| ModEntry::new("factory", Box::new(Factory(id, calls.clone()))))
            .collect::<Vec<_>>()
            .into(),
    );
    let mut stateful = false;
    dispatch_stateful_block_entity(&TestHost, "fixture:heater", &mut stateful);
    assert!(stateful);
    assert_eq!(*calls.lock().unwrap(), [1, 2, 3]);
    install(Vec::<ModEntry>::new().into());
    dispatch_stateful_block_entity(&TestHost, "fixture:heater", &mut stateful);
    assert!(stateful, "no subscribers must not clear ownership");
}

#[test]
fn actual_state_factory_preserves_none_and_stops_at_first_answer() {
    struct Factory(u32, Arc<Mutex<Vec<u32>>>);
    #[mod_api::__subscriptions]
    impl Hooks for Factory {
        fn block_entity(&self, _: LevelSide, event: &mut BlockEntityEvent<'_>) -> Option<Compound> {
            assert_eq!(event.block, "fixture:heater");
            let _actual_state = event.state;
            self.1.lock().unwrap().push(self.0);
            (self.0 == 2).then_some(Compound)
        }
    }
    let calls = Arc::new(Mutex::new(Vec::new()));
    install(vec![ModEntry::new("none", Box::new(Factory(1, calls.clone())))].into());
    let mut entity = None;
    dispatch_block_entity(
        LevelSide::Server(&TestLevel),
        "fixture:heater",
        BlockStateId,
        &mut entity,
    );
    assert!(entity.is_none());
    assert_eq!(*calls.lock().unwrap(), [1]);
    calls.lock().unwrap().clear();
    install(
        (1..=3)
            .map(|id| ModEntry::new("factory", Box::new(Factory(id, calls.clone()))))
            .collect::<Vec<_>>()
            .into(),
    );
    dispatch_block_entity(
        LevelSide::Server(&TestLevel),
        "fixture:heater",
        BlockStateId,
        &mut entity,
    );
    assert!(entity.is_some());
    assert_eq!(
        *calls.lock().unwrap(),
        [1, 2],
        "later factories must not run"
    );
}
