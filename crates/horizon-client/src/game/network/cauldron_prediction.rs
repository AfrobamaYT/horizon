//! Shared cauldron client selection. Prediction never invokes authoritative callbacks.
use super::remote::DryUse;
use super::*;
use crate::game::api_values::TryIntoApi;
use horizon_client::cauldron::{self, Outcome, interactions::Entry};
use horizon_world::level::InteractionResult;

struct DryHand<'a> {
    creative: bool,
    hand: Option<ItemStack>,
    inventory: &'a [Option<ItemStack>],
}
impl cauldron::Player for DryHand<'_> {
    fn creative(&self) -> bool {
        self.creative
    }
    fn hand(&mut self) -> &mut Option<ItemStack> {
        &mut self.hand
    }
    fn add(&mut self, _: &mut ItemStack) -> bool {
        true
    }
    fn contains(&self, stack: &ItemStack) -> bool {
        self.inventory
            .iter()
            .flatten()
            .any(|have| have.same_item_same_components(stack))
    }
    fn drop_stack(&mut self, _: ItemStack) {}
}
impl Game<'_> {
    /// Both prediction routes use this snapshot and one selected callback. Custom decisions
    /// predict consumption only; authoritative block updates provide their visual correction.
    pub(super) fn client_cauldron_prediction(
        &self,
        pos: [i32; 3],
        held: Option<&ItemStack>,
    ) -> (Outcome, DryUse) {
        let pass = || (Outcome::Pass, DryUse::default());
        let Some(held) = held else {
            return pass();
        };
        if self.keys.held(Mapping::Sneak) {
            return pass();
        }
        let Some((state, underwater)) = self.cauldron_at(pos) else {
            return pass();
        };
        let table = cauldron_interactions::snapshot();
        let removes_dye = cauldron::removes_dye(self.items(), &held.item);
        let entry = table.select(&state.name, &held.item, removes_dye);
        if let Some(Entry::Mod(callback)) = &entry {
            let Some(_active) = cauldron_interactions::enter_decision() else {
                return (Outcome::Consume, DryUse::default());
            };
            let decision = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let index = api_values::block_state::current().ok()?;
                let epoch = self.native_lifecycle.epoch()?;
                let remote = self.remote.as_ref().filter(|remote| remote.logged_in)?;
                let actor = mod_api::EntityRef::new(epoch, u32::try_from(remote.entity_id).ok()?);
                let input = mod_api::CauldronUse {
                    dimension: mod_api::Identifier::parse(self.dim.id()).ok()?,
                    pos,
                    state: index
                        .to_api(self.world.block(pos[0], pos[1], pos[2]))
                        .ok()?,
                    actor,
                    hand: match self.hand {
                        Hand::Main => mod_api::Hand::Main,
                        Hand::Off => mod_api::Hand::Off,
                    },
                    held: held.try_into_api().ok()?,
                    underwater,
                    creative: self.creative,
                };
                let _owner = mod_api::__private::enter(callback.owner, "cauldron_prediction");
                let host: &dyn mod_api::ClientHost = self;
                let decision = callback.handler.decide(host, &input);
                if !Arc::ptr_eq(&table, &cauldron_interactions::snapshot())
                    || !api_values::block_state::current()
                        .is_ok_and(|now| Arc::ptr_eq(&index, &now))
                {
                    return None;
                }
                Some(decision)
            }));
            let outcome = match decision {
                Ok(Some(mod_api::CauldronDecision::Pass)) => Outcome::Pass,
                Ok(Some(mod_api::CauldronDecision::Success(_))) => Outcome::Success,
                _ => Outcome::Consume,
            };
            return (outcome, DryUse::default());
        }
        let mut hand = DryHand {
            creative: self.creative,
            hand: Some(held.clone()),
            inventory: &self.inventory,
        };
        let result = cauldron::interact_selected(&state, underwater, entry.as_ref(), &mut hand);
        let at = horizon_core::position::BlockPos::new(pos[0], pos[1], pos[2]);
        let changes = result
            .block
            .and_then(|block| self.pack.assets.registry.resolve(&block).ok())
            .map(|block| (at, block))
            .into_iter()
            .collect();
        let sounds = result
            .sound
            .map(|sound| (at, Arc::from(sound), 1.0, 1.0))
            .into_iter()
            .collect();
        (result.outcome, (changes, sounds))
    }
    /// The visual prediction route uses the same decision as the item-use consumption guard.
    pub(super) fn remote_cauldron_reacts(
        &self,
        hit: &Hit,
        held: Option<&ItemStack>,
    ) -> Option<(InteractionResult, DryUse)> {
        let (outcome, changes) = self.client_cauldron_prediction(hit.pos, held);
        let answer = match outcome {
            Outcome::Pass => return None,
            Outcome::Consume => InteractionResult::Consume,
            Outcome::Success => InteractionResult::Success,
        };
        Some((answer, changes))
    }
}
