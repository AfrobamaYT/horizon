//! The `players` area: host methods the game implements (`game/host/players.rs`).
#![allow(unused_variables)]
use crate::*;
/// The host's methods both sides share of the `players` area, implemented by the game and reached through [`Host`].
pub trait PlayersHost {
    /// Side: both.
    /// `ItemCooldowns.getCooldownGroup(stack)`: the group `stack`'s
    /// cooldown is filed under, the group its `use_cooldown` component names
    /// (its own, else its item's default), else its item id.
    fn item_cooldown_group(&self, stack: &ItemStack) -> String {
        crate::phase::unprovided(
            "Host::item_cooldown_group",
            "item_cooldown_group",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// Resolve a real online player by UUID, never a fake. A fake is reached
    /// through the handle returned by its factory. During a nested player swap,
    /// outer players whose records are displaced return None.
    fn player(&self, uuid: u128) -> Option<PlayerRef> {
        crate::phase::unprovided(
            "Host::player",
            "player",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The server host's methods of the `players` area, implemented by the game and reached through [`ServerHost`].
pub trait PlayersServerHost {
    /// Side: server.
    /// Create or retrieve a connectionless player for the level currently in place.
    fn fake_player(&mut self, level: LevelRef, uuid: u128, name: &str) -> Option<PlayerRef> {
        crate::phase::unprovided(
            "ServerHost::fake_player",
            "fake_player",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The client host's methods of the `players` area, implemented by the game and reached through [`crate::ClientHost`].
pub trait PlayersClientHost {
    /// Side: client.
    /// For a client mod: a push added to the local player's velocity, in blocks a tick, as an
    /// explosion's knockback is (the window's player moves itself, as vanilla's client does). Each component is
    /// clamped to -4..4; a spectator, or a value that is not a number, is left alone.
    fn push_player(&mut self, kick: [f32; 3]) {
        crate::phase::unprovided(
            "ClientHost::push_player",
            "push_player",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' reads of the `players` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait PlayersBackend {
    /// Side: server.
    /// The IP address the player connects from, without the port (behind a
    /// Velocity proxy, the address the proxy forwards). Displaced connected
    /// players keep their address; an acting player with no known connection
    /// has an empty address. None for a stale or unavailable player.
    fn player_address(&self, player: PlayerRef) -> Option<String> {
        crate::phase::unprovided(
            "Backend::player_address",
            "player_address",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The stable name of a connected or acting player. None for a stale or
    /// unavailable player; displaced online players retain their identity.
    fn player_name(&self, player: PlayerRef) -> Option<String> {
        crate::phase::unprovided(
            "Backend::player_name",
            "player_name",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Inventory.getSelectedSlot`: the hotbar slot the player holds, 0..9.
    fn player_selected_slot(&self, player: PlayerRef) -> Option<usize> {
        crate::phase::unprovided(
            "Backend::player_selected_slot",
            "player_selected_slot",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// A player's current supported menu. Refuses crafting tables, merchants,
    /// workstations, lecterns, mounts and menus with hidden slots; also refuses
    /// clients, stale players and players whose state is swapped out.
    fn player_container_menu(&self, player: PlayerRef) -> Option<MenuRef> {
        crate::phase::unprovided(
            "Backend::player_container_menu",
            "player_container_menu",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Every group the player waits on and its ticks left, ordered by
    /// group.
    fn player_item_cooldowns(&self, player: PlayerRef) -> Option<Vec<NativeItemCooldown>> {
        crate::phase::unprovided(
            "Backend::player_item_cooldowns",
            "player_item_cooldowns",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemCooldowns.isOnCooldown`: whether `group` still has time to run.
    fn player_has_item_cooldown(&self, player: PlayerRef, group: &str) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_has_item_cooldown",
            "player_has_item_cooldown",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemCooldowns.getCooldownPercent(group, 0)`: what is left of the
    /// group's cooldown, from 1 (just started) to 0 (none).
    fn player_item_cooldown_percent(&self, player: PlayerRef, group: &str) -> Option<f32> {
        crate::phase::unprovided(
            "Backend::player_item_cooldown_percent",
            "player_item_cooldown_percent",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Player.takeXpDelay`: the ticks before the player can take the next
    /// experience orb.
    fn player_exp_cooldown(&self, player: PlayerRef) -> Option<u32> {
        crate::phase::unprovided(
            "Backend::player_exp_cooldown",
            "player_exp_cooldown",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// The dimension and block of the player's last death
    /// (`Player.getLastDeathLocation`).
    fn player_last_death_location(&self, player: PlayerRef) -> Option<GlobalPos> {
        crate::phase::unprovided(
            "Backend::player_last_death_location",
            "player_last_death_location",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `FoodData.needsFood`: the food level is below 20.
    fn player_needs_food(&self, player: PlayerRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_needs_food",
            "player_needs_food",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `FoodData.hasEnoughFood`: the food level is above 6, enough to
    /// sprint.
    fn player_has_enough_food(&self, player: PlayerRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_has_enough_food",
            "player_has_enough_food",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Player.canEat(always)`: an invulnerable player (creative), `always`
    /// (food that can always be eaten) or a food level below 20.
    fn player_can_eat(&self, player: PlayerRef, always: bool) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_can_eat",
            "player_can_eat",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Player.experienceLevel`, `experienceProgress` and `totalExperience`.
    fn player_experience(&self, player: PlayerRef) -> Option<NativePlayerExperience> {
        crate::phase::unprovided(
            "Backend::player_experience",
            "player_experience",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Player.getXpNeededForNextLevel`: the points the current level needs
    /// to fill.
    fn player_xp_needed_for_next_level(&self, player: PlayerRef) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::player_xp_needed_for_next_level",
            "player_xp_needed_for_next_level",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The player's permission level, 0 to 4 (`Player.permissions()`,
    /// `PlayerList.getProfilePermissions`): an op's level, the
    /// single-player owner's 4 with commands allowed. A displaced connected
    /// player keeps their permission level even while their body is unavailable.
    /// None for a stale or unavailable player.
    fn player_permission_level(&self, player: PlayerRef) -> Option<u8> {
        crate::phase::unprovided(
            "Backend::player_permission_level",
            "player_permission_level",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Player.hasInfiniteMaterials`: the `instabuild` ability, which
    /// creative mode gives.
    fn player_has_infinite_materials(&self, player: PlayerRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_has_infinite_materials",
            "player_has_infinite_materials",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Player.canUseGameMasterBlocks`: creative and a permission level of
    /// at least 2, as command blocks, structure blocks and jigsaws ask.
    fn player_can_use_game_master_blocks(&self, player: PlayerRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_can_use_game_master_blocks",
            "player_can_use_game_master_blocks",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ServerPlayerGameMode.getPreviousGameModeForPlayer`: the mode the
    /// player last left (`previousPlayerGameType`, saved with the player), or
    /// `Some(None)` when they never changed mode. None for a stale or
    /// unavailable player.
    fn player_previous_game_mode(&self, player: PlayerRef) -> Option<Option<GameMode>> {
        crate::phase::unprovided(
            "Backend::player_previous_game_mode",
            "player_previous_game_mode",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Player.isSecondaryUseActive`: whether the player uses items and
    /// blocks as with sneak held, as vanilla reads it from the shift-key
    /// flag (the same read as `EntityRef::is_shift_key_down`). None for a
    /// stale or unavailable player.
    fn player_is_secondary_use_active(&self, player: PlayerRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_is_secondary_use_active",
            "player_is_secondary_use_active",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether a live player handle names a connectionless actor.
    fn player_is_fake(&self, player: PlayerRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_is_fake",
            "player_is_fake",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Inventory.items`: the non-empty main inventory stacks in slot order, including
    /// their components. None for a stale or unavailable player; on clients only the
    /// local player's inventory is available.
    fn player_inventory(&self, player: PlayerRef) -> Option<Vec<ItemStack>> {
        crate::phase::unprovided(
            "Backend::player_inventory",
            "player_inventory",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `ServerPlayerGameMode.getGameModeForPlayer`; Paper `HumanEntity#getGameMode`.
    /// None for stale, offline or unavailable players (including displaced outer players).
    fn player_game_mode(&self, player: PlayerRef) -> Option<GameMode> {
        crate::phase::unprovided(
            "Backend::player_game_mode",
            "player_game_mode",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Player.isCreative`; Paper `HumanEntity#getGameMode`.
    fn player_is_creative(&self, player: PlayerRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_is_creative",
            "player_is_creative",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Player.isSpectator`; Paper `HumanEntity#getGameMode`.
    fn player_is_spectator(&self, player: PlayerRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_is_spectator",
            "player_is_spectator",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Player.mayBuild`; Paper `HumanEntity#getGameMode`.
    fn player_may_build(&self, player: PlayerRef) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_may_build",
            "player_may_build",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `FoodData.getFoodLevel/getSaturationLevel/getExhaustionLevel`; Paper
    /// `HumanEntity#getFoodLevel/getSaturation/getExhaustion`. None until the
    /// guest's body exists, as well as for stale or unavailable players.
    fn player_food(&self, player: PlayerRef) -> Option<NativePlayerFood> {
        crate::phase::unprovided(
            "Backend::player_food",
            "player_food",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: both.
    /// `Player.getAbilities`; Paper `Player#getAllowFlight/isFlying/getFlySpeed/getWalkSpeed`.
    fn player_abilities(&self, player: PlayerRef) -> Option<NativePlayerAbilities> {
        crate::phase::unprovided(
            "Backend::player_abilities",
            "player_abilities",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The value of statistic `stat_type` for `key` (`minecraft:stone`,
    /// `minecraft:jump`...), as `ServerStatsCounter.getValue`: 0 when never
    /// counted.
    fn player_stat(&self, player: PlayerRef, stat_type: NativeStatType, key: &str) -> Option<i32> {
        crate::phase::unprovided(
            "Backend::player_stat",
            "player_stat",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether the player's recipe book knows `recipe`
    /// (`ServerRecipeBook.contains`).
    fn player_has_discovered_recipe(&self, player: PlayerRef, recipe: &str) -> Option<bool> {
        crate::phase::unprovided(
            "Backend::player_has_discovered_recipe",
            "player_has_discovered_recipe",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' client reads of the `players` area, implemented by the game and reached through [`Backend`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait PlayersClientBackend {
    /// Side: client.
    /// The local player's predicted breaking state, including Idle. None for
    /// stale, nonlocal or fake handles and outside a logged-in client world.
    fn player_block_breaking(&self, player: PlayerRef) -> Option<BlockBreakingState> {
        crate::phase::unprovided(
            "Backend::player_block_breaking",
            "player_block_breaking",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: client.
    /// The local player's existing cave ambience mood, normally 0 through 1.
    /// None for stale, nonlocal or fake handles and outside a client world.
    fn player_ambient_mood(&self, player: PlayerRef) -> Option<f32> {
        crate::phase::unprovided(
            "Backend::player_ambient_mood",
            "player_ambient_mood",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
/// The handles' writes of the `players` area, implemented by the game and reached through [`BackendMut`].
// hidden: a handle's backend, reached through `Backend`; mods call the handle, never this trait
#[doc(hidden)]
pub trait PlayersBackendMut {
    /// Side: server.
    /// A system line in that player's chat, also inside a player's hook for
    /// the acting player; false when they aren't on.
    fn player_tell(&mut self, player: PlayerRef, line: Text) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_tell",
            "player_tell",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// A line over that player's hotbar (the action bar), also inside a
    /// player's hook for the acting player; false when they aren't on.
    fn player_action_bar(&mut self, player: PlayerRef, line: Text) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_action_bar",
            "player_action_bar",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// The lines over and under that player's list of players (Tab):
    /// `ClientboundTabListPacket`, an empty text for none. False when they
    /// aren't on.
    fn player_tab_list(&mut self, player: PlayerRef, header: &Text, footer: &Text) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_tab_list",
            "player_tab_list",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Whether that player counts for `players_sleeping_percentage`: an
    /// ignored player who isn't in bed is left out of the players the night
    /// needs asleep, as a spectator is (an away-from-keyboard mod). Kept
    /// until a mod sets it back or the server stops, also if they leave and
    /// come back. False when the server list is unavailable, the handle is
    /// stale or the player isn't online.
    fn player_set_sleep_ignored(&mut self, player: PlayerRef, ignored: bool) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_sleep_ignored",
            "player_set_sleep_ignored",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Shows `nickname` in place of that player's name in chat (their
    /// messages, `/me`, `/msg`) and in the Tab list; None shows their name
    /// again. Hovering the shown name gives the real one, which stays in the
    /// log, commands, selectors and the server's events. Kept until a mod
    /// sets it again or the server stops (also if they leave and come back).
    /// Plain text: the mod checks it (`Messages::text_allowed`, its length,
    /// other players' names). False when the server list is unavailable,
    /// the handle is stale or the player isn't online.
    fn player_set_nickname(&mut self, player: PlayerRef, nickname: Option<&str>) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_nickname",
            "player_set_nickname",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Hides that player (a vanish) from the players below permission
    /// `level`, or shows them again (None): from the next tick those players
    /// lose them from the Tab list and the world (their client stops
    /// tracking the player, as it does a spectator), and don't read their
    /// joining or leaving; players of `level` or more still see them, and
    /// a player made an operator meanwhile sees them again. Kept by UUID, even
    /// after this handle disconnects (the player rejoins hidden), until a mod sets it again
    /// or the server stops. Works inside a player's hook too; false for a stale
    /// world epoch, on a client and in a world nobody else can join.
    fn player_hide(&mut self, player: PlayerRef, level: Option<u8>) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_hide",
            "player_hide",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Opens a block entity's menu for a connected player (`ServerPlayer.openMenu`).
    /// Returns Stale for an invalidated entity; Ok(false) for a wrong-level player,
    /// non-provider, cancelled open, or an unconnected player such as a fake player.
    fn player_open_menu(
        &mut self,
        player: PlayerRef,
        entity: BlockEntityRef,
    ) -> Result<bool, BlockEntityError> {
        crate::phase::unprovided(
            "BackendMut::player_open_menu",
            "player_open_menu",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sends the player `uuid` the whole picture of map `map_id`: 128 x 128
    /// colours (`MapColor` ids times 4 plus the shade, row by row), scale 0,
    /// locked, no decorations, as `ClientboundMapItemDataPacket`. A filled
    /// map the mod gives with that `minecraft:map_id` then shows it in hand
    /// and in item frames; the server keeps nothing (a map id with no saved
    /// map is never sent again by the game), so send it again after the
    /// player joins. Use ids far from the world's own (they count up from 0).
    /// False for a player not here, or `colors` not 16384 long.
    fn player_send_map(&mut self, player: PlayerRef, map_id: i32, colors: &[u8]) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_send_map",
            "player_send_map",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Stores a server player's `Abilities` and tells their client
    /// (`ServerPlayer.onUpdateAbilities`). They stay as written until the
    /// player's game mode changes (`GameType.updatePlayerAbilities`), and are
    /// saved with the player. A speed that is not finite refuses the write
    /// (vanilla's setters take any float). Returns false for a
    /// stale or unavailable player, a displaced outer player, and a speed
    /// that is not finite.
    fn player_set_abilities(
        &mut self,
        player: PlayerRef,
        abilities: NativePlayerAbilities,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_abilities",
            "player_set_abilities",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// A fake player acts as a player does, through the game's own path for the action
    /// (`ServerGamePacketListenerImpl.handleUseItemOn` / `handleUseItem` / `handleInteract`'s
    /// ATTACK, and `ServerPlayerGameMode.destroyBlock`), as its packet would run it: reach,
    /// protection and the border, then the action. Once it runs, an effect a fake can't take
    /// part in (a menu, sleep, a ride, another dimension) is stopped where it would start, and
    /// the answer says which; the action's own side effects (the swing, the hooks before it,
    /// vanilla's result) have still happened then. The fake is found once, and its turn
    /// starts at that place in the list.
    fn player_perform_action(
        &mut self,
        player: PlayerRef,
        action: PlayerAction,
    ) -> Result<ActionOutcome, ActionRefused> {
        crate::phase::unprovided(
            "BackendMut::player_perform_action",
            "player_perform_action",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// A machine's one use on the persistent actor, with its private state put back even
    /// when the action refuses. Only the existing guest turn runs gameplay and its hooks.
    fn player_perform_action_scoped(
        &mut self,
        player: PlayerRef,
        action: PlayerAction,
        setup: PlayerActionSetup,
    ) -> Result<ScopedActionResult, ActionRefused> {
        crate::phase::unprovided(
            "BackendMut::player_perform_action_scoped",
            "player_perform_action_scoped",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemCooldowns.addCooldown(group, ticks)`: starts (or restarts) the
    /// group's cooldown, and the player's client draws it on every item of the
    /// group (`ServerItemCooldowns.onCooldownStarted`).
    fn player_set_item_cooldown(&mut self, player: PlayerRef, group: &str, ticks: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_item_cooldown",
            "player_set_item_cooldown",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ItemCooldowns.removeCooldown(group)`: ends the group's cooldown now,
    /// on the client too (`ServerItemCooldowns.onCooldownEnded`).
    fn player_remove_item_cooldown(&mut self, player: PlayerRef, group: &str) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_remove_item_cooldown",
            "player_remove_item_cooldown",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets `Player.takeXpDelay`: no orb is taken for `ticks` ticks. The
    /// change is not reported to `player_exp_cooldown_change`.
    fn player_set_exp_cooldown(&mut self, player: PlayerRef, ticks: u32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_exp_cooldown",
            "player_set_exp_cooldown",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets the player's `WardenSpawnTracker.cooldownTicks`: sculk shriekers
    /// they set off don't warn for `ticks` ticks. The level's players share
    /// one tracker here, the integrated server owner's (the shriekers read it
    /// for whoever sets them off), so this is every player's.
    /// False, changing nothing, for anyone but the tracker's owner until each player has their own.
    fn player_set_warden_warning_cooldown(&mut self, player: PlayerRef, ticks: u32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_warden_warning_cooldown",
            "player_set_warden_warning_cooldown",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Fires the mod's criteria trigger `trigger` (`mymod:pipe_connected`)
    /// for `player`: vanilla's
    /// `SimpleCriterionTrigger.trigger`. Each criterion of theirs with that
    /// trigger, not yet granted, of an advancement not yet done, is tested
    /// by `test` and then by its `player` predicate; those that pass are
    /// granted, and an advancement they complete is awarded with its toast,
    /// chat line and rewards, as the game's own triggers award.
    ///
    /// The acting player is reached inside their hook; another live player
    /// is reached in their own level, including during a guest's turn. An
    /// outer player displaced by a nested turn is unavailable. `Ok(())`
    /// means the player was reached, whether or not anything was granted.
    /// A stale/offline/displaced handle, a remote client, or a `minecraft:`
    /// trigger or empty id is refused with a typed reason.
    ///
    /// `test` is lent to this call only. A failed test stops further testing
    /// and reports `Test`; earlier matching criteria may already be granted.
    /// Wasm's generated resource route exists, but a callback into the group
    /// currently firing the trigger is refused by the runtime's reentry guard.
    fn player_trigger_criterion(
        &mut self,
        player: PlayerRef,
        trigger: &Identifier,
        test: Arc<dyn CriterionTest>,
    ) -> Result<(), TriggerCriterionError> {
        crate::phase::unprovided(
            "BackendMut::player_trigger_criterion",
            "player_trigger_criterion",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Player.setLastDeathLocation`: stores where the player died, or
    /// forgets it for None. The client hears of it with the player's next
    /// respawn, as in vanilla. False in the cases a read gives None.
    fn player_set_last_death_location(
        &mut self,
        player: PlayerRef,
        location: Option<GlobalPos>,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_last_death_location",
            "player_set_last_death_location",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `FoodData.eat(nutrition, saturationModifier)`: adds `nutrition` to
    /// the food level (0 to 20) and `nutrition * modifier * 2` to the
    /// saturation, which stays at most the food level.
    fn player_feed(&mut self, player: PlayerRef, nutrition: i32, saturation_modifier: f32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_feed",
            "player_feed",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Player.causeFoodExhaustion`: adds exhaustion (at most 40), as
    /// sprinting or jumping does. An invulnerable player (creative) or a
    /// spectator gets none, and the call still returns true.
    fn player_add_exhaustion(&mut self, player: PlayerRef, amount: f32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_add_exhaustion",
            "player_add_exhaustion",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `FoodData.setFoodLevel`: sets the food level as it is, unclamped,
    /// as vanilla does.
    fn player_set_food_level(&mut self, player: PlayerRef, level: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_food_level",
            "player_set_food_level",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `FoodData.setSaturation`: sets the saturation as it is, unclamped.
    fn player_set_saturation(&mut self, player: PlayerRef, saturation: f32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_saturation",
            "player_set_saturation",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Sets the exhaustion as it is (Paper's `HumanEntity#setExhaustion`;
    /// vanilla's `FoodData` only adds to it).
    fn player_set_exhaustion(&mut self, player: PlayerRef, exhaustion: f32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_exhaustion",
            "player_set_exhaustion",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ServerPlayer.setExperiencePoints`: sets the progress toward the next
    /// level to `points` of the points it needs, kept below a full bar. The
    /// level and the total are kept.
    fn player_set_experience_points(&mut self, player: PlayerRef, points: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_experience_points",
            "player_set_experience_points",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ServerPlayer.setExperienceLevels`: sets the level, keeping the
    /// progress and the total.
    fn player_set_experience_levels(&mut self, player: PlayerRef, levels: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_experience_levels",
            "player_set_experience_levels",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Changes a player's game mode as `ServerPlayer.setGameMode` and
    /// `/gamemode` do, including the cancellable change hook and ability
    /// updates. Returns false for a stale or unavailable player, an active
    /// outer guest, no change, or a canceled change.
    fn player_set_game_mode(&mut self, player: PlayerRef, mode: GameMode) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_set_game_mode",
            "player_set_game_mode",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// Remove a fake without treating it as a disconnect or saving it.
    fn player_remove_fake(&mut self, player: PlayerRef) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_remove_fake",
            "player_remove_fake",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Player.awardStat(stat, amount)`: adds `amount`, as the game does
    /// for its own counts, so `player_stat_award` hears it and may cancel
    /// it, and scoreboard criteria follow. False for a player it can't
    /// reach.
    fn player_award_stat(
        &mut self,
        player: PlayerRef,
        stat_type: NativeStatType,
        key: &str,
        amount: i32,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_award_stat",
            "player_award_stat",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Player.resetStat`: sets the statistic back to 0, which scoreboard
    /// criteria follow. False for a player it can't reach.
    fn player_reset_stat(
        &mut self,
        player: PlayerRef,
        stat_type: NativeStatType,
        key: &str,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_reset_stat",
            "player_reset_stat",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ServerPlayer.awardRecipes`, as `/recipe give` does: the recipes
    /// unlock in the player's book, with their toasts and the
    /// `player_recipe_discovered` hook, and the client hears of them.
    /// Unknown ids are skipped. The number of recipe displays added, None
    /// for a player it can't reach.
    fn player_award_recipes(&mut self, player: PlayerRef, recipes: &[&str]) -> Option<i32> {
        crate::phase::unprovided(
            "BackendMut::player_award_recipes",
            "player_award_recipes",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ServerPlayer.resetRecipes`, as `/recipe take` does. The number of
    /// recipe displays taken out, None for a player it can't reach.
    fn player_reset_recipes(&mut self, player: PlayerRef, recipes: &[&str]) -> Option<i32> {
        crate::phase::unprovided(
            "BackendMut::player_reset_recipes",
            "player_reset_recipes",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `ExperienceOrb.playerTouch`: Mending repairs the player's damaged
    /// items with `points` first (`ExperienceOrb.repairPlayerItems`), then
    /// only the rest goes to their experience
    /// (`Player.giveExperiencePoints`), with the level-up sound. No orb
    /// pickup sound. False, and nothing given, as the other writes; `points`
    /// above `i32::MAX` is refused before Mending repairs anything.
    fn player_give_experience_points(&mut self, player: PlayerRef, points: u32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_give_experience_points",
            "player_give_experience_points",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `Player.giveExperienceLevels`: adds `levels`, or takes them off when
    /// negative, as the anvil and the enchanting table charge with. Below 0
    /// the level, the total and the progress toward the next level are all 0.
    fn player_give_levels(&mut self, player: PlayerRef, levels: i32) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_give_levels",
            "player_give_levels",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `StopSoundCommand` for one player: their client stops the playing
    /// sounds that match `sound` (an event id) and `source`; `None` matches
    /// any. True means sent.
    fn player_stop_sound(
        &mut self,
        player: PlayerRef,
        sound: Option<&str>,
        source: Option<SoundSource>,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_stop_sound",
            "player_stop_sound",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
    /// Side: server.
    /// `PlaySoundCommand` for one player: plays sound event `sound` at the
    /// player, for them only, under `source`'s volume slider. The seed is the
    /// level random's `nextLong`, drawn once per call. True means sent, not
    /// heard.
    fn player_play_sound(
        &mut self,
        player: PlayerRef,
        sound: &str,
        source: SoundSource,
        volume: f32,
        pitch: f32,
    ) -> bool {
        crate::phase::unprovided(
            "BackendMut::player_play_sound",
            "player_play_sound",
            crate::phase::LoadAccess::WorldOnly,
        )
    }
}
