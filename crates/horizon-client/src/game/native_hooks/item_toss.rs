use super::*;

/// The hook points. Each does nothing in Horizon.
impl Game<'_> {
    /// `player` tosses the item entity `item` (`ItemTossEvent`, from
    /// `Player.drop`: the drop key, a menu's throw, what no longer fits),
    /// not what scatters when they die. The entity is made but not yet in
    /// the level: the `item_entity_*` methods read and change it (its stack,
    /// pickup delay, target), and what they change stays. Set `canceled` and
    /// it never joins the level; the items are already out of the inventory
    /// and are lost. Server-side.
    pub(in crate::game) fn native_item_toss(
        &mut self,
        player: u128,
        item: ItemEntityRef,
        canceled: &mut bool,
    ) {
        self.fire_item_toss(player, item, canceled);
    }

    /// `Player.drop`'s item entity for `thrown`, at `at` with motion `v`
    /// and `thrower`: made, heard by `item_toss`, then added to the level
    /// (`ServerLevel.addFreshEntity`). Its id, or None when a mod canceled
    /// the toss or kept it out of the level.
    pub(in crate::game) fn toss_item(
        &mut self,
        thrown: ItemStack,
        at: [f64; 3],
        v: [f64; 3],
        thrower: Option<[i32; 4]>,
    ) -> Option<u32> {
        self.push_item_entity(thrown, at, Some(v), 40, None);
        let id = self.next_entity;
        if let Some(item) = self.dropped.last_mut() {
            item.thrower = thrower;
        }
        if mod_api::__private::listened(mod_api::HookId::ItemToss)
            && let Some(item) = self.native_item_handle(id)
        {
            let player = self.native_acting_id();
            let mut canceled = false;
            self.native_item_toss(player, item, &mut canceled);
            if canceled {
                // A canceled toss never enters the world; what was thrown is gone.
                if let Some(at) = self.dropped.iter().rposition(|d| d.id == id) {
                    self.remove_dropped(at);
                }
                self.forget_entity(id);
                return None;
            }
        }
        self.join_item_entity(id, at);
        self.dropped.iter().rev().any(|d| d.id == id).then_some(id)
    }
}

#[cfg(test)]
mod item_toss_tests {
    use super::*;
    use mod_api::{Flow, Hooks, ItemTossEvent, ModEntry, ServerHost};
    use mod_platform::testing::with_mods;
    use std::sync::{Arc, Mutex};

    /// Counts the tosses it hears. A thrown dirt block loses its pickup
    /// delay through the handle; anything else is canceled.
    struct Tosser(Arc<Mutex<u32>>);
    #[mod_api::__subscriptions]
    impl Hooks for Tosser {
        fn item_toss(&self, host: &mut dyn ServerHost, event: &mut ItemTossEvent) -> Flow {
            *self.0.lock().unwrap() += 1;
            let stack = event.item.stack(host).expect("the tossed item reads");
            if mod_api::__private::item_key_id(stack.item())
                .is_some_and(|id| id == "minecraft:dirt")
            {
                assert!(event.item.set_no_pickup_delay(host));
                Flow::Continue
            } else {
                Flow::Cancel
            }
        }
    }

    /// A toss asks `item_toss` with the item entity before it joins: what
    /// the listener changes stays, and a cancel leaves no item entity. The
    /// random scatter of a death's drops is not a toss and is never asked.
    #[test]
    #[ignore = "needs prepared resources"]
    fn item_toss_hears_the_item_entity_and_a_cancel_leaves_none() {
        let heard = Arc::new(Mutex::new(0));
        let mods = vec![ModEntry::new("toss_probe", Box::new(Tosser(heard.clone())))];
        with_mods(mods.into(), || {
            crate::game::native_entity_refs::tests::with_game(|game| {
                game.options.dedicated = Some(Vec::new());
                let eye = [0.5, 65.0, 0.5];
                let before = game.dropped.len();
                let stone = ItemStack::new("minecraft:stone", 1);
                assert!(game.throw_from_eyes(stone, eye, false, true).is_none());
                assert_eq!(*heard.lock().unwrap(), 1);
                assert_eq!(game.dropped.len(), before, "a canceled toss leaves no item");
                let dirt = ItemStack::new("minecraft:dirt", 1);
                let id = game.throw_from_eyes(dirt, eye, false, true).unwrap();
                assert_eq!(*heard.lock().unwrap(), 2);
                let thrown = game.dropped.iter().find(|d| d.id == id).unwrap();
                assert_eq!(thrown.pickup_delay, 0, "the listener's change stays");
                let dirt = ItemStack::new("minecraft:dirt", 1);
                assert!(game.throw_from_eyes(dirt, eye, true, false).is_some());
                assert_eq!(*heard.lock().unwrap(), 2);
            });
        });
    }
}
