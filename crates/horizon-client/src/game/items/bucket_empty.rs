//! Emptying a filled bucket from a player's hand: the `empty_contents` Part's say, the guest's
//! sounds and the client's prediction.
use super::{Game, Hit};
use horizon_client::first_person::UseResult;
use horizon_client::items::ItemStack;
use horizon_core::{geometry::Direction, position::BlockPos};
use horizon_world::level::BucketContent;

impl Game<'_> {
    /// `BucketItem.use`'s `emptyContents` for the acting player's `stack`,
    /// poured at `target` from the clicked `hit`: the use's result when the
    /// item's `empty_contents` Part answered, `None` for vanilla's pour.
    pub(in crate::game) fn player_empty_contents(
        &mut self,
        stack: &ItemStack,
        target: [i32; 3],
        hit: &Hit,
    ) -> Option<UseResult> {
        let face = match hit.face {
            [0, 1, 0] => Direction::Up,
            [0, -1, 0] => Direction::Down,
            [0, 0, -1] => Direction::North,
            [0, 0, 1] => Direction::South,
            [-1, 0, 0] => Direction::West,
            _ => Direction::East,
        };
        let clicked = BlockPos::new(hit.pos[0], hit.pos[1], hit.pos[2]);
        let pos = BlockPos::new(target[0], target[1], target[2]);
        if !self.native_empty_contents_event(pos, Some((clicked, face)), stack)? {
            return Some(UseResult::Fail);
        }
        // `BucketItem.getEmptySuccessItem`: the empty bucket, kept full in creative.
        if !self.creative {
            *self.held_mut() = Some(ItemStack::new("minecraft:bucket", 1));
        }
        Some(UseResult::Success { swing: true })
    }

    /// `BucketItem.emptyContents` plays `playSound(player, …)`: a guest's own client already
    /// played the empty or evaporate `sound` from its prediction (#4684), so those go to the
    /// guest's own list instead of being sent back to them.
    pub(in crate::game) fn keep_guest_bucket_sounds(&mut self, sound: &str) {
        if self.lan_acting.is_none() {
            return;
        }
        let Some(capture) = self.audio.capture.as_mut() else {
            return;
        };
        let want = sound.trim_start_matches("minecraft:");
        let mut keep = Vec::with_capacity(capture.len());
        for entry in capture.drain(..) {
            let name = entry.event.trim_start_matches("minecraft:");
            if name == want || name == "block.fire.extinguish" {
                self.guest_player_sounds.push((
                    entry.event,
                    entry.position,
                    entry.volume,
                    entry.pitch,
                ));
            } else {
                keep.push(entry);
            }
        }
        *capture = keep;
    }

    /// `bucket_contents` for the pour a server's client predicts: none for an
    /// item whose `empty_contents` Part answers on the server, whose result
    /// stands.
    pub(in crate::game) fn predicted_bucket_contents<'s>(
        &self,
        stack: &'s ItemStack,
    ) -> Option<(BucketContent, &'s str, Option<&'s str>)> {
        if self.empty_contents_listens(&stack.item) {
            return None;
        }
        horizon_client::items::bucket_contents(&stack.item)
    }
}
