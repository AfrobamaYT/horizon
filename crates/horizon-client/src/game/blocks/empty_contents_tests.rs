//! `ItemPart::empty_contents` on a dispenser: an item with the Part is emptied as a bucket.
use horizon_client::items::ItemStack;
use horizon_core::geometry::Direction;
use mod_api::{EmptyContentsEvent, Hooks, ItemPart, Registrar, RegistrationContext, ServerHost};
use std::sync::{Arc, Mutex};

/// What the Part answers, and whether each ask named a player or a clicked block.
#[derive(Clone, Default)]
struct Jar(Arc<Mutex<(Option<bool>, Vec<(bool, bool)>)>>);

#[mod_api::__subscriptions]
impl Hooks for Jar {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        let key = r.item_key("minecraft:stick");
        r.item_part(key, self.clone());
    }
}
#[mod_api::horizon_part]
impl ItemPart for Jar {
    fn empty_contents(
        &self,
        _: &mut dyn ServerHost,
        event: &mut EmptyContentsEvent<'_>,
    ) -> Option<bool> {
        let mut jar = self.0.lock().unwrap();
        jar.1.push((event.player.is_some(), event.hit.is_some()));
        jar.0
    }
}

/// A dispenser asks the Part of an item that is no bucket: `Some(true)` leaves an empty bucket in
/// the slot, `Some(false)` and `None` (no contents vanilla knows) dispense the item itself.
#[test]
#[ignore = "needs prepared resources in HORIZON_TEST_RESOURCES; process-wide parts, run this test alone"]
fn a_part_empties_its_item_from_a_dispenser() {
    use horizon_client::chunk::Chunk;
    let jar = Jar::default();
    let mods: mod_api::Mods = vec![mod_api::ModEntry::new("jar", Box::new(jar.clone()))].into();
    crate::game::api_values::install_parts(&mods).unwrap();
    mod_platform::testing::with_mods(mods, || {
        crate::game::native_entity_refs::tests::with_game(|game| {
            game.options.dedicated = Some(Vec::new());
            game.world
                .chunks
                .insert((0, 0), Arc::new(Chunk::empty(0, 0)));
            let dispense = |game: &mut super::Game<'_>| {
                game.dispense_item(
                    (5, 64, 5),
                    Direction::East,
                    ItemStack::new("minecraft:stick", 1),
                )
                .unwrap()
            };

            jar.0.lock().unwrap().0 = Some(true);
            let left = dispense(game).map(|stack| stack.item.to_string());
            assert_eq!(left.as_deref(), Some("minecraft:bucket"));
            assert!(game.dropped.is_empty());

            for answer in [Some(false), None] {
                jar.0.lock().unwrap().0 = answer;
                let dropped = game.dropped.len();
                assert_eq!(dispense(game), None);
                assert_eq!(game.dropped.len(), dropped + 1);
            }
            assert_eq!(jar.0.lock().unwrap().1, [(false, false); 3]);
        });
    });
}
