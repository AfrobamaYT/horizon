//! Borrow the storage a player's active guest turn actually owns.

use super::super::guest::Guest;
use crate::{advancements::Progress, recipe_book::Settings};
use std::collections::BTreeSet;

/// The advancement fields swapped together when entering and leaving a guest.
pub struct AdvancementState<'a> {
    pub progress: &'a mut Progress,
    pub changed: &'a mut BTreeSet<usize>,
    pub unlocked: &'a mut Vec<String>,
    pub book: &'a mut Settings,
}

/// Active turns, in entry order, and the current turn's live storage. Other
/// players keep their state in their own record; an outer active turn keeps it
/// in the next nested guest's record, as `enter_guest` and `leave_guest` swap it.
#[derive(Default)]
pub struct AdvancementOwners<'a> {
    turns: &'a [u32],
    live: Option<AdvancementState<'a>>,
}

impl<'a> AdvancementOwners<'a> {
    pub fn active(turns: &'a [u32], live: AdvancementState<'a>) -> Self {
        Self {
            turns,
            live: Some(live),
        }
    }

    /// `index` names the player, not necessarily the record holding their state.
    /// Inactive players use that index directly; only nested turns need a lookup.
    pub fn state<'b>(
        &'b mut self,
        guests: &'b mut [Guest],
        mut index: usize,
    ) -> Option<AdvancementState<'b>> {
        let conn = guests.get(index)?.conn;
        if let Some(turn) = self.turns.iter().position(|active| *active == conn) {
            if let Some(next) = self.turns.get(turn + 1) {
                index = guests.iter().position(|guest| guest.conn == *next)?;
            } else {
                let live = self.live.as_mut()?;
                return Some(AdvancementState {
                    progress: live.progress,
                    changed: live.changed,
                    unlocked: live.unlocked,
                    book: live.book,
                });
            }
        }
        let guest = guests.get_mut(index)?;
        Some(AdvancementState {
            progress: &mut guest.advancements,
            changed: &mut guest.advancements_changed,
            unlocked: &mut guest.recipes_unlocked,
            book: &mut guest.recipe_book,
        })
    }
}
