//! The notices a side's freshly bound data gives mods, in NeoForge's order.
use super::Game;

impl Game<'_> {
    /// Items' default components are bound (`DefaultDataComponentsBoundEvent`), then the
    /// tags (`TagsUpdatedEvent`), both for `cause`.
    pub(in crate::game) fn announce_data_bound(&mut self, cause: mod_api::TagsUpdateCause) {
        self.native_default_components_bound(cause);
        self.native_tags_updated(cause);
    }
}

#[cfg(test)]
mod tests {
    use crate::game::native_entity_refs::tests::with_game;
    use mod_api::{
        DefaultComponentsBoundEvent, Hooks, HostSide, ModEntry, TagsUpdateCause, TagsUpdatedEvent,
    };
    use std::sync::{Arc, Mutex};

    struct Probe(Arc<Mutex<Vec<(&'static str, TagsUpdateCause)>>>);
    #[mod_api::__subscriptions]
    impl Hooks for Probe {
        fn default_components_bound(
            &self,
            _: HostSide<'_>,
            event: &mut DefaultComponentsBoundEvent,
        ) {
            self.0.lock().unwrap().push(("bound", event.cause));
        }
        fn tags_updated(&self, _: HostSide<'_>, event: &mut TagsUpdatedEvent) {
            self.0.lock().unwrap().push(("tags", event.cause));
        }
    }

    #[test]
    #[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES"]
    fn defaults_are_announced_bound_before_the_tags() {
        let heard = Arc::new(Mutex::new(Vec::new()));
        let mods = vec![ModEntry::new("bound_probe", Box::new(Probe(heard.clone())))].into();
        let reload = TagsUpdateCause::ServerDataLoad { reload: true };
        mod_platform::testing::with_mods(mods, || {
            with_game(|game| {
                heard.lock().unwrap().clear();
                game.announce_data_bound(reload);
            });
        });
        assert_eq!(
            *heard.lock().unwrap(),
            [("bound", reload), ("tags", reload)]
        );
    }
}
