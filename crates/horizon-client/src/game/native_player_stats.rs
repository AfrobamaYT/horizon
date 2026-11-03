//! A player's statistics and recipe book for native server mods, through a
//! `PlayerRef` (docs/native-mods.md): `ServerStatsCounter` and
//! `ServerRecipeBook`. Reads return None for a stale handle or a player who
//! isn't online here. Writes return false (or None) for those, for a dead
//! player, and, inside another player's turn, for anyone but that player.
use super::{Game, stats::Stats};
use horizon_client::mod_api::PlayerRef;

/// The vanilla stat types (`BuiltInRegistries.STAT_TYPE`), by path.
const STAT_TYPES: [&str; 9] = [
    "mined",
    "crafted",
    "used",
    "broken",
    "picked_up",
    "dropped",
    "killed",
    "killed_by",
    "custom",
];

/// A stat type's path (`minecraft:used` or `used`), or None for one vanilla
/// doesn't have.
fn stat_type(ty: &str) -> Option<&str> {
    let path = ty.strip_prefix("minecraft:").unwrap_or(ty);
    STAT_TYPES.contains(&path).then_some(path)
}

/// An id with its namespace: `stone` is `minecraft:stone`.
fn qualified(id: &str) -> String {
    if id.contains(':') {
        id.to_owned()
    } else {
        format!("minecraft:{id}")
    }
}

impl Game<'_> {
    /// Whether `player` still names the same player on this server.
    fn native_player_current(&self, player: PlayerRef) -> bool {
        self.native_player_ref(player.uuid()) == Some(player)
    }
    /// The guest record `player` reads from outside their turn, or None
    /// when their state is in the game (they act, or they are the host).
    fn native_player_resting(
        &self,
        player: PlayerRef,
    ) -> Option<&horizon_client::lan::guest::Guest> {
        if self.lan_acting_uuid == Some(player.uuid()) {
            return None;
        }
        self.lan
            .iter()
            .flat_map(|h| h.hub.guests.iter())
            .find(|g| g.placed && g.uuid == player.uuid())
    }
    /// The player's statistics, where they are kept now.
    fn native_player_stats(&self, player: PlayerRef) -> Option<Option<&Stats>> {
        if !self.native_player_current(player) {
            return None;
        }
        Some(match self.native_player_resting(player) {
            Some(g) => self.lan.as_ref()?.play.get(&g.conn).map(|p| &p.stats),
            None => Some(&self.stats),
        })
    }

    /// The value of statistic `stat_type` (`minecraft:mined`,
    /// `minecraft:custom`...) for `key` (`minecraft:stone`,
    /// `minecraft:jump`...), as `ServerStatsCounter.getValue`: 0 when never
    /// counted. None for a stat type vanilla doesn't have. API
    /// @API:player_stats@.
    pub(super) fn native_player_stat(
        &self,
        player: PlayerRef,
        stat_type: &str,
        key: &str,
    ) -> Option<i32> {
        let ty = self::stat_type(stat_type)?;
        let stats = self.native_player_stats(player)?;
        Some(stats.map_or(0, |s| s.get(ty, &qualified(key))))
    }
    /// `Player.awardStat(stat, amount)`: adds `amount`, as the game does
    /// for its own counts, so `player_stat_award` hears it and may cancel
    /// it, and scoreboard criteria follow. False for an unknown stat type,
    /// or a player it can't reach. API @API:player_stats@.
    pub(super) fn native_player_award_stat(
        &mut self,
        player: PlayerRef,
        stat_type: &str,
        key: &str,
        amount: i32,
    ) -> bool {
        let Some(ty) = self::stat_type(stat_type) else {
            return false;
        };
        let key = qualified(key);
        self.native_player_current(player)
            && self.native_as_player(player.uuid(), |game| game.award_stat(ty, &key, amount))
    }
    /// `Player.resetStat`: sets the statistic back to 0, which scoreboard
    /// criteria follow. False for an unknown stat type, or a player it can't
    /// reach. API @API:player_stats@.
    pub(super) fn native_player_reset_stat(
        &mut self,
        player: PlayerRef,
        stat_type: &str,
        key: &str,
    ) -> bool {
        let Some(ty) = self::stat_type(stat_type) else {
            return false;
        };
        let key = qualified(key);
        self.native_player_current(player)
            && self.native_as_player(player.uuid(), |game| game.stats.reset(ty, &key))
    }

    /// Whether the player's recipe book knows `recipe`
    /// (`ServerRecipeBook.contains`). API @API:player_stats@.
    pub(super) fn native_player_has_discovered_recipe(
        &self,
        player: PlayerRef,
        recipe: &str,
    ) -> Option<bool> {
        if !self.native_player_current(player) {
            return None;
        }
        let id = qualified(recipe);
        Some(match self.native_player_resting(player) {
            Some(g) => g.recipe_book.recipes.contains(&id),
            None => self.level.recipe_book.recipes.contains(&id),
        })
    }
    /// `ServerPlayer.awardRecipes`, as `/recipe give` does: the recipes
    /// unlock in the player's book, with their toasts and the
    /// `player_recipe_discovered` hook, and the client hears of them.
    /// Unknown ids are skipped. The number of recipe displays added, None
    /// for a player it can't reach. API @API:player_stats@.
    pub(super) fn native_player_award_recipes(
        &mut self,
        player: PlayerRef,
        recipes: &[&str],
    ) -> Option<i32> {
        self.native_player_change_recipes(player, true, recipes)
    }
    /// `ServerPlayer.resetRecipes`, as `/recipe take` does. The number of
    /// recipe displays taken out, None for a player it can't reach. API
    /// @API:player_stats@.
    pub(super) fn native_player_reset_recipes(
        &mut self,
        player: PlayerRef,
        recipes: &[&str],
    ) -> Option<i32> {
        self.native_player_change_recipes(player, false, recipes)
    }
    fn native_player_change_recipes(
        &mut self,
        player: PlayerRef,
        give: bool,
        recipes: &[&str],
    ) -> Option<i32> {
        if !self.native_player_current(player) {
            return None;
        }
        // `awardRecipes` takes recipes the server has: an id no recipe has
        // would otherwise go into the book.
        let index = &self.stations().recipe_index;
        let ids: Vec<String> = recipes
            .iter()
            .map(|r| qualified(r))
            .filter(|id| index.contains(id))
            .collect();
        let mut count = 0;
        self.native_as_player(player.uuid(), |game| {
            count = game.change_recipes(give, &ids);
        })
        .then_some(count)
    }
}
