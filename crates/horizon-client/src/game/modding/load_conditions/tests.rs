use mod_api::__private::game::RegistrationContextNew;
use mod_api::{
    ConditionError, Hooks, Identifier, LoadCondition, ModEntry, RegisterLoadConditionsEvent,
    RegistrationContext,
};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
struct Flag(Arc<AtomicUsize>);
impl LoadCondition for Flag {
    fn test(&self, args: &Value) -> std::result::Result<bool, ConditionError> {
        assert!(args.get("type").is_none());
        self.0.fetch_add(1, Ordering::SeqCst);
        args["value"].as_bool().ok_or_else(|| ConditionError {
            message: "value must be a boolean".into(),
        })
    }
}
struct Registrant {
    count: Arc<AtomicUsize>,
    mode: Arc<AtomicUsize>,
}
#[mod_api::__subscriptions]
impl Hooks for Registrant {
    fn register_load_conditions(
        &self,
        _: &RegistrationContext,
        event: &mut RegisterLoadConditionsEvent<'_>,
    ) {
        event
            .conditions
            .register(
                Identifier::parse("testmod:flag").unwrap(),
                Flag(self.count.clone()),
            )
            .unwrap();
    }
    fn data_pack_files(
        &self,
        _: &mut dyn mod_api::ServerHost,
        event: &mut mod_api::DataPackFilesEvent<'_>,
    ) {
        let yes = match self.mode.load(Ordering::SeqCst) {
            1 => json!({"type":"testmod:nope"}),
            2 => json!({"type":"testmod:flag","value":"bad"}),
            _ => json!({"type":"testmod:flag","value":true}),
        };
        for (name, condition) in [
            ("enabled", yes),
            ("disabled", json!({"type":"testmod:flag","value":false})),
            (
                "nested",
                json!({"type":"neoforge:and","values":[{"type":"neoforge:true"},{"type":"neoforge:or","values":[{"type":"testmod:flag","value":true}]}]}),
            ),
        ] {
            event.files.push((
                format!("recipe/testmod:{name}"),
                serde_json::to_vec(&json!({
                    "type":"minecraft:crafting_shapeless", "ingredients":["minecraft:flint"],
                    "result":{"id":"minecraft:stick"}, "neoforge:conditions":[condition]
                }))
                .unwrap(),
            ));
        }
    }
}
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
fn load_conditions_recipe_reload_counts_and_failed_candidates_keep_old_recipes() {
    let scratch = tempfile::tempdir().unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let mode = Arc::new(AtomicUsize::new(0));
    mod_api::testing::with_mods(
        vec![ModEntry::new(
            "testmod",
            Box::new(Registrant {
                count: count.clone(),
                mode: mode.clone(),
            }),
        )]
        .into(),
        || {
            crate::game::native_entity_refs::tests::with_game(|game| {
                let mut request = crate::game::world_loading::WorldRequest::new(
                    "conditions",
                    Some("test"),
                    Some(1),
                    true,
                    None,
                    Arc::new(horizon_client::gamerules::RuleRegistry::vanilla()),
                )
                .unwrap();
                request.save_root = scratch.path().join("save");
                game.load_world(request).unwrap();
                assert!(game.recipes().recipe("testmod:enabled").is_some());
                assert!(game.recipes().recipe("testmod:disabled").is_none());
                assert!(game.recipes().recipe("testmod:nested").is_some());
                for _ in 0..2 {
                    count.store(0, Ordering::SeqCst);
                    game.reload_datapacks(true, &game.level.enabled_features.clone())
                        .unwrap();
                    assert_eq!(count.load(Ordering::SeqCst), 3);
                }
                let old: Vec<_> = game
                    .recipes()
                    .crafting
                    .iter()
                    .map(|(id, _)| id.clone())
                    .collect();
                for (bad, expected) in [(1, "testmod:nope"), (2, "value must be a boolean")] {
                    mode.store(bad, Ordering::SeqCst);
                    let error = format!(
                        "{:?}",
                        game.reload_datapacks(true, &game.level.enabled_features.clone())
                            .err()
                            .unwrap()
                    );
                    assert!(
                        error.contains("data/testmod/recipe/enabled.json"),
                        "{error}"
                    );
                    assert!(error.contains(expected), "{error}");
                    if bad == 1 {
                        assert!(error.contains("mod testmod"), "{error}");
                    }
                    assert_eq!(
                        game.recipes()
                            .crafting
                            .iter()
                            .map(|(id, _)| id.clone())
                            .collect::<Vec<_>>(),
                        old
                    );
                    assert!(game.recipes().recipe("testmod:enabled").is_some());
                }
            });
        },
    );
}
struct Duplicate;
#[mod_api::__subscriptions]
impl Hooks for Duplicate {
    fn register_load_conditions(
        &self,
        _: &RegistrationContext,
        event: &mut RegisterLoadConditionsEvent<'_>,
    ) {
        let _ = event.conditions.register(
            Identifier::parse("testmod:flag").unwrap(),
            Flag(Arc::default()),
        );
    }
}
#[test]
fn load_conditions_registration_refuses_duplicates_and_foreign_namespace_even_if_ignored() {
    for owners in [vec!["testmod", "intruder"], vec!["intruder"]] {
        let entries: Vec<_> = owners
            .iter()
            .map(|owner| ModEntry::new(owner, Box::new(Duplicate)))
            .collect();
        mod_api::testing::with_mods(entries.into(), || {
            let mut registrar = mod_api::LoadConditionDeclarations::default();
            mod_api::dispatch_register_load_conditions(&RegistrationContext::new(), &mut registrar);
            let error = registrar.finish().err().unwrap().message;
            assert!(error.contains("testmod:flag"), "{error}");
            assert!(error.contains("intruder"), "{error}");
            if owners.len() == 2 {
                assert!(error.contains("both testmod and intruder"), "{error}");
            } else {
                assert!(error.contains("outside its namespace"), "{error}");
            }
        });
    }
}
