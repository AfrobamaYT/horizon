//! A dispenser emptying a filled bucket, or a mod container item whose `empty_contents` Part
//! empties it.
use super::Game;
use super::dispenser::{CLICK, Dispensed, Pos, offset};
use horizon_client::{Result, items::ItemStack};
use horizon_core::geometry::Direction;
use horizon_world::{fluid::FluidKind, level::BucketContent};

impl Game<'_> {
    /// Whether the dispenser empties `item` as a filled bucket: a bucket of a
    /// mod fluid, or an item with an `empty_contents` Part.
    pub(super) fn dispenses_as_filled_bucket(&self, item: &str) -> bool {
        horizon_world::fluid::mod_fluid_of_bucket(item).is_some()
            || self.empty_contents_listens(item)
    }

    /// The filled bucket behaviour: `DispensibleContainerItem.emptyContents`,
    /// asking the item's `empty_contents` Part first. Emptied, the slot gets
    /// the empty bucket; not, the stack is dispensed as an item.
    pub(super) fn dispense_filled_bucket(
        &mut self,
        pos: Pos,
        facing: Direction,
        stack: ItemStack,
    ) -> Result<Dispensed> {
        let target = offset(pos, facing);
        let emptied = match self.native_empty_contents_event(Self::pos_of(target), None, &stack) {
            Some(emptied) => emptied,
            None => self.empty_bucket_contents(target, &stack)?,
        };
        if !emptied {
            return Ok((self.dispense_default_in_level(pos, facing, stack), CLICK));
        }
        let bucket = ItemStack::new("minecraft:bucket", 1);
        Ok((
            self.consume_with_remainder(pos, facing, stack, bucket),
            CLICK,
        ))
    }

    /// Vanilla's emptying of `stack` into `target`: the fluid or block, the
    /// evaporation smoke and the bucket's mob. False when nothing was placed,
    /// or `stack` holds no contents vanilla knows.
    fn empty_bucket_contents(&mut self, target: Pos, stack: &ItemStack) -> Result<bool> {
        let Some((content, sound, mob)) = horizon_client::items::bucket_contents(&stack.item)
        else {
            return Ok(false);
        };
        let target_pos = Self::pos_of(target);
        let emptied = self.with_level(|b, level, ticks| {
            b.bucket_empty(level, ticks, target_pos, None, content, sound, false, false)
        });
        self.process_level_events()?;
        if emptied.is_none() {
            return Ok(false);
        }
        if self.dimension_type().water_evaporates
            && matches!(
                content,
                BucketContent::Fluid(FluidKind::Water) | BucketContent::MobWater
            )
        {
            self.evaporation_smoke([target.0, target.1, target.2]);
        }
        if let Some(kind) = mob {
            // MobBucketItem.checkExtraContent: the mob from the bucket calls
            // out, then ENTITY_PLACE.
            if let Some(id) = self.spawn_mob_at(kind, target, true) {
                self.mob_placed(id, stack)?;
            }
            self.post_block_game_event(
                "minecraft:entity_place",
                [target.0, target.1, target.2],
                None,
                None,
            );
        }
        Ok(true)
    }
}
