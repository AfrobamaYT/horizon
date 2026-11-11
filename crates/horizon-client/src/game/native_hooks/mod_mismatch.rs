use super::*;
use horizon_client::save::level_mods;

impl Game<'_> {
    /// A world opened whose saved mod list differs from the mods loaded now
    /// (NeoForge's `ModMismatchEvent`): `mismatches` names each mod the world
    /// was saved with whose version changed, or that is no longer loaded
    /// (`current_version` none). A mod loaded now that the world wasn't saved
    /// with is no mismatch, and a version unknown on either side (`""`)
    /// compares equal. Push a mod id into `resolved` to mark its mismatch
    /// handled, as `markResolved` does: each mismatch nobody resolves is
    /// logged as a warning once the hook returns. Fired once per world open,
    /// after `level.dat` is read and before any chunk loads, only when there
    /// is at least one mismatch; nothing stops the world from opening.
    /// Server-side (the single player's integrated server included).
    pub(in crate::game) fn native_mod_mismatch(
        &mut self,
        mismatches: &[mod_api::ModMismatch],
        resolved: &mut Vec<mod_api::ModMismatchResolution>,
    ) {
        self.fire_mod_mismatch(mismatches, resolved);
    }

    /// A server opened a world: its saved mod list against the loaded mods.
    /// The mods hear the differences once, and each one nobody resolved is
    /// a warning.
    pub(in crate::game) fn check_mod_mismatch(&mut self) {
        let Some(loaded) = level_mods::loaded() else {
            return;
        };
        let mismatches = self.level.mod_mismatches(&loaded);
        if self.level.saved_mods.is_none() {
            return;
        }
        let mut resolved = Vec::new();
        self.native_mod_mismatch(&mismatches, &mut resolved);
        let shown = |version: &str| -> String {
            if version.is_empty() {
                "an unknown version".to_owned()
            } else {
                version.to_owned()
            }
        };
        let mut ordered: Vec<_> = mismatches.iter().collect();
        ordered.sort_by(|a, b| a.mod_id.cmp(&b.mod_id));
        for mismatch in ordered {
            let current = mismatch
                .current_version
                .as_deref()
                .map_or_else(|| "missing".to_owned(), shown);
            if let Some(resolution) = resolved.iter().find(|r| r.mod_id == mismatch.mod_id) {
                log::debug!(
                    "Horizon: mod {} was {}, now {}; resolved by {} ({})",
                    mismatch.mod_id,
                    shown(&mismatch.saved_version),
                    current,
                    resolution.resolver,
                    if resolution.resolver == mismatch.mod_id {
                        "self"
                    } else {
                        "other mod"
                    }
                );
            } else {
                log::warn!(
                    "Horizon: mod {} was {} when the world was saved, now {}",
                    mismatch.mod_id,
                    shown(&mismatch.saved_version),
                    current
                );
            }
        }
        if !mismatches.is_empty() {
            log::warn!("Horizon: changed mods may cause the world to work incorrectly");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mod_api::{Hooks, HostSide, ModEntry, ModMismatchEvent};
    use std::sync::{Arc, Mutex};

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn existing_lists_fire_on_client_and_dedicated_even_without_differences() {
        struct Listener(Arc<Mutex<usize>>);
        #[mod_api::__subscriptions]
        impl Hooks for Listener {
            fn mod_mismatch(&self, _: HostSide, event: &mut ModMismatchEvent<'_>) {
                assert!(event.mismatches.is_empty());
                *self.0.lock().unwrap() += 1;
            }
        }
        let heard = Arc::new(Mutex::new(0));
        let old = level_mods::set_loaded(Some(vec![]));
        mod_api::testing::with_mods(
            vec![ModEntry::new("listener", Box::new(Listener(heard.clone())))].into(),
            || {
                for role in [
                    execution_role::ExecutionRole::Client,
                    execution_role::ExecutionRole::DedicatedServer,
                ] {
                    super::super::super::native_entity_refs::tests::with_game(|game| {
                        game.options.execution_role = role;
                        game.level.saved_mods = None;
                        game.check_mod_mismatch();
                        game.level.saved_mods = Some(vec![]);
                        game.check_mod_mismatch();
                    });
                }
            },
        );
        level_mods::set_loaded(old);
        assert_eq!(*heard.lock().unwrap(), 2);
    }
}
