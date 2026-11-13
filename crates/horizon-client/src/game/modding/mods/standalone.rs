//! Client-only standalone model registration.
use super::{ExecutionRole, Game, RegistrationContext};

pub(super) fn collect_standalone_models(role: ExecutionRole) -> Option<Vec<String>> {
    if !role.is_client() {
        return None;
    }
    let mut names = Vec::new();
    Game::native_standalone_models(&RegistrationContext::new(), &mut names);
    Some(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standalone_models_collect_only_on_the_client() {
        use mod_api::{self, Hooks, ModEntry, StandaloneModelsEvent};
        use mod_platform::testing::with_mods;
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        #[derive(Default)]
        struct Models(Arc<AtomicUsize>);
        #[mod_api::horizon_mod]
        impl Hooks for Models {
            fn standalone_models(
                &self,
                context: &RegistrationContext,
                event: &mut StandaloneModelsEvent<'_>,
            ) {
                assert_eq!(
                    context.mods().ids().collect::<Vec<_>>(),
                    ["registration_test"]
                );
                self.0.fetch_add(1, Ordering::Relaxed);
                event.models.push("registration_test:block/shaft".into());
            }
        }
        let calls = Arc::new(AtomicUsize::new(0));
        with_mods(
            vec![ModEntry::new(
                "registration_test",
                Box::new(Models(calls.clone())),
            )]
            .into(),
            || {
                mod_platform::__private::with_test_packages(&["registration_test"], || {
                    assert_eq!(
                        collect_standalone_models(ExecutionRole::IntegratedServer),
                        None
                    );
                    assert_eq!(
                        collect_standalone_models(ExecutionRole::DedicatedServer),
                        None
                    );
                    assert_eq!(calls.load(Ordering::Relaxed), 0);
                    let models = collect_standalone_models(ExecutionRole::Client).unwrap();
                    assert_eq!(calls.load(Ordering::Relaxed), 1);
                    assert_eq!(models, ["registration_test:block/shaft"]);
                })
            },
        );
    }
}
