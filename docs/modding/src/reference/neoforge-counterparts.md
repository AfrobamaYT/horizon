# NeoForge counterparts

Each hook's `neoforge = ...` key in `crates/horizon-mod-api/src/hooks/` names the NeoForge 26.3 event or method it
matches, since a counterpart's name and contract are the rule for a hook. This page lists the keys as declared.
It is written by `tools/neoforge_counterparts.py` (run it again after changing a key; `--check` tells if the page is
stale). A key names the event class (`PlayerEvent.Clone`) or the extension method (`IPlayerExtension.openMenu`);
notes about cancelling or the event bus belong in the hook's `///` doc, not in the key.

579 hooks: 266 name a counterpart, 150 say they have none, 163 have no key yet.
The names were checked for form, not yet against the NeoForge 26.3 Javadoc; the third table is the part of that
check still to do ([#1029](https://github.com/arismoko/horizon/issues/1029)).

## Hooks with a counterpart

| Hook | Side | NeoForge |
| --- | --- | --- |
| `add_client_reload_listeners` | client | `AddClientReloadListenersEvent` |
| `add_server_reload_listeners` | server | `AddServerReloadListenersEvent` |
| `animal_tame` | server | `AnimalTameEvent` |
| `anvil_craft_post` | server | `AnvilCraftEvent.Post` |
| `anvil_craft_pre` | server | `AnvilCraftEvent.Pre` |
| `anvil_update` | both | `AnvilUpdateEvent` |
| `armor_hurt` | server | `ArmorHurtEvent` |
| `arrow_loose` | server | `ArrowLooseEvent` |
| `arrow_nock` | server | `ArrowNockEvent` |
| `baby_entity_spawn` | server | `BabyEntitySpawnEvent` |
| `block_drops` | server | `BlockDropsEvent` |
| `block_entity_energy` | server | `Capabilities.Energy.BLOCK` |
| `block_grow_feature` | server | `BlockGrowFeatureEvent` |
| `block_tool_modification` | both | `BlockEvent.BlockToolModificationEvent` |
| `blocks` | both | `RegisterEvent` |
| `bonemeal` | both | `BonemealEvent` |
| `break_block` | server | `BlockEvent.BreakEvent` |
| `calculate_player_turn` | client | `CalculatePlayerTurnEvent` |
| `camera_distance` | client | `CalculateDetachedCameraDistanceEvent` |
| `camera_pose` | client | `ViewportEvent.ComputeCameraAngles` |
| `can_continue_sleeping` | server | `CanContinueSleepingEvent` |
| `can_player_sleep` | server | `CanPlayerSleepEvent` |
| `chunk_sent` | server | `ChunkWatchEvent.Sent` |
| `chunk_unwatch` | server | `ChunkWatchEvent.UnWatch` |
| `chunk_watch` | server | `ChunkWatchEvent.Watch` |
| `client_chat` | client | `ClientChatEvent` |
| `client_chat_received` | client | `ClientChatReceivedEvent` |
| `client_information_updated` | server | `ClientInformationUpdatedEvent` |
| `client_level_load` | client | `LevelEvent.Load` |
| `client_level_unload` | client | `LevelEvent.Unload` |
| `client_pause_change_post` | client | `ClientPauseChangeEvent.Post` |
| `client_pause_change_pre` | client | `ClientPauseChangeEvent.Pre` |
| `client_player_change_game_type` | client | `ClientPlayerChangeGameTypeEvent` |
| `client_player_clone` | client | `ClientPlayerNetworkEvent.Clone` |
| `client_player_logging_in` | client | `ClientPlayerNetworkEvent.LoggingIn` |
| `client_player_logging_out` | client | `ClientPlayerNetworkEvent.LoggingOut` |
| `client_setup` | client | `FMLClientSetupEvent` |
| `client_stopped` | client | `ClientStoppedEvent` |
| `client_tick_post` | client | `ClientTickEvent.Post` |
| `client_tick_pre` | client | `ClientTickEvent.Pre` |
| `clone_item_stack` | server | `IBlockExtension.getCloneItemStack` |
| `compute_fog_color` | client | `ViewportEvent.ComputeFogColor` |
| `compute_fov` | client | `ViewportEvent.ComputeFov` |
| `compute_fov_modifier` | client | `ComputeFovModifierEvent` |
| `config_loading` | both | `ModConfigEvent.Loading` |
| `config_reloading` | both | `ModConfigEvent.Reloading` |
| `config_unloading` | server | `ModConfigEvent.Unloading` |
| `create_fluid_source` | server | `BlockEvent.CreateFluidSourceEvent` |
| `create_spawn_position` | server | `LevelEvent.CreateSpawnPosition` |
| `critical_hit` | server | `CriticalHitEvent` |
| `crop_grow_post` | server | `CropGrowEvent.Post` |
| `crop_grow_pre` | server | `CropGrowEvent.Pre` |
| `custom_click_action` | server | `CustomClickActionEvent` |
| `customize_chat_overlay` | client | `CustomizeGuiOverlayEvent.Chat` |
| `data_map_types` | server | `RegisterDataMapTypesEvent` |
| `datapack_sync` | server | `OnDatapackSyncEvent` |
| `difficulty_changed` | server | `DifficultyChangeEvent` |
| `enchantment_level_set` | server | `EnchantmentLevelSetEvent` |
| `ender_entity_teleport` | server | `EntityTeleportEvent.EnderEntity` |
| `ender_pearl_teleport` | server | `EntityTeleportEvent.EnderPearl` |
| `enderman_anger` | server | `EnderManAngerEvent` |
| `entity_interact` | server | `PlayerInteractEvent.EntityInteract` |
| `entity_mob_griefing` | server | `EntityMobGriefingEvent` |
| `entity_mount` | server | `EntityMountEvent` |
| `entity_place` | server | `BlockEvent.EntityPlaceEvent, BlockEvent.EntityMultiPlaceEvent` |
| `entity_struck_by_lightning` | server | `EntityStruckByLightningEvent` |
| `entity_tick_post` | server | `EntityTickEvent.Post` |
| `entity_tick_pre` | server | `EntityTickEvent.Pre` |
| `explosion_knockback` | server | `ExplosionKnockbackEvent` |
| `explosion_start` | server | `ExplosionEvent.Start` |
| `extract_block_screen_effect` | client | `ExtractBlockScreenEffectEvent` |
| `farmland_trample` | server | `BlockEvent.FarmlandTrampleEvent` |
| `finalize_spawn` | server | `FinalizeSpawnEvent` |
| `fluid_place_block` | server | `BlockEvent.FluidPlaceBlockEvent` |
| `game_rule_changed` | server | `GameRuleChangedEvent` |
| `game_shutting_down` | both | `GameShuttingDownEvent` |
| `gather_effect_screen_tooltips` | client | `GatherEffectScreenTooltipsEvent` |
| `get_arm_pose` | client | `IClientItemExtensions.getArmPose` |
| `get_enchantment_level` | both | `GetEnchantmentLevelEvent` |
| `get_equipment_drop_chance` | server | `GetEquipmentDropChanceEvent` |
| `grindstone_place` | server | `GrindstoneEvent.OnPlaceItem` |
| `grindstone_take` | server | `GrindstoneEvent.OnTakeItem` |
| `gui_layer_post` | client | `RenderGuiLayerEvent.Post` |
| `gui_layer_pre` | client | `RenderGuiLayerEvent.Pre` |
| `input_key` | client | `InputEvent.Key` |
| `interaction_key_mapping_triggered` | client | `InputEvent.InteractionKeyMappingTriggered` |
| `is_pathfindable` | server | `IBlockExtension.isPathfindable` |
| `item_attribute_modifier` | both | `ItemAttributeModifierEvent` |
| `item_consumption_teleport` | server | `EntityTeleportEvent.ItemConsumption` |
| `item_container` | both | `Capabilities.Item.ITEM` |
| `item_crafted` | server | `PlayerEvent.ItemCraftedEvent` |
| `item_decorations` | client | `RegisterItemDecorationsEvent` |
| `item_expire` | server | `ItemExpireEvent` |
| `item_fished` | server | `ItemFishedEvent` |
| `item_smelted` | server | `PlayerEvent.ItemSmeltedEvent` |
| `item_stacked_on_other` | both | `ItemStackedOnOtherEvent` |
| `item_tooltip` | client | `ItemTooltipEvent` |
| `item_toss` | server | `ItemTossEvent` |
| `items` | both | `RegisterEvent` |
| `layer_definitions` | client | `EntityRenderersEvent.RegisterLayerDefinitions` |
| `left_click_empty` | client | `PlayerInteractEvent.LeftClickEmpty` |
| `level_load` | server | `LevelEvent.Load` |
| `level_save` | server | `LevelEvent.Save` |
| `level_unload` | server | `LevelEvent.Unload` |
| `living_breathe` | server | `LivingBreatheEvent` |
| `living_conversion_post` | server | `LivingConversionEvent.Post` |
| `living_conversion_pre` | server | `LivingConversionEvent.Pre` |
| `living_damage_post` | server | `LivingDamageEvent.Post` |
| `living_damage_pre` | server | `LivingDamageEvent.Pre` |
| `living_death` | server | `LivingDeathEvent` |
| `living_destroy_block` | server | `LivingDestroyBlockEvent` |
| `living_drops` | server | `LivingDropsEvent` |
| `living_drown` | server | `LivingDrownEvent` |
| `living_entity_use_item_finish` | server | `LivingEntityUseItemEvent.Finish` |
| `living_entity_use_item_start` | both | `LivingEntityUseItemEvent.Start` |
| `living_entity_use_item_stop` | both | `LivingEntityUseItemEvent.Stop` |
| `living_entity_use_item_tick` | both | `LivingEntityUseItemEvent.Tick` |
| `living_equipment_change` | server | `LivingEquipmentChangeEvent` |
| `living_experience_drop` | server | `LivingExperienceDropEvent` |
| `living_fall` | both | `LivingFallEvent` |
| `living_get_projectile` | both | `LivingGetProjectileEvent` |
| `living_heal` | both | `LivingHealEvent` |
| `living_incoming_damage` | server | `LivingIncomingDamageEvent` |
| `living_jump` | both | `LivingEvent.LivingJumpEvent` |
| `living_knockback` | server | `LivingKnockBackEvent` |
| `living_shield_block` | server | `LivingShieldBlockEvent` |
| `living_use_totem` | server | `LivingUseTotemEvent` |
| `living_visibility` | server | `LivingEvent.LivingVisibilityEvent` |
| `map_decorations` | client | `MapDecorationType` |
| `menu_extra_data` | server | `IPlayerExtension.openMenu` |
| `mob_despawn` | server | `MobDespawnEvent` |
| `mob_effect_applicable` | server | `MobEffectEvent.Applicable` |
| `mob_effect_expired` | server | `MobEffectEvent.Expired` |
| `mob_effect_remove` | server | `MobEffectEvent.Remove` |
| `mob_layer` | client | `EntityRenderersEvent.AddLayers` |
| `mob_spawn_placement_check` | server | `MobSpawnEvent.SpawnPlacementCheck` |
| `mob_spawn_position_check` | server | `MobSpawnEvent.PositionCheck` |
| `mob_split` | server | `MobSplitEvent` |
| `mob_submit` | client | `RenderLivingEvent.Pre` |
| `modify_custom_spawners` | server | `ModifyCustomSpawnersEvent` |
| `mouse_button_post` | client | `InputEvent.MouseButton.Post` |
| `mouse_button_pre` | client | `InputEvent.MouseButton.Pre` |
| `mouse_scrolling` | client | `InputEvent.MouseScrollingEvent` |
| `movement_input` | client | `MovementInputUpdateEvent` |
| `music` | client | `SelectMusicEvent` |
| `neighbor_notify` | server | `BlockEvent.NeighborNotifyEvent` |
| `note_block_change` | server | `NoteBlockEvent.Change` |
| `note_block_play` | server | `NoteBlockEvent.Play` |
| `permissions_changed` | server | `PermissionsChangedEvent` |
| `pickup_xp` | server | `PlayerXpEvent.PickupXp` |
| `piston_post` | server | `PistonEvent.Post` |
| `piston_pre` | server | `PistonEvent.Pre` |
| `play_level_sound_at_entity` | server | `PlayLevelSoundEvent.AtEntity` |
| `play_level_sound_at_position` | server | `PlayLevelSoundEvent.AtPosition` |
| `play_sound` | client | `PlaySoundEvent` |
| `play_sound_source` | client | `PlaySoundSourceEvent` |
| `play_streaming_source` | client | `PlayStreamingSourceEvent` |
| `player_advancement_done` | server | `AdvancementEvent.AdvancementEarnEvent` |
| `player_advancement_progress` | server | `AdvancementEvent.AdvancementProgressEvent` |
| `player_bed_leave` | server | `PlayerWakeUpEvent` |
| `player_break_speed` | both | `PlayerEvent.BreakSpeed` |
| `player_brewed_potion` | server | `PlayerBrewedPotionEvent` |
| `player_changed_dimension` | server | `PlayerEvent.PlayerChangedDimensionEvent` |
| `player_clone` | server | `PlayerEvent.Clone` |
| `player_command` | server | `CommandEvent` |
| `player_container_close` | server | `PlayerContainerEvent.Close` |
| `player_container_open` | server | `PlayerContainerEvent.Open` |
| `player_enchant_item` | server | `PlayerEnchantItemEvent` |
| `player_flyable_fall` | server | `PlayerFlyableFallEvent` |
| `player_harvest_check` | both | `PlayerEvent.HarvestCheck` |
| `player_heart_type` | client | `PlayerHeartTypeEvent` |
| `player_item_smithed` | server | `PlayerEvent.ItemSmithedEvent` |
| `player_load_from_file` | server | `PlayerEvent.LoadFromFile` |
| `player_name_format` | both | `PlayerEvent.NameFormat` |
| `player_portal` | server | `EntityTravelToDimensionEvent` |
| `player_respawn_position` | server | `PlayerRespawnPositionEvent` |
| `player_respawned` | server | `PlayerEvent.PlayerRespawnEvent` |
| `player_save_to_file` | server | `PlayerEvent.SaveToFile` |
| `player_set_spawn` | server | `PlayerSetSpawnEvent` |
| `player_spawn_phantoms` | server | `PlayerSpawnPhantomsEvent` |
| `player_start_tracking` | server | `PlayerEvent.StartTracking` |
| `player_stat_award` | server | `StatAwardEvent` |
| `player_stop_tracking` | server | `PlayerEvent.StopTracking` |
| `player_submit` | client | `RenderPlayerEvent.Pre` |
| `player_swap_hands` | server | `LivingSwapItemsEvent.Hands` |
| `player_switch_hotbar_slot_post` | server | `PlayerSwitchHotbarSlotEvent.Post` |
| `player_switch_hotbar_slot_pre` | server | `PlayerSwitchHotbarSlotEvent.Pre` |
| `player_tab_list_name_format` | server | `PlayerEvent.TabListNameFormat` |
| `player_tick_post` | both | `PlayerTickEvent.Post` |
| `player_tick_pre` | both | `PlayerTickEvent.Pre` |
| `player_xp_change` | server | `PlayerXpEvent.XpChange` |
| `player_xp_level_change` | server | `PlayerXpEvent.LevelChange` |
| `portal_spawn` | server | `BlockEvent.PortalSpawnEvent` |
| `potential_spawns` | server | `LevelEvent.PotentialSpawns` |
| `potion_brew_post` | server | `PotionBrewEvent.Post` |
| `potion_brew_pre` | server | `PotionBrewEvent.Pre` |
| `projectile_impact` | server | `ProjectileImpactEvent` |
| `recipes_received` | client | `RecipesReceivedEvent` |
| `register_client_commands` | client | `RegisterClientCommandsEvent` |
| `register_configs` | both | `ModContainer.registerConfig` |
| `register_debug_entries` | client | `RegisterDebugEntriesEvent` |
| `register_range_select_item_model_property` | client | `RegisterRangeSelectItemModelPropertyEvent` |
| `register_select_item_model_property` | client | `RegisterSelectItemModelPropertyEvent` |
| `render_arm` | client | `RenderArmEvent` |
| `render_fog` | client | `ViewportEvent.RenderFog` |
| `render_frame_post` | client | `RenderFrameEvent.Post` |
| `render_frame_pre` | client | `RenderFrameEvent.Pre` |
| `render_gui_post` | client | `RenderGuiEvent.Post` |
| `render_gui_pre` | client | `RenderGuiEvent.Pre` |
| `render_hand` | client | `RenderHandEvent` |
| `render_item_in_frame` | client | `RenderItemInFrameEvent` |
| `render_name_tag` | client | `RenderNameTagEvent` |
| `render_tooltip_gather_components` | client | `RenderTooltipEvent.GatherComponents` |
| `render_tooltip_pre` | client | `RenderTooltipEvent.Pre` |
| `right_click_block` | both | `PlayerInteractEvent.RightClickBlock` |
| `right_click_empty` | client | `PlayerInteractEvent.RightClickEmpty` |
| `right_click_item` | both | `PlayerInteractEvent.RightClickItem` |
| `screen_char_typed_post` | client | `ScreenEvent.CharacterTyped.Post` |
| `screen_char_typed_pre` | client | `ScreenEvent.CharacterTyped.Pre` |
| `screen_closing` | client | `ScreenEvent.Closing` |
| `screen_init_post` | client | `ScreenEvent.Init.Post` |
| `screen_key_pressed_post` | client | `ScreenEvent.KeyPressed.Post` |
| `screen_key_pressed_pre` | client | `ScreenEvent.KeyPressed.Pre` |
| `screen_key_released_post` | client | `ScreenEvent.KeyReleased.Post` |
| `screen_key_released_pre` | client | `ScreenEvent.KeyReleased.Pre` |
| `screen_mouse_button_pressed_post` | client | `ScreenEvent.MouseButtonPressed.Post` |
| `screen_mouse_button_pressed_pre` | client | `ScreenEvent.MouseButtonPressed.Pre` |
| `screen_mouse_button_released_post` | client | `ScreenEvent.MouseButtonReleased.Post` |
| `screen_mouse_button_released_pre` | client | `ScreenEvent.MouseButtonReleased.Pre` |
| `screen_mouse_dragged_post` | client | `ScreenEvent.MouseDragged.Post` |
| `screen_mouse_dragged_pre` | client | `ScreenEvent.MouseDragged.Pre` |
| `screen_mouse_scrolled_post` | client | `ScreenEvent.MouseScrolled.Post` |
| `screen_mouse_scrolled_pre` | client | `ScreenEvent.MouseScrolled.Pre` |
| `screen_opening` | client | `ScreenEvent.Opening` |
| `screen_preedit_post` | client | `ScreenEvent.Preedit.Post` |
| `screen_preedit_pre` | client | `ScreenEvent.Preedit.Pre` |
| `screen_render_background` | client | `ScreenEvent.Render.Background` |
| `screen_render_foreground` | client | `ScreenEvent.Render.Foreground` |
| `screen_render_inventory_mob_effects` | client | `ScreenEvent.RenderInventoryMobEffects` |
| `screen_render_post` | client | `ScreenEvent.Render.Post` |
| `screen_render_pre` | client | `ScreenEvent.Render.Pre` |
| `screen_transparent_background` | client | `ScreenEvent.Render.Pre (transparent background)` |
| `screenshot` | client | `ScreenshotEvent` |
| `server_about_to_start` | server | `ServerAboutToStartEvent` |
| `server_chat` | server | `ServerChatEvent` |
| `server_started` | server | `ServerStartedEvent` |
| `server_starting` | server | `ServerStartingEvent` |
| `server_stopped` | server | `ServerStoppedEvent` |
| `server_stopping` | server | `ServerStoppingEvent` |
| `should_check_weak_power` | server | `IBlockExtension.shouldCheckWeakPower` |
| `sleep_finished_time` | server | `SleepFinishedTimeEvent` |
| `sound_engine_load` | client | `SoundEngineLoadEvent` |
| `spread_players_teleport` | server | `EntityTeleportEvent.SpreadPlayersCommand` |
| `standalone_models` | client | `ModelEvent.RegisterStandalone` |
| `submit_custom_geometry` | client | `SubmitCustomGeometryEvent` |
| `sweep_attack` | both | `SweepAttackEvent` |
| `teleport_command` | server | `EntityTeleportEvent.TeleportCommand` |
| `toast_add` | client | `ToastAddEvent` |
| `tooltip_color` | client | `RenderTooltipEvent.Texture` |
| `trade_with_villager` | server | `TradeWithVillagerEvent` |
| `use_item_on_block` | both | `UseItemOnBlockEvent` |
| `validate_tickets` | server | `LoadingValidationCallback` |
| `vanilla_game_event` | server | `VanillaGameEvent` |
| `village_siege` | server | `VillageSiegeEvent` |
| `window_resize` | client | `WindowResizeEvent` |
| `xp_orb_targeting` | server | `XpOrbTargetingEvent` |

## Hooks that declare no counterpart

A hook with no NeoForge counterpart is judged on its merits like any other. The note is the key's own reason.

| Hook | Side | Reason |
| --- | --- | --- |
| `anvil_damaged` | server | NeoForge has no such event |
| `armor_stand_manipulate` | server | Bukkit's PlayerArmorStandManipulateEvent |
| `beacon_state_change` | server | NeoForge has no such event |
| `bell_ring` | server | NeoForge has no such event; Bukkit's BellRingEvent |
| `block_break_progress_update` | server | NeoForge has no such event |
| `block_burn` | server | NeoForge has no such event; Bukkit's BlockBurnEvent |
| `block_cook` | server | NeoForge has no such event |
| `block_damage_abort` | server | NeoForge has no such event; Bukkit's BlockDamageAbortEvent |
| `block_dispense_armor` | server | NeoForge has no such event |
| `block_fade` | server | NeoForge has no such event |
| `block_failed_dispense` | server | NeoForge has no such event |
| `block_form` | server | NeoForge has no such event |
| `block_ignite` | server | NeoForge has no such event; Bukkit's BlockIgniteEvent |
| `block_lock_check` | server | NeoForge has no such event |
| `block_receive_game` | server | NeoForge has no such event |
| `block_shear_entity` | server | NeoForge has no such event |
| `brewing_complete` | server | NeoForge has no such event |
| `brewing_fuel` | server | NeoForge has no such event |
| `brewing_start` | server | NeoForge has no such event |
| `broadcast_message` | server | NeoForge has no such event; Bukkit's BroadcastMessageEvent |
| `bucket_empty` | server | vanilla BucketItem.emptyContents; NeoForge has no matching event |
| `campfire_start` | server | NeoForge has no such event |
| `cauldron_level_changed` | server | NeoForge has no such event; Bukkit's CauldronLevelChangeEvent |
| `compost_item` | server | NeoForge has no such event |
| `container_opened` | server | NeoForge's PlayerContainerEvent.Open fires after the menu is open and is not cancellable |
| `contents_changed` | server | Container.setChanged has no bus event |
| `crafter_craft` | server | NeoForge has no such event |
| `creeper_ignite` | server | NeoForge has no such event |
| `creeper_power` | server | NeoForge has no such event |
| `dragon_egg_form` | server | NeoForge has no such event |
| `enchant_item` | server | NeoForge has no such event |
| `entities_load` | server | NeoForge has no such event |
| `entities_unload` | server | NeoForge has no such event |
| `entity_break_door` | server | NeoForge has no such event |
| `entity_breed` | server | NeoForge has no such event |
| `entity_change_block` | server | NeoForge has no such event |
| `entity_change_block_pre` | server | NeoForge has no such event; Bukkit's EntityChangeBlockEvent is cancellable |
| `entity_drop_item` | server | NeoForge has no such event; Bukkit's EntityDropItemEvent |
| `entity_enter_block` | server | NeoForge has no such event |
| `entity_enter_love_mode` | server | NeoForge has no such event |
| `entity_portal` | server | NeoForge has no such event |
| `entity_pose_change` | server | NeoForge has no such event |
| `entity_shoot_bow` | server | NeoForge has no such event |
| `entity_tame` | server | NeoForge has no such event |
| `entity_teleport` | server | NeoForge has no such event |
| `entity_toggle_glide` | server | NeoForge has no such event |
| `entity_unleash` | server | NeoForge has no such event; Bukkit's EntityUnleashEvent |
| `exp_bottle` | server | NeoForge has no such event |
| `explosion_prime` | server | NeoForge has no such event; Bukkit's ExplosionPrimeEvent |
| `firework_explode` | server | NeoForge has no such event |
| `fluid_level_change` | server | NeoForge has no such event |
| `food_level_changed` | server | vanilla: FoodData; Bukkit's FoodLevelChangeEvent |
| `framerate_limit` | client | vanilla FramerateLimitTracker.getFramerateLimit fires no event |
| `furnace_burn` | server | NeoForge has no such event |
| `furnace_extract` | server | NeoForge has no such event |
| `furnace_smelt` | server | NeoForge has no such event |
| `furnace_start_smelt` | server | NeoForge has no such event |
| `hopper_inventory_search` | server | NeoForge has no such event; Bukkit's InventoryMoveItemEvent |
| `hopper_pickup_item` | server | NeoForge has no direct equivalent; Bukkit's InventoryPickupItemEvent |
| `inventory_block_start` | server | NeoForge has no such event |
| `inventory_move_item` | server | NeoForge has no such event; Bukkit's InventoryMoveItemEvent |
| `item_damaged` | server | vanilla: ItemStack.hurtAndBreak; PlayerDestroyItemEvent is only the break |
| `item_despawn` | server | NeoForge has no such event; Bukkit's ItemDespawnEvent |
| `item_spawn` | server | NeoForge has no such event |
| `leaves_decay` | server | NeoForge has no such event |
| `lightning_strike` | server | NeoForge has no such event |
| `menu_button` | server | NeoForge has no such event; vanilla AbstractContainerMenu.clickMenuButton |
| `menu_quick_move` | server | a Java menu overrides AbstractContainerMenu.quickMoveStack; no event |
| `moisture_change` | server | NeoForge has no such event |
| `player_arm_swing` | server | NeoForge has no such event; Bukkit's PlayerAnimationEvent |
| `player_attack_cooldown_reset` | server | NeoForge has no such event |
| `player_bed_enter` | server | NeoForge has no such event; Bukkit's PlayerBedEnterEvent |
| `player_bucket_entity` | server | NeoForge has no such event |
| `player_bundle_item_select` | server | NeoForge has no such event; Bukkit's PlayerBundleItemSelect |
| `player_change_beacon_effect` | server | NeoForge has no such event |
| `player_client_loaded_world` | server | NeoForge has no such event; Bukkit's PlayerClientLoadedWorld |
| `player_deep_sleep` | server | NeoForge has no such event |
| `player_destroy_item` | server | NeoForge's PlayerDestroyItemEvent carries the original stack and a hand, which armour has not; this hook only listens and names the slot instead. Bukkit's PlayerItemBreakEvent |
| `player_edit_book` | server | NeoForge has no such event; Bukkit's PlayerEditBookEvent |
| `player_egg_throw` | server | NeoForge has no such event |
| `player_elytra_boost` | server | NeoForge has no such event |
| `player_exp_cooldown_change` | server | NeoForge has no such event |
| `player_fish` | server | NeoForge's ItemFishedEvent fires after the loot roll, in FishingHook.retrieve; Bukkit's PlayerFishEvent |
| `player_flower_pot_manipulate` | server | NeoForge has no such event |
| `player_input` | server | NeoForge has no such event; Bukkit's PlayerInputEvent |
| `player_insert_lectern_book` | server | NeoForge has no such event |
| `player_item_cooldown` | server | NeoForge has no such event; Paper's PlayerItemCooldownEvent |
| `player_item_frame_change` | server | NeoForge has no such event |
| `player_item_mend` | server | NeoForge has no such event |
| `player_jump` | server | NeoForge has no such event; Bukkit's PlayerJumpEvent |
| `player_kicked` | server | NeoForge has no such event; Bukkit's PlayerKickEvent |
| `player_launch_projectile` | server | NeoForge has no such event; Bukkit's PlayerLaunchProjectile |
| `player_lectern_page_change` | server | NeoForge has no such event |
| `player_level_change` | server | NeoForge has no such event; Bukkit's PlayerLevelChangeEvent |
| `player_login` | server | Bukkit's PlayerLoginEvent; NeoForge's PlayerNegotiationEvent is not cancellable |
| `player_loom_pattern_select` | server | NeoForge has no such event |
| `player_map_filled` | server | NeoForge has no such event |
| `player_name_entity` | server | NeoForge has no such event |
| `player_open_sign` | server | NeoForge has no such event |
| `player_pick_item` | server | NeoForge has no such event; Paper's PlayerPickItemEvent |
| `player_pickup_arrow` | server | NeoForge has no such event |
| `player_ready_arrow` | server | NeoForge has no such event; Bukkit's PlayerReadyArrow |
| `player_recipe_book_click` | server | NeoForge has no such event |
| `player_recipe_book_settings_change` | server | NeoForge has no such event; Bukkit's PlayerRecipeBookSettingsChange |
| `player_recipe_discovered` | server | NeoForge has no such event; Bukkit's PlayerRecipeDiscoverEvent |
| `player_riptide` | server | NeoForge has no such event; Bukkit's PlayerRiptideEvent |
| `player_shear_entity` | server | NeoForge has no such event; Bukkit's PlayerShearEntityEvent |
| `player_shield_disable` | server | NeoForge has no such event |
| `player_stonecutter_recipe_select` | server | NeoForge has no such event |
| `player_stop_spectating_entity` | server | NeoForge has no such event |
| `player_stop_using_item` | server | NeoForge has no such event; Bukkit's PlayerStopUsingItem |
| `player_swap_with_equipment_slot` | server | NeoForge has no such event |
| `player_take_lectern_book` | server | NeoForge has no such event |
| `player_teleport_end_gateway` | server | NeoForge has no such event |
| `player_teleported` | server | EntityTeleportEvent.TeleportCommand fires before the move and can cancel it; this hook is told after the move |
| `player_toggle_entity_age_lock` | server | NeoForge has no such event |
| `player_toggle_flight` | server | NeoForge has no flight toggle event; Bukkit's PlayerToggleFlightEvent |
| `player_toggle_sneak` | server | NeoForge has no such event; Bukkit's PlayerToggleSneakEvent |
| `player_totem` | server | NeoForge's LivingUseTotemEvent can cancel the save; this hook only listens. Bukkit's EntityResurrectEvent |
| `player_trade` | server | NeoForge has no such event |
| `player_unleash_entity` | server | NeoForge has no such event |
| `player_velocity` | server | NeoForge has no such event |
| `portal_create` | server | NeoForge has no such event |
| `potion_splash` | server | NeoForge has no such event |
| `prepare_item_craft` | server | NeoForge has no such event |
| `prepare_item_enchant` | server | NeoForge has no such event |
| `prepare_smithing` | server | NeoForge has no such event |
| `projectile_launch` | server | NeoForge has no such event |
| `raid_spawn_wave` | server | NeoForge has no such event |
| `raid_state_change` | server | NeoForge has no such event |
| `sculk_bloom` | server | NeoForge has no such event |
| `sign_changed` | server | NeoForge has no sign-editing event |
| `spawn_change` | server | NeoForge has no such event |
| `sponge_absorb` | server | NeoForge has no such event |
| `tab_complete` | server | NeoForge has no generic tab completion event |
| `target_hit` | server | NeoForge has no such event |
| `thunder_change` | server | NeoForge has no such event |
| `time_skip` | server | NeoForge has no such event |
| `trade_select` | server | NeoForge has no such event |
| `vault_change_state` | server | NeoForge has no such event |
| `vault_loot` | server | NeoForge has no such event; Bukkit's BlockDispenseLoot |
| `vehicle_create` | server | NeoForge has no such event; Bukkit's VehicleCreateEvent |
| `vehicle_damage` | server | NeoForge has no such event; Bukkit's VehicleDamageEvent |
| `vehicle_destroy` | server | NeoForge has no such event; Bukkit's VehicleDestroyEvent |
| `weather_change` | server | NeoForge has no such event |
| `whitelist_state_update` | server | NeoForge has no such event; Bukkit's WhitelistToggleEvent |
| `world_border_bounds_change_finish` | server | NeoForge has no such event; Bukkit's WorldBorderBoundsChangeFinish |
| `world_border_bounds_changed` | server | NeoForge has no such event |
| `world_border_center_changed` | server | NeoForge has no such event |
| `world_saved` | server | NeoForge has no such event |

## Hooks with no key yet

These declare no `neoforge` key. Each needs either its NeoForge name or a `none` with the reason.

| Hook | Side | NeoForge |
| --- | --- | --- |
| `affect_neighbors_after_removal` | server | not set |
| `analog_output` | server | not set |
| `apply_effect_tick` | server | not set |
| `attack_entity` | server | not set |
| `biome_colors` | client | not set |
| `block_changed` | server | not set |
| `block_entity` | both | not set |
| `block_entity_container` | both | not set |
| `block_entity_fluids` | server | not set |
| `block_entity_render_bounding_box` | client | not set |
| `block_entity_submit` | client | not set |
| `block_entity_tick` | server | not set |
| `block_entity_ticks` | server | not set |
| `block_entity_update_tag` | server | not set |
| `block_entity_view_distance` | client | not set |
| `block_highlight` | client | not set |
| `block_jump_factor` | both | not set |
| `block_render_visibility` | client | not set |
| `block_signal` | server | not set |
| `block_speed_factor` | both | not set |
| `block_spread` | server | not set |
| `block_step_on` | server | not set |
| `bounce_on` | server | not set |
| `camera_shake` | client | not set |
| `camera_type_cycle` | client | not set |
| `can_eat` | both | not set |
| `can_survive` | server | not set |
| `chunk_load` | server | not set |
| `chunk_unload` | server | not set |
| `client_disconnecting` | client | not set |
| `client_packet` | client | not set |
| `client_packet_send` | client | not set |
| `client_resources_reloaded` | client | not set |
| `client_stopping` | client | not set |
| `command` | client | not set |
| `component_types` | both | not set |
| `connects_to` | server | not set |
| `consume_effect` | server | not set |
| `creative_slot` | server | not set |
| `custom_spawner_tick` | server | not set |
| `data_pack_files` | server | not set |
| `debug_entry_lines` | client | not set |
| `dispense_item` | server | not set |
| `emissive` | client | not set |
| `enchant_effect` | server | not set |
| `entity_inside` | server | not set |
| `entity_join_level` | server | not set |
| `entity_leave_level` | server | not set |
| `entity_outline` | client | not set |
| `entity_tick_listened` | server | not set |
| `experience_orb_can_merge` | server | not set |
| `experience_orb_merge` | server | not set |
| `explosion_detonate` | server | not set |
| `fall_on` | server | not set |
| `fill_bucket` | server | not set |
| `fire_odds` | server | not set |
| `fluid_flow` | both | not set |
| `gui_render_post` | client | not set |
| `hand_drawn` | client | not set |
| `hanging_break` | server | not set |
| `hanging_place` | server | not set |
| `has_analog_output` | server | not set |
| `health_hud` | client | not set |
| `hears_projectile_impacts` | server | not set |
| `hurt_enemy` | server | not set |
| `ingredient` | both | not set |
| `ingredient_test` | both | not set |
| `inventory_tick` | server | not set |
| `is_bonemeal_success` | server | not set |
| `is_signal_source` | server | not set |
| `is_valid_bonemeal_target` | both | not set |
| `item_bar` | client | not set |
| `item_model_property` | client | not set |
| `item_model_range_property` | client | not set |
| `item_model_select_property` | client | not set |
| `item_picked_up` | server | not set |
| `key_mapping_pre` | client | not set |
| `left_click_block` | server | not set |
| `level_tick_post` | server | not set |
| `level_tick_pre` | server | not set |
| `living_equipment_listened` | server | not set |
| `living_flying_speed` | both | not set |
| `living_jump_power` | both | not set |
| `living_speed` | both | not set |
| `loot_condition` | server | not set |
| `loot_drops` | server | not set |
| `loot_function` | server | not set |
| `loot_table_load` | server | not set |
| `may_place_on` | server | not set |
| `mending_repair` | server | not set |
| `menu_background` | client | not set |
| `menu_clicked` | server | not set |
| `menu_closed` | server | not set |
| `menu_slot_rules` | both | not set |
| `minecart_pass` | server | not set |
| `minimap` | client | not set |
| `mob_effect_added` | server | not set |
| `mob_killed` | server | not set |
| `mob_spawned` | server | not set |
| `mod_container_viewers` | server | not set |
| `modify_default_components` | both | not set |
| `music_layers` | client | not set |
| `neighbor_changed` | server | not set |
| `net_from_client` | server | not set |
| `net_from_server` | client | not set |
| `on_effect_added` | server | not set |
| `on_equip` | server | not set |
| `on_item_use_first` | server | not set |
| `on_use_tick` | both | not set |
| `panel_event` | client | not set |
| `perform_bonemeal` | server | not set |
| `perform_command` | server | not set |
| `perform_console_command` | server | not set |
| `player_commands_sent` | server | not set |
| `player_died` | server | not set |
| `player_game_mode_changed` | server | not set |
| `player_logged_in` | server | not set |
| `player_logged_out` | server | not set |
| `player_render_listened` | client | not set |
| `player_sprint` | server | not set |
| `player_stays_on_ground_surface` | both | not set |
| `pre_remove_side_effects` | server | not set |
| `prepare_hopper_transfer` | server | not set |
| `random_tick` | server | not set |
| `recipe_book_categories` | both | not set |
| `recipe_property_sets` | both | not set |
| `register_commands` | server | not set |
| `release_using` | both | not set |
| `render_needs` | client | not set |
| `resource_pack_status` | server | not set |
| `screen` | both | not set |
| `screen_click` | client | not set |
| `screen_draw` | client | not set |
| `screen_exclusion_areas` | client | not set |
| `screen_tooltip` | client | not set |
| `server_data_loaded` | server | not set |
| `server_tick` | server | not set |
| `settings` | both | not set |
| `settings_click` | client | not set |
| `shader_params` | client | not set |
| `should_apply_effect_tick_this_tick` | server | not set |
| `should_redstone_wire_connect_to` | server | not set |
| `smoke_source` | server | not set |
| `state_for_placement` | server | not set |
| `texture_atlas_stitched` | client | not set |
| `tick_block` | server | not set |
| `tick_target_millis` | client | not set |
| `tnt_primed` | server | not set |
| `tooltip` | client | not set |
| `tooltip_component` | client | not set |
| `tooltip_component_draw` | client | not set |
| `update_shape` | server | not set |
| `use_animation` | client | not set |
| `use_duration` | both | not set |
| `use_item` | server | not set |
| `use_item_on` | server | not set |
| `use_on` | server | not set |
| `use_without_item` | server | not set |
| `vehicle_moved` | server | not set |
| `visuals` | client | not set |
| `world_drawn` | client | not set |
| `world_lifecycle` | server | not set |
| `zoom` | client | not set |
