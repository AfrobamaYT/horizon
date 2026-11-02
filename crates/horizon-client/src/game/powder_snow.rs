//! Powder snow around the player: its snowflakes, fire put out by it, and the
//! bucket that picks it up (`BucketItem.use` on a `BucketPickup` block). The
//! sinking, slowing, climbing and freezing are `PlayerMotion`'s
//! (`crate::motion`); placing it is the powder snow bucket's `BlockItem.place`.
use super::Game;
use horizon_client::{Result, items::ItemStack, mobs::Who};
use horizon_core::{position::BlockPos, random::RandomSource};

impl Game<'_> {
    /// Entity.playEntityOnFireExtinguishedSound
    pub(super) fn extinguish_sound(&mut self, at: [f64; 3]) {
        let r = self.particles.random();
        let pitch = 1.6 + (r.next_float() - r.next_float()) * 0.4;
        self.audio.play(
            "minecraft:entity.generic.extinguish_fire",
            Some(at),
            0.7,
            pitch,
        );
    }
    /// `Entity.applyEffectsFromBlocks`: a burning player in the rain
    /// (`isInRain`) has the fire put out (`clearFire`), with the extinguish
    /// sound that everyone near hears.
    pub(super) fn rain_extinguish_player(&mut self) {
        let Some(player) = self.player.as_ref() else {
            return;
        };
        let (at, top) = (player.position, player.bounds().max_position().y);
        if player.fire_ticks <= 0 || !self.in_rain(at, top) {
            return;
        }
        if let Some(player) = self.player.as_mut() {
            player.fire_ticks = 0;
        }
        self.extinguish_sound([at.x, at.y, at.z]);
    }
    /// Whether the block at `pos` is powder snow.
    pub(super) fn is_powder_snow(&self, pos: [i32; 3]) -> bool {
        &*self
            .res
            .facts()
            .get(self.world.block(pos[0], pos[1], pos[2]))
            .name
            == "minecraft:powder_snow"
    }
    /// `BucketItem.use` with an empty bucket on powder snow:
    /// `PowderSnowBlock.pickupBlock` (air, level event 2001), the used
    /// statistic, the fill sound, FLUID_PICKUP, `ItemUtils.createFilledResult`
    /// and the filled_bucket trigger.
    pub(super) fn pick_up_powder_snow(&mut self, pos: [i32; 3]) -> Result<()> {
        let at = BlockPos::new(pos[0], pos[1], pos[2]);
        let creative = self.creative;
        let picked = self.with_level(|b, level, ticks| b.bucket_pickup(level, ticks, at, creative));
        self.process_level_events()?;
        let Some(filled) = picked else {
            return Ok(());
        };
        self.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
        self.award_stat("used", "minecraft:bucket", 1);
        if let Some(eye) = self
            .player
            .as_ref()
            .map(|p| [p.position.x, p.position.y, p.position.z])
        {
            self.audio.play(
                "minecraft:item.bucket.fill_powder_snow",
                Some(eye),
                1.0,
                1.0,
            );
        }
        self.post_block_game_event("minecraft:fluid_pickup", pos, Some(Who::Player), None);
        let filled = ItemStack::new(filled, 1);
        self.filled_result(filled.clone());
        self.trigger_item("minecraft:filled_bucket", &filled);
        self.swing_arm();
        Ok(())
    }
    /// `ItemUtils.createFilledResult(held, player, filled)` for the held stack.
    pub(super) fn filled_result(&mut self, filled: ItemStack) {
        self.filled_result_in(self.held_slot(), filled);
    }
    /// `createFilledResult` for the stack in `slot` (`stack_in_mut`'s numbers). What the
    /// inventory doesn't take is dropped as `Player.drop(stack, false)`: no thrower, statistic
    /// or drop hook.
    pub(super) fn filled_result_in(&mut self, slot: usize, filled: ItemStack) {
        let has = self.creative
            && self
                .inventory
                .iter()
                .flatten()
                .any(|s| s.same_item_same_components(&filled));
        let held = self.stack_in_mut(slot);
        let (kept, out) = filled_result_split(held.take(), filled, self.creative, has);
        *self.stack_in_mut(slot) = kept;
        if let Some(rest) = out.and_then(|out| self.give(out)) {
            self.drop_plain(rest);
        }
    }
}

/// `createFilledResult` on `held`: what the slot keeps, and the item to give the player. In
/// creative (`hasInfiniteMaterials`) `held` is kept and `filled` given unless the inventory
/// `has` it; otherwise one item of `held` becomes `filled`, in place when it was the last.
pub(super) fn filled_result_split(
    held: Option<ItemStack>,
    filled: ItemStack,
    creative: bool,
    has: bool,
) -> (Option<ItemStack>, Option<ItemStack>) {
    let filled = (filled.count > 0).then_some(filled);
    if creative {
        return (held, filled.filter(|_| !has));
    }
    match held {
        Some(held) if held.count > 1 => (
            Some(ItemStack {
                count: held.count - 1,
                ..held
            }),
            filled,
        ),
        _ => (filled, None),
    }
}

#[cfg(test)]
mod filled_result_tests {
    use super::filled_result_split;
    use horizon_client::items::ItemStack;

    fn stack(item: &str, count: u32) -> ItemStack {
        ItemStack {
            item: item.into(),
            count,
            damage: 0,
            extra: None,
        }
    }

    #[test]
    fn creative_keeps_the_held_stack_and_gives_the_filled_one_once() {
        let bucket = || Some(stack("minecraft:bucket", 1));
        let filled = || stack("minecraft:powder_snow_bucket", 1);
        assert_eq!(
            filled_result_split(bucket(), filled(), true, false),
            (bucket(), Some(filled()))
        );
        assert_eq!(
            filled_result_split(bucket(), filled(), true, true),
            (bucket(), None)
        );
        // Survival: the last item becomes the filled one; from more, one splits off.
        assert_eq!(
            filled_result_split(bucket(), filled(), false, false),
            (Some(filled()), None)
        );
        assert_eq!(
            filled_result_split(Some(stack("minecraft:bucket", 3)), filled(), false, false),
            (Some(stack("minecraft:bucket", 2)), Some(filled()))
        );
    }
}
