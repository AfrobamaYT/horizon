# Paper and Bukkit porting ledger

This is a map for porting plugin logic to Horizon native mods, **not** a Bukkit or
Paper compatibility layer. Java plugins do not run through these APIs. Event-by-event
mappings belong in the native hook catalog; this page covers other plugin services.
The Paper/Bukkit references below are public Javadoc, not implementation sources.

| Plugin API | Horizon native-mod route and limits |
| --- | --- |
| [Permissions](https://hub.spigotmc.org/javadocs/bukkit/org/bukkit/permissions/Permissible.html) | **Partial:** `NativeCommand::op` uses vanilla operator levels 0–4, not named permission nodes or Bukkit attachments. `native_player_command` can reject a player command; implement any finer policy in the mod. The command tree and dispatch are in `crates/horizon-client/src/game/modding/native_server.rs`. |
| [Scheduler](https://hub.spigotmc.org/javadocs/bukkit/org/bukkit/scheduler/BukkitScheduler.html) | **Gap:** no general delayed, repeating or asynchronous plugin task service. `native_server_tick` provides start and end callbacks for non-paused server ticks: a mod can manage its own tick deadlines there, rather than assume Bukkit task ownership or cancellation. Block/fluid tick scheduling in `crates/horizon-world/src/level/` is for world ticks, not arbitrary tasks. |
| [PersistentDataContainer](https://hub.spigotmc.org/javadocs/bukkit/org/bukkit/persistence/PersistentDataContainer.html) | **Partial:** typed `AttachmentType<T>` with a codec persists mod-namespaced values on `Holder::Player` and `Holder::Entity`, among other holders. It is not a Bukkit data container or wire format; holders must be present, and player death copying is opt-in. See `crates/horizon-mod-runtime/src/attachments.rs` and [server attachments](modding/src/concepts/trait-mods.md#server-attachments). |
| [ServicesManager](https://hub.spigotmc.org/javadocs/bukkit/org/bukkit/plugin/ServicesManager.html) | **Gap:** no equivalent provider registry, priority selection or economy service contract. Native mods must arrange their own integration; the host and attachment APIs are not a service registry. |
| [CommandExecutor](https://hub.spigotmc.org/javadocs/bukkit/org/bukkit/command/CommandExecutor.html) and Brigadier | **Partial:** register `NativeCommand` usages through `native_register_commands`, then execute commands from any source in `native_run_mod_command` with typed source identity and arguments. Horizon builds a command tree from its supported usage syntax; this is not a general Bukkit executor or arbitrary Brigadier registration. See `crates/horizon-client/src/game/modding/native_server.rs` and [server commands](https://arismoko.github.io/horizon/reference/hook-notes.html#server-commands-and-players). |

For callback signatures, side and current limitations, use [native mods](modding/src/how-to/native-mods.md)
and the hook declarations under `crates/horizon-mod-api/src/hooks/` at the fork revision being targeted.

## Event counterparts: miscellaneous additions (#1015)

Paper API 26.3 (paper-api `26.3.build.244-stable`), public Javadoc only; Horizon hooks are cited by function name.
Scope: the twenty `misc-add` hooks below.

| Horizon hook | Paper event / status | Evidence and porting boundary |
|---|---|---|
| `add_pack_finders` | **Gap:** none (nearest supplied candidate [`PlayerResourcePackStatusEvent`][paper-PlayerResourcePackStatusEvent]) | The candidate reports a player's response to a resource-pack request, not the pack-discovery and registration moment used by `discover_mod_packs`. No Paper event exposes pack-finder registration. No key. |
| `add_server_reload_listeners` | **Gap:** none | `prepare_server_reload` registers data-load listeners before a reload; Paper has no event for registering server reload listeners at that point. No key. |
| `affect_neighbors_after_removal` | **Gap:** none | The hook runs inside level writes to choose ordered neighbour updates before a removed block's updates run; no Paper event exposes those update kinds or their order. No key. |
| `anvil_update` | **Partial:** [`PrepareAnvilEvent`][paper-PrepareAnvilEvent] | The event fires at anvil result preparation and exposes the inventory and result; its [`AnvilView`][paper-AnvilView] also exposes XP and material cost setters. It is not cancellable, unlike Horizon's cancel contract (although a null result can suppress output). No key. |
| `armor_hurt` | **Partial:** [`EntityDamageItemEvent`][paper-EntityDamageItemEvent] | The event covers durability damage to an item from a hit, with a damage setter and cancel, but it is generic to any damaged item and does not expose the hit's damage type/source or Horizon's armor slot grouping. Per-item cancel cannot express the hook's single cancel for all pieces. No key. |
| `arrow_nock` | **Partial:** [`PlayerReadyArrowEvent`][paper-PlayerReadyArrowEvent] | The event documents choosing an arrow while firing a bow and exposes the bow, arrow and per-arrow cancel; Horizon asks whether bow use may start, passes the hand and `has_ammo`, and still fires when no ammunition was found. Its single use-cancel is not the arrow-selection result. No key. |
| `baby_entity_spawn` | **Partial:** [`EntityBreedEvent`][paper-EntityBreedEvent] | The server breeding event exposes both parents, the breeder and the child and is cancellable. Unlike Horizon's pre-creation baby-kind hook, it exposes an existing child, the breeding item and experience, including an experience setter that Horizon cannot express. No key. |
| `block_drops` | **Partial:** [`BlockDropItemEvent`][paper-BlockDropItemEvent] | `BlockDropItemEvent` fires after a player breaks a block and exposes the block state, player and item entities with cancel. Horizon also serves non-player breakers and passes the tool and experience, which the event does not. No key. |
| `block_jump_factor` | **Gap:** none | Paper's supplied jump events describe an entity jumping, not reads of a block's jump factor (`Block.getJumpFactor`). No event has the hook's per-block-state moment. No key. |
| `block_speed_factor` | **Gap:** none | No Paper event covers reads of a block's speed factor; the supplied movement events concern entity outcomes, not per-block factor lookups. No key. |
| `block_grow_feature` | **Partial:** [`StructureGrowEvent`][paper-StructureGrowEvent] | `StructureGrowEvent` matches a sapling or mushroom attempting to grow and is cancellable, but it exposes a `TreeType` species and block list rather than Horizon's configurable feature key, and does not document replacing the feature. No key. |
| `bounce_on` | **Gap:** none | No Paper event covers a block's `bounceOn` response with the post-bounce movement vector; the hook's moment is inside entity movement. No key. |
| `fall_on` | **Gap:** none | No Paper event exposes the landed block state, the calculated fall damage and the requested landing writes together at the hook's moment. No key. |
| `block_tool_modification` | **Gap:** none | No Paper event covers a tool transforming a block with an old/new block state and a cancelable final-state replacement; the hook also runs for simulated tool uses. No key. |
| `use_item_on` | **Partial:** [`PlayerInteractEvent`][paper-PlayerInteractEvent] | Filtering `RIGHT_CLICK_BLOCK` identifies the server right-click-block path and exposes the block, item, hand, face and clicked position, but the event precedes the block behaviour rather than replacing `useItemOn` itself. Its interaction allow/deny controls cannot express Horizon's replacement answer (`TryWithEmptyHand`, `Pass`, and the other results). No key. |
| `use_without_item` | **Partial:** [`PlayerInteractEvent`][paper-PlayerInteractEvent] | The generic right-click-block event precedes block behaviour; Horizon's `useWithoutItem` stage can also follow an item-bearing call that returns `TryWithEmptyHand`, rather than only an empty-hand click. The event's interaction allow/deny controls cannot replace that block behaviour's answer. No key. |
| `bonemeal` | **Gap:** none (nearest [`BlockFertilizeEvent`][paper-BlockFertilizeEvent]) | The event reports resulting fertilization block changes after any `StructureGrowEvent`, rather than Horizon's pre-growth validity and success/cancel decision with the bone meal stack and player-or-dispenser-or-villager cause. No key. |
| `can_continue_sleeping` | **Gap:** none | No Paper event covers the per-tick check that a sleeping player, villager or wandering trader may continue sleeping; the supplied block-build event is unrelated. No key. |
| `can_player_sleep` | **Partial:** [`PlayerBedEnterEvent`][paper-PlayerBedEnterEvent] | `PlayerBedEnterEvent` fires as a player is about to enter a bed and exposes the bed and a vanilla result, but its `BedEnterResult` is read-only and only allow/deny can be set. Horizon lets mods replace the specific problem, including `other_problem`. No key. |
| `chunk_sent` | **Partial:** [`PlayerChunkLoadEvent`][paper-PlayerChunkLoadEvent] | The non-cancellable server packet notification identifies the player and chunk when the player receives that chunk, matching `run_chunk_watches` after the chunk packet was sent and its listen-only result; the Javadoc recommends this moment for sending client-related data such as fake entities. Key added. Not told for the far ring's columns (a Horizon server option, off by default), which a server may still send to players, so a port that sends data per received chunk misses those. No key. |

[paper-PlayerResourcePackStatusEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerResourcePackStatusEvent.html
[paper-PrepareAnvilEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/inventory/PrepareAnvilEvent.html
[paper-EntityDamageItemEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/entity/EntityDamageItemEvent.html
[paper-PlayerReadyArrowEvent]: https://jd.papermc.io/paper/26.3/com/destroystokyo/paper/event/player/PlayerReadyArrowEvent.html
[paper-BlockDropItemEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockDropItemEvent.html
[paper-StructureGrowEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/world/StructureGrowEvent.html
[paper-PlayerInteractEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerInteractEvent.html
[paper-PlayerBedEnterEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerBedEnterEvent.html
[paper-EntityBreedEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityBreedEvent.html
[paper-PlayerChunkLoadEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/packet/PlayerChunkLoadEvent.html
[paper-BlockFertilizeEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockFertilizeEvent.html
[paper-AnvilView]: https://jd.papermc.io/paper/26.3/org/bukkit/inventory/view/AnvilView.html

## Event counterparts: base-analog hooks (#1015)

Paper API 26.3 (paper-api `26.3.build.244-stable`), public Javadoc only; Horizon hooks are cited by function name.
Scope: the twenty `base-analog` hooks below.

| Horizon hook | Paper event / status | Evidence and porting boundary |
|---|---|---|
| `analog_output` | **Gap:** none | The hook answers vanilla's `BlockState.getAnalogOutputSignal` for a comparator read, a pure query whose `output` a mod may replace; no Paper event reports or replaces a block's comparator signal answer. No key. |
| `anvil_craft_post` | **Gap:** none (nearest [`PrepareAnvilEvent`][paper-PrepareAnvilEvent]) | The preparation event fires when an item enters a slot and can replace the shown result, not when the player takes the result after the levels, inputs and anvil wear are settled. No Paper event reports the completed take. No key. |
| `anvil_craft_pre` | **Gap:** none (nearest [`PrepareAnvilEvent`][paper-PrepareAnvilEvent]) | The event covers result preparation before the take, is not cancellable and does not report the take itself; Horizon's `cancel` refuses the click and leaves the result, inputs and cursor as before. No key. |
| `anvil_damaged` | **Partial:** [`AnvilDamagedEvent`][paper-AnvilDamagedEvent] | The event covers an anvil damaged from use, but fires before the change with a settable damage state and breaking flag and a cancel; Horizon fires after the block already changed from `from` to `to` and is listen-only, so it can express neither. No key. |
| `apply_effect_tick` | **Partial:** [`EntityEffectTickEvent`][paper-EntityEffectTickEvent] | Both are server-side and fire on a tick when an effect acts on an entity; the event's cancel skips that tick's application while Horizon's cleared `keep` ends the effect, and the event lacks Horizon's holder health and food. The hook is told only for a content mod's own effects, not vanilla ones such as poison, so a port that relies on Paper's event for vanilla effects loses them. No key. |
| `armor_stand_manipulate` | **Partial:** [`PlayerArmorStandManipulateEvent`][paper-PlayerArmorStandManipulateEvent] | Same pre-move swap, retrieve or place moment and cancel, with the player, stand and equipment slot; the event also exposes the interacting hand and both item stacks, which Horizon's hook does not pass. No key. |
| `arrow_loose` | **Partial:** [`PlayerReadyArrowEvent`][paper-PlayerReadyArrowEvent] | The event fires while a bow is fired and the server picks an arrow and can cancel that arrow; Horizon fires at the release with the charge, lets mods change the charge, and also fires when no ammunition was found, none of which the event expresses. No key. |
| `attack_entity` | **Partial:** [`PrePlayerAttackEntityEvent`][paper-PrePlayerAttackEntityEvent] | The event fires when a player tries to attack an entity before damage logic and cancelling prevents the blow, matching Horizon's moment and `canceled`; it additionally reports `willAttack` and starts cancelled for entities vanilla will not attack. No key. |
| `beacon_state_change` | **Partial:** [`BeaconActivatedEvent`][paper-BeaconActivatedEvent], [`BeaconDeactivatedEvent`][paper-BeaconDeactivatedEvent] | Both are server-side, non-cancellable and carry the beacon block, but activation is documented when the beam becomes visible and deactivation includes destruction of the beacon itself. Horizon's `tick_beacon_at` reports pyramid-level zero/nonzero transitions after checking the beam; it does not report all those lifecycle moments, and no single event covers both transitions. No key. |
| `bell_ring` | **Partial:** [`BellRingEvent`][paper-BellRingEvent] | The cancellable event fires as the bell is being rung, with the block and the rung entity, matching Horizon's moment, side and cancel; it also reports the rung `BlockFace` direction, which the hook does not pass. No key. |
| `block_break_progress_update` | **Partial:** [`BlockBreakProgressUpdateEvent`][paper-BlockBreakProgressUpdateEvent] | Both are non-cancellable server notifications of block-breaking progress, with the block and breaker; Paper exposes float progress from 0.0 to 1.0 and an entity, while `guest_dig_feedback` passes integer animation stages for a server guest. The public Javadoc does not establish an equivalent stage conversion or limit the breaker to players. No key. |
| `block_burn` | **Partial:** [`BlockBurnEvent`][paper-BlockBurnEvent] | The event fires as fire destroys a block and its cancel keeps it, matching Horizon's moment and `cancel`; it also names the igniting block, which the hook does not pass. No key. |
| `block_changed` | **Gap:** none | The hook observes every committed `Level.setBlock` change before side effects and neighbour updates; Paper has no general block-state-changed event, and NeoForge's `onBlockStateChange` also runs after the change. No key. |
| `block_cook` | **Partial:** [`BlockCookEvent`][paper-BlockCookEvent] | The event reports a block successfully cooking a source into a result, with a settable result, a recipe and a cancel; Horizon's `block_cook` is a listen-only notice after the cook finishes and passes no recipe. No key. |
| `block_damage_abort` | **Partial:** [`BlockDamageAbortEvent`][paper-BlockDamageAbortEvent] | The non-cancellable event reports a player stopping damage to a block, matching Horizon's notification after the abort; it also exposes the item in hand, which the hook does not pass. No key. |
| `block_dispense_armor` | **Partial:** [`BlockDispenseArmorEvent`][paper-BlockDispenseArmorEvent] | The event covers a dispenser equipping an item on a nearby entity and can cancel it before the equip; Horizon fires after the equip, consumption, sound and game event completed as a listen-only notice. No key. |
| `block_entity` | **Gap:** none | The hook supplies a declared block entity's initial saved data inside the write that creates it; Paper has no event for block-entity creation or its initial data. No key. |
| `block_entity_container` | **Gap:** none | The hook declares a mod block entity's item slots and menu when `native_containers` builds the ticking table, and is also queried by `open_native_screen`; Paper's inventory model has no event for this declaration. No key. |
| `block_entity_energy` | **Gap:** none | The hook declares a mod block entity's energy buffer once when the ticking table is built; Paper has no block-entity energy capability or event for it. No key. |
| `block_entity_fluids` | **Gap:** none (nearest [`FluidLevelChangeEvent`][paper-FluidLevelChangeEvent]) | The nearest event reports a world fluid block's level changing from adjacent blocks; the hook declares a mod block entity's fluid tanks when tables are built, a different moment and subject. No key. |

[paper-PrepareAnvilEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/inventory/PrepareAnvilEvent.html
[paper-AnvilDamagedEvent]: https://jd.papermc.io/paper/26.3/com/destroystokyo/paper/event/block/AnvilDamagedEvent.html
[paper-EntityEffectTickEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/entity/EntityEffectTickEvent.html
[paper-PlayerArmorStandManipulateEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerArmorStandManipulateEvent.html
[paper-PlayerReadyArrowEvent]: https://jd.papermc.io/paper/26.3/com/destroystokyo/paper/event/player/PlayerReadyArrowEvent.html
[paper-PrePlayerAttackEntityEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/player/PrePlayerAttackEntityEvent.html
[paper-BeaconActivatedEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/block/BeaconActivatedEvent.html
[paper-BeaconDeactivatedEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/block/BeaconDeactivatedEvent.html
[paper-BellRingEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BellRingEvent.html
[paper-BlockBreakProgressUpdateEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/block/BlockBreakProgressUpdateEvent.html
[paper-BlockBurnEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockBurnEvent.html
[paper-BlockCookEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockCookEvent.html
[paper-BlockDamageAbortEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockDamageAbortEvent.html
[paper-BlockDispenseArmorEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockDispenseArmorEvent.html
[paper-FluidLevelChangeEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/FluidLevelChangeEvent.html

## Event counterparts: base block hooks (#1015)

Paper API 26.3 (paper-api `26.3.build.244-stable`), public Javadoc; Horizon cited by function name; scope = the hooks below.

| Horizon hook | Paper event / status | Evidence and porting boundary |
|---|---|---|
| `block_entity_tick` | **Gap:** none | No Paper/Bukkit event covers a block entity's server tick (`Level.tickBlockEntities`); NeoForge has no counterpart either. No key. |
| `block_entity_ticks` | **Gap:** none | A world-open query (`EntityBlock.getTicker`), asked for each block state, for whether a block entity ticks; no Paper event or API hook exists for it. No key. |
| `block_entity_update_tag` | **Gap:** none | Server-side serialisation of a block entity's client update tag (`BlockEntity.getUpdateTag`) has no Paper event. No key. |
| `block_fade` | **Partial:** [`BlockFadeEvent`][paper-BlockFadeEvent] | Both fire before a block fades, for melting and coral death, and both can be cancelled so the old state stays; the event gives the block (position and current state) and `getNewState()`, which is `to`. Paper also fires it for fire burn-out and turtle eggs, which this hook does not reach, so a plugin relying on those cases loses them. No key. |
| `block_failed_dispense` | **Partial:** [`BlockFailedDispenseEvent`][paper-BlockFailedDispenseEvent] | Paper fires only when the dispenser's inventory is empty, while the hook fires for any failed dispense and passes the `item` that was not dispensed; the event exposes no item. Paper can also change whether the failure effect plays, which the listen-only hook cannot express; the narrower moment and missing item also prevent coverage. No key. |
| `block_form` | **Partial:** [`BlockFormEvent`][paper-BlockFormEvent] | Both fire before a block forms from world conditions and can cancel it, with the position and the new state (`getNewState()`); snow and ice forming are the event's documented examples. Paper also covers water/lava and concrete forms, which this hook does not reach, so a plugin relying on those cases loses them. No key. |
| `block_ignite` | **Partial:** [`BlockIgniteEvent`][paper-BlockIgniteEvent] | Both fire server-side before ignition and can cancel it, with the target block, nullable player and cause. Paper also exposes the igniting entity and source block, which the hook does not pass; its Javadoc directs player fire placement to BlockPlaceEvent. No key. |
| `block_lock_check` | **Partial:** [`BlockLockCheckEvent`][paper-BlockLockCheckEvent] | Both check a player's container lock and can change the answer (`setResult` / `locked`). Paper additionally exposes and can replace the key item and customize denial feedback; the hook passes only the player, block and boolean result, for server guests. No key. |
| `block_receive_game` | **Partial:** [`BlockReceiveGameEvent`][paper-BlockReceiveGameEvent] | Both can suppress a sculk sensor's response to a received game event, with the receiving block and event. Paper additionally exposes the source entity, which the hook lacks, and its Javadoc documents sensors only while the hook also reaches shriekers. No key. |
| `block_shear_entity` | **Partial:** [`BlockShearEntityEvent`][paper-BlockShearEntityEvent] | Paper fires when a dispenser shears a nearby entity and is cancellable, exposing the tool and a mutable drop list; the hook fires only after the shearing effects, is listen-only, and passes neither tool nor drops. No key. |
| `block_signal` | **Gap:** none | Redstone signal computation (`getSignal`, `getDirectSignal`) has no Paper/Bukkit event; this is a query, not a moment. No key. |
| `block_spread` | **Partial:** [`BlockSpreadEvent`][paper-BlockSpreadEvent] | Paper's cancellable event covers fire spreading from a source block, matching the hook's `fire` half, with `getSource()` as `from`. It has no fluid-flow case and no `kind`, so the hook's `fluid` half is not covered by this event; Paper's cancellable [`BlockFromToEvent`][paper-BlockFromToEvent] covers water and lava flow with source and destination blocks, but has no `kind` either. No key. |
| `block_step_on` | **Gap:** none | `Block.stepOn` has no Paper/Bukkit event; general movement events do not describe a block's own step callback. No key. |
| `blocks` | **Gap:** none | Paper has no event for declaring or registering new block types at startup; this is a registration collect hook, not a runtime moment. No key. |
| `break_block` | **Partial:** [`BlockBreakEvent`][paper-BlockBreakEvent] | Both fire server-side before a player breaks a block and can cancel it or override dropped experience. Paper exposes the sampled experience and an independent `setDropItems` control; the hook starts with an optional experience override and has no independent item-drop control. No key. |
| `brewing_complete` | **Partial:** [`BrewEvent`][paper-BrewEvent] | Paper documents brewing completion, but `getContents()` still holds the pre-finalization items and the event can cancel or change the result list; the hook fires after brewing, is listen-only and passes only the stand position. Paper also exposes the remaining fuel level. No key. |
| `brewing_fuel` | **Partial:** [`BrewingStandFuelEvent`][paper-BrewingStandFuelEvent] | Paper fires before a fuel item raises the fuel level and can cancel or change the fuel power; the hook fires after the fuel was consumed and gained, is listen-only, and cannot change the amount. Same fuel item, different moment and result. No key. |
| `brewing_start` | **Partial:** [`BrewingStartEvent`][paper-BrewingStartEvent] | Both concern a stand starting to brew, but the hook fires after fuel and the timer are set and is listen-only, while the event exposes setters for the brew time. No key. |
| `broadcast_message` | **Partial:** [`BroadcastMessageEvent`][paper-BroadcastMessageEvent] | Both fire before a server broadcast, let listeners change the message and cancel it. Paper additionally exposes the recipient set, which the hook lacks; Paper notes that this event is async when fired from an async thread. No key. |
| `bucket_empty` | **Partial:** [`PlayerBucketEmptyEvent`][paper-PlayerBucketEmptyEvent] | Both concern a player emptying a bucket server-side and can cancel placement, with the player and target block. Paper additionally exposes the clicked block, face, hand and exact bucket material and can replace the resulting held item; the hook passes only the fluid, collapsing aquatic-entity buckets to water, and cannot replace the result item. The hook is told only for guests of a server, not for the local player in single player or on a LAN host. No key. |

[paper-BlockFadeEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockFadeEvent.html
[paper-BlockFailedDispenseEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/block/BlockFailedDispenseEvent.html
[paper-BlockFormEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockFormEvent.html
[paper-BlockIgniteEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockIgniteEvent.html
[paper-BlockLockCheckEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/block/BlockLockCheckEvent.html
[paper-BlockReceiveGameEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockReceiveGameEvent.html
[paper-BlockShearEntityEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockShearEntityEvent.html
[paper-BlockSpreadEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockSpreadEvent.html
[paper-BlockBreakEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockBreakEvent.html
[paper-BrewingStandFuelEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/inventory/BrewingStandFuelEvent.html
[paper-BrewingStartEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BrewingStartEvent.html
[paper-BroadcastMessageEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/server/BroadcastMessageEvent.html
[paper-PlayerBucketEmptyEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerBucketEmptyEvent.html

[paper-BrewEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/inventory/BrewEvent.html
[paper-BlockFromToEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockFromToEvent.html

## Event counterparts: base campfire hooks (#1015)

Paper API 26.3 (paper-api `26.3.build.244-stable`), public Javadoc; Horizon cited by function name; scope is the 20 base-campfire hooks below.
A key is added only where moment, side, information and result all agree; otherwise the row records the mismatch.

| Horizon hook | Paper event / status | Evidence and porting boundary |
|---|---|---|
| `campfire_start` | **Partial:** [`CampfireStartEvent`][paper-CampfireStartEvent] | Both fire server-side when an item on a campfire begins cooking, with its block and source item exposed; Paper also exposes the recipe and lets plugins set total cook time, while the hook only listens with position and item ID. No key. |
| `can_eat` | **Gap:** none (nearest [`BlockCanBuildEvent`][paper-BlockCanBuildEvent]) | The supplied candidate decides block placement, not whether a player can eat a food stack; no Paper event exposes the vanilla `canEat` answer for the hook's server half. No key. |
| `can_survive` | **Partial:** [`BlockCanBuildEvent`][paper-BlockCanBuildEvent] | The placement check and buildable setter match the hook's placement case, but Paper does not fire this for the shape-update path and the event is not the general `canSurvive` question. No key. |
| `cauldron_level_changed` | **Partial:** [`CauldronLevelChangeEvent`][paper-CauldronLevelChangeEvent] | Both fire server-side before a cauldron level change and allow cancellation, but Paper exposes an acting entity rather than only an optional player and also permits changing the new level, which the hook cannot express. Read old/new block states rather than the deprecated level accessors. No key. |
| `chunk_load` | **Covered:** [`ChunkLoadEvent`][paper-ChunkLoadEvent] | Both fire server-side when a chunk is loaded; `isNewChunk()` matches the hook's `new` flag and the chunk/world identify dimension and x/z. Non-cancellable, matching the listen-only hook. Key added. |
| `chunk_unload` | **Partial:** [`ChunkUnloadEvent`][paper-ChunkUnloadEvent] | Both notify server-side chunk unloading with chunk/world coordinates, but Paper exposes and lets plugins change whether the chunk is saved; the hook only listens after its save copy has been made. No key. |
| `component_types` | **Gap:** none | Registering namespaced NBT component types is loader/registration work, not a Paper event moment; no supplied Javadoc event covers it. No key. |
| `compost_item` | **Partial:** [`EntityCompostItemEvent`][paper-EntityCompostItemEvent] | The entity/player compost event exposes the item, but it fires before composting while the hook fires after `composter_add`; it is also cancellable, which the listen-only hook cannot express. No key. |
| `consume_effect` | **Gap:** none | No Paper event covers a mod-defined `ConsumeEffect.apply` carrying an arbitrary kind and JSON; the supplied potion events only cover potion effects. No key. |
| `container_opened` | **Partial:** [`InventoryOpenEvent`][paper-InventoryOpenEvent] | Both concern server-side inventory opening and allow cancellation, but Paper also permits a title override that the hook cannot express, and its Javadoc does not establish the hook's timing before the previous menu closes or its restriction to game-owned menus. No key. |
| `crafter_craft` | **Partial:** [`CrafterCraftEvent`][paper-CrafterCraftEvent] | Same crafter/recipe subject on the server, but Paper fires before the craft and is cancellable/settable, whereas the hook fires after a completed craft and only listens. No key. |
| `creative_slot` | **Partial:** [`InventoryCreativeEvent`][paper-InventoryCreativeEvent] | Both cover the server-side creative inventory slot action with player, slot, item and count and allow cancellation, but Paper also lets plugins replace the cursor stack; the hook cannot change the submitted item or count. No key. |
| `creeper_ignite` | **Partial:** [`CreeperIgniteEvent`][paper-CreeperIgniteEvent] | The ignition moment and creeper agree, but the event carries no igniting player or item and is cancellable, while the hook passes both and only listens. No key. |
| `creeper_power` | **Partial:** [`CreeperPowerEvent`][paper-CreeperPowerEvent] | The lightning-charges-creeper moment, creeper and strike location agree, but the event is cancellable and the listen-only hook has no cancel out. No key. |
| `critical_hit` | **Gap:** none | No supplied event covers the melee `Player.attack` critical-hit decision with vanilla/critical/multiplier facts; the candidates are projectile or target-block hits. No key. |
| `data_map_types` | **Gap:** none | Registering data-map types while server data tables are assembled has no Paper event in the dossier; the supplied map events concern map items. No key. |
| `data_pack_files` | **Gap:** none | No Paper event supplies arbitrary data-pack files during server resource assembly; the resource-pack status candidate is client-facing and unrelated. No key. |
| `datapack_sync` | **Gap:** none | No Paper event asks a mod which recipe types to sync to clients; the dossier has no datapack-sync counterpart. No key. |
| `difficulty_changed` | **Partial:** [`WorldDifficultyChangeEvent`][paper-WorldDifficultyChangeEvent] | Both are server-side, non-cancellable notifications after a difficulty change, but Paper exposes only the new difficulty and is per world, while the hook passes `from` and `to` server-wide. No key. |
| `dispense_item` | **Partial:** [`BlockDispenseEvent`][paper-BlockDispenseEvent] | Both concern a dispenser using an item, but Paper fires at dispense time with cancel/item/velocity controls, not at `getDispenseMethod` where the hook replaces the behaviour, remainder and level event. No key. |

[paper-CampfireStartEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/CampfireStartEvent.html
[paper-BlockCanBuildEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockCanBuildEvent.html
[paper-CauldronLevelChangeEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/CauldronLevelChangeEvent.html
[paper-ChunkLoadEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/world/ChunkLoadEvent.html
[paper-ChunkUnloadEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/world/ChunkUnloadEvent.html
[paper-EntityCompostItemEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/entity/EntityCompostItemEvent.html
[paper-CrafterCraftEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/CrafterCraftEvent.html
[paper-InventoryCreativeEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/inventory/InventoryCreativeEvent.html
[paper-CreeperIgniteEvent]: https://jd.papermc.io/paper/26.3/com/destroystokyo/paper/event/entity/CreeperIgniteEvent.html
[paper-CreeperPowerEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/CreeperPowerEvent.html
[paper-WorldDifficultyChangeEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/world/WorldDifficultyChangeEvent.html
[paper-BlockDispenseEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockDispenseEvent.html

[paper-InventoryOpenEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/inventory/InventoryOpenEvent.html

## Event counterparts: base world, entity and dragon server hooks (#1015)

Paper API 26.3 (paper-api `26.3.build.244-stable`), public Javadoc only; Horizon hooks are cited by function
name and scope is the base/dragon server hooks below. Rows without a key differ in moment, side, information or result.

| Horizon hook | Paper event / status | Evidence and porting boundary |
|---|---|---|
| `dragon_egg_form` | **Covered:** [`DragonEggFormEvent`][paper-DragonEggFormEvent] | The same before-placement moment: the dragon is defeated and its first egg is about to appear, and `isCancelled()` decides whether the block is placed, exactly like the hook's `cancel`. Server-side, and the `DragonBattle` handle identifies the fight whose egg is forming. Key added. |
| `enchant_effect` | **Gap:** none (nearest [`EntityPotionEffectEvent`][paper-EntityPotionEffectEvent]) | The hook reports a mod-declared `post_attack` enchantment entry on a hit mob (enchantment id, kind, effect JSON, level, victim), which no Paper event describes. The potion-effect events cover effects being applied or ticking, not enchantment entries. No key. |
| `enchant_item` | **Partial:** [`EnchantItemEvent`][paper-EnchantItemEvent] | Same enchant-table action, with the enchanter and block, but `getItem()` is the item to be enchanted and `getExpLevelCost()` is the displayed minimum level, not the hook's finished item and levels charged. Cancellation and outcome setters also have no equivalent in the listen-only hook. No key. |
| `ender_pearl_teleport` | **Partial:** [`PlayerTeleportEvent`][paper-PlayerTeleportEvent] | The pearl teleport is a `PlayerTeleportEvent` (told apart by `getCause()`), before the teleport like the hook, and the player and from/to come through `PlayerMoveEvent`. The pearl entity and the fall-damage amount are missing, so `pearl` and `damage` can neither be read nor changed. No key. |
| `entities_load` | **Partial:** [`EntitiesLoadEvent`][paper-EntitiesLoadEvent] | Both report server chunk entity loading without cancellation, and the chunk supplies dimension and coordinates. The hook counts saved records read in `spawn_loaded_entities`, including records skipped or retained without a live entity; `getEntities()` lists loaded entities, not saved records. No key. |
| `entities_unload` | **Partial:** [`EntitiesUnloadEvent`][paper-EntitiesUnloadEvent] | Both report server chunk entity unloading without cancellation. The hook counts serialized records queued by `unload_entities` (passengers can be nested in a root record), whereas `getEntities()` lists unloaded entities; its size is not proof of the same record count. No key. |
| `entity_break_door` | **Partial:** [`EntityBreakDoorEvent`][paper-EntityBreakDoorEvent] | Same entity and door, but the event is cancellable (cancelling delays the break) and is raised as the door goes, while the hook only hears after the block was removed and cannot express that cancel. No key. |
| `entity_breed` | **Partial:** [`EntityBreedEvent`][paper-EntityBreedEvent] | Both parents and the child are exposed for breeding that creates a baby, but the hook also reports egg-laying with no child. The event is cancellable and can change experience, while the hook listens after `breed` returned and has no equivalent outputs. No key. |
| `entity_change_block` | **Partial:** [`EntityChangeBlockEvent`][paper-EntityChangeBlockEvent] | The event describes an entity changing a block, exposing the entity, block and proposed block data. It is cancellable while the hook listens after the state was applied; its generic coverage also excludes changes with a more specific event. No key. |
| `entity_drop_item` | **Partial:** [`EntityDropItemEvent`][paper-EntityDropItemEvent] | Fires when an entity creates an item drop, exposing the entity and dropped `Item` (stack, amount, location), corresponding to `item`, `count` and `pos`. The Javadoc does not establish that the item entity does not yet exist; cancellation has no equivalent in the listen-only hook. No key. |
| `entity_enter_block` | **Partial:** [`EntityEnterBlockEvent`][paper-EntityEnterBlockEvent] | The same bee-into-hive moment with the entity and `getBlock()`. The event is cancellable before storage, while the hook only hears after `bee_enter_hive` stored and removed the mob, so its cancel has no `out`. No key. |
| `entity_enter_love_mode` | **Partial:** [`EntityEnterLoveModeEvent`][paper-EntityEnterLoveModeEvent] | Both identify the animal and feeding human for entering love mode. Paper additionally exposes and can change ticks in love and permits cancellation, while the hook passes no duration and listens after love mode was set. No key. |
| `entity_interact` | **Covered:** [`PlayerInteractEntityEvent`][paper-PlayerInteractEntityEvent] | The same moment (right-click on an entity before its own interaction), once per hand and server-side; the player, `getRightClicked()` entity and `getHand()` carry the hook's facts, with the stack read from that hand. Cancelling is the hook's `claimed`. Key added. |
| `entity_join_level` | **Partial:** [`EntityAddToWorldEvent`][paper-EntityAddToWorldEvent] | Called when an entity is added to a world for any reason, including chunk loading, with entity and world handles. It is non-cancellable and exposes no fresh-versus-disk flag, while the hook permits rejecting the addition and passes `loaded_from_disk`. No key. |
| `entity_leave_level` | **Partial:** [`EntityRemoveFromWorldEvent`][paper-EntityRemoveFromWorldEvent] | Both are non-cancellable server-side notifications that an entity left a world for any reason, including chunk unloading; the entity supplies ID and type, and Paper's `getWorld()` supplies the world being left. The hook is told only for the kinds `entity_join_level` is told (not the displays, markers and interactions Horizon keeps without simulating), and when the game next notices the entity gone rather than at the removal itself, so a port that relies on Paper's event for those entities or that exact moment loses them. No key. |
| `entity_mount` | **Partial:** [`EntityMountEvent`][paper-EntityMountEvent] | The cancellable attempt to mount exposes rider and vehicle, matching the mounting half of the hook. It does not describe dismounts or expose the hook's `mounting` direction; a separate `EntityDismountEvent` is required for getting off. No single-event key. No key. |
| `entity_portal` | **Partial:** [`EntityPortalEvent`][paper-EntityPortalEvent] | The event is called before a non-player entity teleports through a portal and exposes from/to locations with cancellation and destination setters. The hook fires in `send_through_portal` after the entity was removed and an arrival queued, passing only the source dimension, entity ID and portal block position; it cannot express those outcomes. No key. |
| `entity_pose_change` | **Partial:** [`EntityPoseChangeEvent`][paper-EntityPoseChangeEvent] | Called when the pose changes and non-cancellable, matching side and result. `getPose()` returns only the new pose, so the hook's `from` is not exposed; the hook also covers guest players only. No key. |
| `entity_shoot_bow` | **Partial:** [`EntityShootBowEvent`][paper-EntityShootBowEvent] | Both identify the shooter and projectile, but Paper documents `getForce()` as up to 1.0 whereas `release_bow` passes power multiplied by 3 and `shoot_crossbow` passes projectile speed. Paper also exposes weapon and ammunition and permits cancellation and projectile replacement; the hook only listens after the projectile entered the level. No key. |
| `entity_tame` | **Partial:** [`EntityTameEvent`][paper-EntityTameEvent] | The same taming, with the tamed `LivingEntity` and `getOwner()` matching the hook's entity and `owner`. The event is cancellable before the tame is applied, while the hook only hears after the owner and persistence were set, so the cancel has no `out`. No key. |

[paper-DragonEggFormEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/block/DragonEggFormEvent.html
[paper-EntityPotionEffectEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityPotionEffectEvent.html
[paper-EnchantItemEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/enchantment/EnchantItemEvent.html
[paper-PlayerTeleportEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerTeleportEvent.html
[paper-EntitiesLoadEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/world/EntitiesLoadEvent.html
[paper-EntitiesUnloadEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/world/EntitiesUnloadEvent.html
[paper-EntityBreakDoorEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityBreakDoorEvent.html
[paper-EntityBreedEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityBreedEvent.html
[paper-EntityDropItemEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityDropItemEvent.html
[paper-EntityEnterBlockEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityEnterBlockEvent.html
[paper-EntityEnterLoveModeEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityEnterLoveModeEvent.html
[paper-PlayerInteractEntityEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerInteractEntityEvent.html
[paper-PlayerJoinEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerJoinEvent.html
[paper-PlayerBedLeaveEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerBedLeaveEvent.html
[paper-EntityMountEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityMountEvent.html
[paper-EntityPortalEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityPortalEvent.html
[paper-EntityPoseChangeEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityPoseChangeEvent.html
[paper-EntityShootBowEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityShootBowEvent.html
[paper-EntityTameEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityTameEvent.html
[paper-EntityChangeBlockEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityChangeBlockEvent.html
[paper-EntityAddToWorldEvent]: https://jd.papermc.io/paper/26.3/com/destroystokyo/paper/event/entity/EntityAddToWorldEvent.html
[paper-EntityRemoveFromWorldEvent]: https://jd.papermc.io/paper/26.3/com/destroystokyo/paper/event/entity/EntityRemoveFromWorldEvent.html

## Event counterparts: base entity hooks (#1015)

Paper API 26.3 (paper-api `26.3.build.244-stable`), public Javadoc only, cited by event name; Horizon hooks are cited by function name.
Scope: the 20 base-entity hooks below, comparing each Javadoc moment, side, exposed facts and cancel/setter behaviour against the hook's contract.

| Horizon hook | Paper event / status | Evidence and porting boundary |
|---|---|---|
| `entity_teleport` | **Partial:** [`PlayerTeleportEvent`][paper-PlayerTeleportEvent] | A player command teleport carries the same from/to coordinates and a `getCause()` field, but the event is players-only, fires before the move and is cancellable, while the hook reports every server entity after its same-level command teleport and is listen-only. No key. |
| `entity_tick_listened` | **Gap:** none | No Paper/Bukkit event describes whether a mod listens to entity ticks; the hook is Horizon's opt-in gate asked before the tick round and has no game moment to map. No key. |
| `entity_tick_post` | **Gap:** none | The Paper API has no per-entity tick event and none of the candidates mark the after-tick moment for mobs, items, minecarts or boats. No key. |
| `entity_tick_pre` | **Gap:** none | For the same reason no event marks an entity about to tick, so the hook's pre-tick cancel (no AI, physics, ageing or merging) has no counterpart. No key. |
| `entity_toggle_glide` | **Partial:** [`EntityToggleGlideEvent`][paper-EntityToggleGlideEvent] | Same server-side gliding toggle for a player and the `isGliding()` fact, but the event fires before the toggle is applied and is cancellable, while the hook reports a guest's change after it was applied and is listen-only. No key. |
| `entity_unleash` | **Partial:** [`EntityUnleashEvent`][paper-EntityUnleashEvent] | Same entity and server-side unleash, but the event is "immediately prior to" it and cancellable with `isDropLeash()`/`setDropLeash()`, while the hook fires after the leash is removed (before the lead drop is handled) and cannot cancel or choose the drop. No key. |
| `exp_bottle` | **Partial:** [`ExpBottleEvent`][paper-ExpBottleEvent] | Same thrown bottle and the experience it releases, but the event fires when the bottle hits and is cancellable with `setExperience()`/`setShowEffect()`, while the hook reports the break after the experience was awarded and cannot change it. No key. |
| `experience_orb_can_merge` | **Partial:** [`ExperienceOrbMergeEvent`][paper-ExperienceOrbMergeEvent] | The event marks the server about to merge two orbs, after vanilla's answer, while the hook is asked earlier for every pair whose boxes meet and lets a mod answer either way, so it can force a merge the event never fires for; the spawning orb (`from` is 0) is not an `ExperienceOrb` the event could pass. No key. |
| `experience_orb_merge` | **Partial:** [`ExperienceOrbMergeEvent`][paper-ExperienceOrbMergeEvent] | For two existing orbs the moment (about to merge), the target/source orbs and the cancel match the hook's `into`, `from` and `canceled`, but the hook also merges a spawning orb that is not an entity yet (`from` is 0, only `from_value` known), which an event exposing two orbs cannot describe. No key. |
| `explosion_detonate` | **Partial:** [`EntityExplodeEvent`][paper-EntityExplodeEvent] (block blasts: [`BlockExplodeEvent`][paper-BlockExplodeEvent]) | The server-side block-interaction moment and mutable `blockList()` are related to the hook's positions to spare, but Paper splits entity and block sources and documents no event when mob griefing disables block interaction, while the hook hears every server explosion. Paper also exposes cancellation and a drop-yield setter that the hook cannot express; its public Javadoc does not establish that cancellation prevents entity damage. No key. |
| `explosion_prime` | **Partial:** [`ExplosionPrimeEvent`][paper-ExplosionPrimeEvent] | Same moment (an explosion about to begin) and `getRadius()` gives the radius, but the event is documented for an entity's decision to explode and is cancellable with radius and fire setters, while the hook hears every server explosion and is listen-only. No key. |
| `fill_bucket` | **Partial:** [`PlayerBucketFillEvent`][paper-PlayerBucketFillEvent] / [`PlayerBucketEmptyEvent`][paper-PlayerBucketEmptyEvent] | The hook fires at one point in `BucketItem.use` for both filling and emptying, and Paper matches each half (same moment, player's bucket use, cancellable); but no single event covers the hook, because filling and emptying are separate events. No key. |
| `fire_odds` | **Gap:** none (nearest [`BlockSpreadEvent`][paper-BlockSpreadEvent]) | The hook replaces a level's flammability table, a lookup rather than a game moment, and `BlockSpreadEvent` only fires when a block actually spreads and can only cancel that spread; it exposes no ignite or burn odds. No key. |
| `firework_explode` | **Partial:** [`FireworkExplodeEvent`][paper-FireworkExplodeEvent] | Same rocket and position via `getEntity()`, but the event fires when the firework explodes and is cancellable, while the hook reports the explosion after its effects and damage have run and cannot cancel. No key. |
| `fluid_level_change` | **Partial:** [`FluidLevelChangeEvent`][paper-FluidLevelChangeEvent] | Same server-side fluid-level change, with the block's current data and `getNewData()` carrying the fluid type and levels, and cancellation keeping the old level. However, `setNewData()` can change the resulting block data, while the hook can only cancel and cannot change `to`. No key. |
| `food_level_changed` | **Partial:** [`FoodLevelChangeEvent`][paper-FoodLevelChangeEvent] | Same server-side player food change, with a mutable resultant food level and cancellation, but Paper's `getItem()` identifies the triggering consumed item, which the hook does not pass. The hook also combines a player turn's changes into one notification, rather than identifying an individual triggering change. No key. |
| `furnace_burn` | **Partial:** [`FurnaceBurnEvent`][paper-FurnaceBurnEvent] | Same block and fuel item, but the event fires before the fuel is burned and is cancellable with burn-time and burning setters, while the hook reports after the furnace consumed the fuel and began or resumed burning and is listen-only. No key. |
| `furnace_extract` | **Partial:** [`FurnaceExtractEvent`][paper-FurnaceExtractEvent] | Same server-side moment when a player takes output, and `getPlayer()`, the block, `getItemStack()` and `getItemAmount()` carry the hook's player, pos, item and count. However, the inherited `setExpToDrop()` changes the experience awarded after the event, while the hook is listen-only and exposes no experience amount or output. No key. |
| `furnace_smelt` | **Partial:** [`FurnaceSmeltEvent`][paper-FurnaceSmeltEvent] | Same block, input and result, but the event fires before the result is written and can be cancelled, while the hook reports after the output grew and cannot change it. No key. |
| `furnace_start_smelt` | **Partial:** [`FurnaceStartSmeltEvent`][paper-FurnaceStartSmeltEvent] | Same server-side start of smelting, with the block and inherited `getSource()` carrying the position and input item. However, Paper exposes the recipe and total cook time and lets a plugin change that time with `setTotalCookTime()`, while the hook reports that the cooking timer advanced from zero and is listen-only. No key. |

[paper-EntityToggleGlideEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityToggleGlideEvent.html
[paper-EntityUnleashEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityUnleashEvent.html
[paper-ExpBottleEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/ExpBottleEvent.html
[paper-ExperienceOrbMergeEvent]: https://jd.papermc.io/paper/26.3/com/destroystokyo/paper/event/entity/ExperienceOrbMergeEvent.html
[paper-EntityExplodeEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityExplodeEvent.html
[paper-BlockExplodeEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockExplodeEvent.html
[paper-ExplosionPrimeEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/ExplosionPrimeEvent.html
[paper-PlayerBucketFillEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerBucketFillEvent.html
[paper-PlayerBucketEmptyEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerBucketEmptyEvent.html
[paper-BlockSpreadEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockSpreadEvent.html
[paper-FireworkExplodeEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/FireworkExplodeEvent.html
[paper-FluidLevelChangeEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/FluidLevelChangeEvent.html
[paper-FoodLevelChangeEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/FoodLevelChangeEvent.html
[paper-FurnaceBurnEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/inventory/FurnaceBurnEvent.html
[paper-FurnaceExtractEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/inventory/FurnaceExtractEvent.html
[paper-FurnaceSmeltEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/inventory/FurnaceSmeltEvent.html
[paper-FurnaceStartSmeltEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/inventory/FurnaceStartSmeltEvent.html

## Event counterparts: base hooks (hanging, hopper, inventory, item, block state) (#1015)

Paper API 26.3 (paper-api `26.3.build.244-stable`), public Javadoc only; Horizon hooks are cited by function name. Scope: the 20 `base.rs` hooks below.
A key exists only where the moment, side, facts and outcome all agree; otherwise the row says which part differs.

| Horizon hook | Paper event / status | Evidence and porting boundary |
|---|---|---|
| `hanging_break` | **Partial:** [`HangingBreakEvent`][paper-HangingBreakEvent] | The hanging entity exposes its kind and position, and cancellation prevents removal, but Paper also reports `getCause()` (and the remover in `HangingBreakByEntityEvent`); `hanging_break` supplies only an optional player, so blast and lost support cannot be distinguished. No key. |
| `hanging_place` | **Partial:** [`HangingPlaceEvent`][paper-HangingPlaceEvent] | Same server placement moment and cancellation, with the player and hanging entity supplying kind and position; Paper additionally exposes the clicked block/face, hand and originating item stack, which `hanging_place` does not pass. No key. |
| `has_analog_output` | **Gap:** none | No Paper/Bukkit event is consulted when a comparator reads a block's analog output; the hook is a per-state virtual call with no event counterpart. No key. |
| `hears_projectile_impacts` | **Gap:** none (nearest [`ProjectileHitEvent`][paper-ProjectileHitEvent]) | The hook is a Horizon-only opt-in gate asked once per tick (whether any mod wants impact events), not an in-world moment; `ProjectileHitEvent` is the impact itself and fires regardless of listeners. No key. |
| `hopper_inventory_search` | **Partial:** [`HopperInventorySearchEvent`][paper-HopperInventorySearchEvent] | Same server search with the hopper and selected container, but Paper fires on each search attempt (including no inventory) and `setInventory()` changes the inventory the hopper will use; `hopper_inventory_search` only reports a selected target and cannot replace it. No key. |
| `hopper_pickup_item` | **Partial:** [`InventoryPickupItemEvent`][paper-InventoryPickupItemEvent] | Fired when a hopper or hopper minecart takes a dropped item, cancellable; the `Item` entity gives the stack's item and count plus its position, the inventory is the hopper (filter minecarts out if needed), and cancelling leaves the stack in the world as the hook's `cancel` does. The hook is documented only as "a hopper at `pos`"; Paper's event also fires for hopper minecarts, which the hook's declaration does not say it covers, so a port that relies on the event for minecarts must not assume it. No key. |
| `hurt_enemy` | **Gap:** none (nearest [`EntityDamageEvent`][paper-EntityDamageEvent]) | The generic damage event describes damage before it is applied, not the landed-hit `Item.hurtEnemy` callback before post-attack enchantments and weapon wear. Its mutable damage and cancellation also have no counterpart in this listen-only hook. No key. |
| `ingredient` | **Gap:** none | No Paper/Bukkit event resolves a custom recipe ingredient kind to item ids while a data load parses recipes; recipes are registered through the API after load, with no pre-parse hook. No key. |
| `inventory_block_start` | **Partial:** [`InventoryOpenEvent`][paper-InventoryOpenEvent] | Same server inventory opening and cancellation for block-held inventories, with player and block location available; Paper additionally lets a listener change the title sent in the open packet through `titleOverride(Component)`, which this cancel-only hook cannot express. No key. |
| `inventory_move_item` | **Partial:** [`InventoryMoveItemEvent`][paper-InventoryMoveItemEvent] | Same server transfer attempt with source, destination and item, and cancellation returns or leaves it at the source; Paper exposes the transferred stack and `setItem()` can replace it, whereas `inventory_move_item` reports only the item id for a one-item transfer and cannot change it. No key. |
| `inventory_tick` | **Gap:** none | No Paper/Bukkit event ticks each carried stack: the hook is `Item.inventoryTick` per slot per tick, with no event exposing a slot tick or letting a listener change the stack in place. No key. |
| `is_bonemeal_success` | **Gap:** none | No event asks a block whether this bone meal use grows it (the random roll inside `isBonemealSuccess`); Paper has no counterpart for the per-block success decision. No key. |
| `is_signal_source` | **Gap:** none | No event asks whether a block state is a redstone signal source; it is a per-state virtual the level consults when wire connects, with no Paper/Bukkit equivalent. No key. |
| `is_valid_bonemeal_target` | **Gap:** none | No event asks, before bone meal is used, whether a block accepts it; the `TargetHitEvent`, `EntityTargetEvent` and `SlimeTargetLivingEntityEvent` candidates match on the word "target" only and describe unrelated moments. No key. |
| `item_consumption_teleport` | **Partial:** [`PlayerTeleportEvent`][paper-PlayerTeleportEvent] | The chorus-fruit teleport is the same moment (`TeleportCause.CHORUS_FRUIT`), the from/to pair is the parent `PlayerMoveEvent`'s and can be changed, and cancel refuses it; but the hook fires per candidate draw before the ground check (up to 16) and its cancel ends the whole effect, while Paper fires for the teleport itself and its cancel only refuses that teleport. No key. |
| `item_crafted` | **Partial:** [`ItemCraftedEvent`][paper-ItemCraftedEvent] | Same moment (a player takes the result from a crafting grid, not a stonecutter, smithing table or furnace), server side and listen-only, but the event gives only the player and the crafted stack: the recipe id the hook passes is missing. No key. |
| `item_damaged` | **Partial:** [`EntityDamageItemEvent`][paper-EntityDamageItemEvent] | Same server durability-wear operation with entity, item, mutable damage and cancellation, but Paper exposes the damaged stack while `item_damaged` passes only its item id; the public Javadoc also does not establish that this generic event is after Unbreaking, as the hook requires. No key. |
| `item_despawn` | **Partial:** [`ItemDespawnEvent`][paper-ItemDespawnEvent] | Same 6000-tick moment with the item entity, its stack and location, but the hook only hears while cancelling the event grants 5 more minutes: a listen-only hook cannot express that cancel (the separate `item_expire` hook extends life by ticks). No key. |
| `item_expire` | **Partial:** [`ItemDespawnEvent`][paper-ItemDespawnEvent] | Same lifetime moment, but the hook lets each mod set or add an arbitrary `extra_life` in ticks (capped at 32766) while the event offers only a cancel that extends by a fixed 5 minutes, so the hook's out cannot be expressed. No key. |
| `item_picked_up` | **Partial:** [`EntityPickupItemEvent`][paper-EntityPickupItemEvent] | Fired when a living entity picks an item off the ground, cancellable; the `Item` entity gives the stack's item and amount, `getRemaining()` the part left, and the entity is the player (filter out non-player pickups the hook does not hear), with cancel leaving the stack on the ground. The hook is told only for a player taking an item (`ItemEntity.playerTouch`) and, once cancelled, does not offer the stack again for 10 ticks; Paper's event also covers every other living entity that picks items up, so a port that relies on mob pickups loses them. No key. |

[paper-HangingBreakEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/hanging/HangingBreakEvent.html
[paper-HangingPlaceEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/hanging/HangingPlaceEvent.html
[paper-ProjectileHitEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/ProjectileHitEvent.html
[paper-HopperInventorySearchEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/inventory/HopperInventorySearchEvent.html
[paper-InventoryPickupItemEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/inventory/InventoryPickupItemEvent.html
[paper-EntityDamageEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityDamageEvent.html
[paper-InventoryOpenEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/inventory/InventoryOpenEvent.html
[paper-InventoryMoveItemEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/inventory/InventoryMoveItemEvent.html
[paper-PlayerTeleportEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerTeleportEvent.html
[paper-ItemCraftedEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/inventory/ItemCraftedEvent.html
[paper-ItemDespawnEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/ItemDespawnEvent.html
[paper-EntityPickupItemEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityPickupItemEvent.html

## Event counterparts: base item hooks (#1015)

Paper API 26.3 (paper-api `26.3.build.244-stable`), public Javadoc only: no Paper server source is used and no Bukkit compatibility layer is proposed. Horizon hooks are cited by function name; scope is the 20 `base-item` hooks below.

| Horizon hook | Paper event / status | Evidence and porting boundary |
|---|---|---|
| `item_spawn` | **Gap:** none (nearest [`ItemSpawnEvent`][paper-ItemSpawnEvent]) | Paper describes a cancellable spawn whose cancellation prevents the item from spawning, whereas `item_spawn` listens only after the item was successfully added. Its item entity supplies the stack and position, but the before/after boundary differs. No key. |
| `items` | **Gap:** none cited | Item declarations are collected once at registration; no cited Paper event fires when content is registered or offers the declared items as an out-value. No key. |
| `left_click_block` | **Partial:** [`PlayerInteractEvent`][paper-PlayerInteractEvent] | Action.LEFT_CLICK_BLOCK narrows the server interaction to a left click on a block, with the clicked block and face. Paper additionally exposes the item and hand and separate block-use and item-use results, whereas left_click_block passes block state, position and face with one cancel flag; its Javadoc also does not establish the hook's precise post-guard dig-start ordering. No key. |
| `level_load` | **Partial:** [`WorldLoadEvent`][paper-WorldLoadEvent] | Paper announces a loaded world on the server and exposes that world, but its Javadoc does not establish Horizon's ordering before the first chunk or its lazy dimension-open lifecycle. No key. |
| `level_tick_post` | **Gap:** none cited | No cited Paper event marks the end of one dimension's tick pass, and the cited set has no per-level tick event at all, so Horizon's per-dimension ordering cannot be expressed. No key. |
| `level_tick_pre` | **Gap:** none cited | As `level_tick_post`: the cited set has no per-level tick event, so the start of a dimension's tick pass is not observable through the cited events. No key. |
| `level_unload` | **Gap:** none (nearest [`WorldUnloadEvent`][paper-WorldUnloadEvent]) | Paper exposes the unloading world but is cancellable; it does not document the hook's already-closed, after-last-save notification. No key. |
| `lightning_strike` | **Partial:** [`LightningStrikeEvent`][paper-LightningStrikeEvent] | Both describe a server lightning strike with cancellation, position and cause; the lightning entity also exposes its causing entity. Paper distinguishes SPAWNER, TRAP and ENCHANTMENT as well as COMMAND, CUSTOM, TRIDENT and WEATHER, so its cause information cannot all be expressed by Horizon's weather/trident/command/mod values. No key. |
| `living_breathe` | **Gap:** none (nearest [`EntityAirChangeEvent`][paper-EntityAirChangeEvent]) | Paper fires when remaining air changes, with a settable amount and cancellation; it does not describe the per-tick breathing decision with separate mutable can_breathe, consume and refill inputs. No key. |
| `living_drown` | **Gap:** none (nearest [`EntityDamageEvent`][paper-EntityDamageEvent]) | Paper's drowning damage event concerns the damage application, rather than the air tick deciding whether to drown; it lacks the hook's is_drowning flag and bubble count. No key. |
| `living_entity_use_item_finish` | **Gap:** none (nearest [`PlayerItemConsumeEvent`][paper-PlayerItemConsumeEvent]) | Paper fires while a player is finishing consumption, before effects and item removal, and can cancel them or set a replacement. Horizon fires after finishUsingItem and exposes the actual resulting stack, also for non-consumable timed uses; the moments differ. No key. |
| `living_entity_use_item_start` | **Gap:** none (nearest [`PlayerInteractEvent`][paper-PlayerInteractEvent]) | A right-click interaction does not establish that LivingEntity.startUsingItem has been reached: interactions can perform other actions or never start a timed use. It also lacks the mutable duration and mob uses. No key. |
| `living_entity_use_item_stop` | **Partial:** [`PlayerStopUsingItemEvent`][paper-PlayerStopUsingItemEvent] | Paper detects a player releasing an item and exposes the item and elapsed ticks. It is non-cancellable, lacks the hand and remaining duration, and cannot express Horizon's cancellation of the release outcome or its mob uses. No key. |
| `living_entity_use_item_tick` | **Gap:** none cited | No cited Paper event fires on every tick of an item use or lets a listener move the remaining duration; the cited click events are a different moment. No key. |
| `living_equipment_change` | **Partial:** [`EntityEquipmentChangedEvent`][paper-EntityEquipmentChangedEvent] | Paper fires after an equipment change is detected, server-side, for a living entity, and its changed-slot map carries each slot's old and new stack, matching the hook's `from`/`to`; it is listen-only (non-cancellable) like the hook. Paper batches the slots where Horizon reports one per slot. The hook hears mobs and armor stands only (players are heard by `native_on_equip`), so a port that relies on Paper's event for players loses them. No key. |
| `living_equipment_listened` | **Gap:** none cited | It is a Horizon-only opt-in gate asked once a tick, not a game moment; no cited event expresses it, and the equipment events it gates are unrelated to the gate itself. No key. |
| `living_fall` | **Gap:** none (nearest [`EntityDamageEvent`][paper-EntityDamageEvent]) | Paper's damage event fires when fall damage lands, after fall handling, and exposes neither the fall distance nor the damage multiplier the hook lets mods change; the both-side pre-damage moment has no cited event. No key. |
| `living_heal` | **Partial:** [`EntityRegainHealthEvent`][paper-EntityRegainHealthEvent] | Paper exposes the healing entity, mutable amount and cancellation on the server, matching the server half of living_heal. It also supplies a regain reason and fast-regeneration flag which Horizon does not pass; its Javadoc does not specify the hook's precise pre-application ordering. No key. |
| `living_incoming_damage` | **Partial:** [`EntityDamageEvent`][paper-EntityDamageEvent] | The hit moment, server side, cancellation and a settable damage amount match, but Horizon also passes and lets mods change the invulnerability ticks a fresh hit leaves and passes the attacker's weapon, neither of which the event exposes. No key. |
| `living_jump` | **Partial:** [`EntityJumpEvent`][paper-EntityJumpEvent] | `EntityJumpEvent` detects a living entity's jump server-side and identifies the entity, but it is cancellable (a listen-only hook cannot express its cancel) and fires at the jump decision rather than after the velocity write; `HorseJumpEvent` and `PlayerJumpEvent` are narrower. No key. |

[paper-LightningStrikeEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/weather/LightningStrikeEvent.html
[paper-EntityEquipmentChangedEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/entity/EntityEquipmentChangedEvent.html
[paper-EntityJumpEvent]: https://jd.papermc.io/paper/26.3/com/destroystokyo/paper/event/entity/EntityJumpEvent.html
[paper-ItemSpawnEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/ItemSpawnEvent.html
[paper-WorldLoadEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/world/WorldLoadEvent.html
[paper-WorldUnloadEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/world/WorldUnloadEvent.html
[paper-EntityAirChangeEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityAirChangeEvent.html
[paper-PlayerItemConsumeEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerItemConsumeEvent.html
[paper-PlayerStopUsingItemEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/player/PlayerStopUsingItemEvent.html
[paper-EntityRegainHealthEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityRegainHealthEvent.html

## Event counterparts: base/living hooks (#1015)

Paper API 26.3 (paper-api `26.3.build.244-stable`), public Javadoc; Horizon cited by function name; scope = the hooks below.
Decisions follow the documented moment, side, information and result; only Covered rows get a `paper` key.

| Horizon hook | Paper event / status | Evidence and porting boundary |
|---|---|---|
| `living_knockback` | **Partial:** [`EntityKnockbackEvent`][paper-EntityKnockbackEvent] | The server-side knockback moment and cancellable vector cover the push, but the event exposes no damage source, attacker id or held main/off stacks. No key. |
| `loot_condition` | **Gap:** none (nearest [`LootGenerateEvent`][paper-LootGenerateEvent]) | That event fires when a loot table generates and exposes rolled loot, not when a mod-defined condition type is evaluated; there is no condition kind, JSON or `holds` result. No key. |
| `loot_drops` | **Partial:** [`BlockDropItemEvent`][paper-BlockDropItemEvent] | It matches only player block breaks and the item list, while the hook also covers mob deaths, explosion/mob breaks, player-use and leash drops and passes table, position, entity, player, tool and explosion context. No key. |
| `loot_function` | **Gap:** none (nearest [`LootGenerateEvent`][paper-LootGenerateEvent]) | The nearest event fires after table generation; it has no per-function type, JSON, context or mutable stack at the function's evaluation moment. No key. |
| `loot_table_load` | **Gap:** none (nearest [`LootGenerateEvent`][paper-LootGenerateEvent]) | The hook runs as table JSON loads from data packs, before anything rolls it; `LootGenerateEvent` runs when a table generates in the world and offers generated loot, not the raw table document. No key. |
| `may_place_on` | **Gap:** none (nearest [`BlockCanBuildEvent`][paper-BlockCanBuildEvent]) | `BlockCanBuildEvent` asks whether a player can place a block at a location; the hook is the server-side `mayPlaceOn` survival answer for a plant state on a ground state, with no player, hand or placement. No key. |
| `mending_repair` | **Partial:** [`PlayerItemMendEvent`][paper-PlayerItemMendEvent] | The server-side pre-repair event exposes the selected item and mutable repair amount, but also an experience orb, equipment slot, cancellation and a durability-to-XP operation that the hook cannot express; the hook also handles directly granted experience. No key. |
| `menu_button` | **Gap:** none | No cited event covers a mod container's `clickMenuButton`, including the out-of-world block entity and the mutable carried stack. No key. |
| `menu_clicked` | **Partial:** [`InventoryClickEvent`][paper-InventoryClickEvent] | The server inventory-click event exposes the player, view, slot and click type, but runs within inventory modification and can cancel or change its outcome; `menu_clicked` only reports clicks that moved nothing after they are queued. No key. |
| `menu_closed` | **Partial:** [`InventoryCloseEvent`][paper-InventoryCloseEvent] | The non-cancellable server close notification exposes the player and inventory view, with documented PLAYER, OPEN_NEW, DISCONNECT and PLUGIN reasons matching the close causes. It does not expose the protocol window id passed by `menu_closed`, and its reason is absent from the hook. No key. |
| `menu_quick_move` | **Partial:** [`InventoryClickEvent`][paper-InventoryClickEvent] | `isShiftClick()` identifies server shift-clicks, and cancellation plus manual inventory changes can replace their outcome, but this is the inventory-click transaction rather than the menu's `quickMoveStack` routing call with a mutable out-of-world mod block entity and remaining stack. No key. |
| `menu_slot_rules` | **Gap:** none | No cited event covers `Slot.mayPlace` or per-slot maximum rules for registered vanilla and mod menus; the both-side hook has no server event counterpart here. No key. |
| `mob_effect_added` | **Gap:** none (nearest [`EntityPotionEffectEvent`][paper-EntityPotionEffectEvent]) | The Paper event precedes a potion-effect modification and can cancel it or change override behavior; `mob_effect_added` reports every addEffect call for a mod effect, changed or unchanged, after the start callback. Its ADDED/CHANGED actions do not establish that post-call, unchanged-inclusive moment. No key. |
| `mob_killed` | **Partial:** [`EntityDeathEvent`][paper-EntityDeathEvent] | Death of the living entity is the same after-the-blow moment, and the damage source can identify its causing player, but the event covers all deaths, exposes additional damage/drop/experience information, and is cancellable while the hook cannot cancel. No key. |
| `mob_spawned` | **Partial:** [`EntityAddToWorldEvent`][paper-EntityAddToWorldEvent] | The non-cancellable server world-add notification exposes the entity, world and position, but includes chunk loading and other additions excluded by the hook and has no spawn reason or `natural` flag. [`CreatureSpawnEvent`][paper-CreatureSpawnEvent] offers a spawn reason but is cancellable before spawning, whereas the hook reports a mob already added. No key. |
| `mod_container_viewers` | **Gap:** none | No cited event carries the aggregate viewer count and `previous`/`viewers` transition for a mod container; per-player open/close events would not supply it. No key. |
| `modify_default_components` | **Gap:** none | No cited Paper event modifies per-item default component maps at data-load/registry-freeze time. No key. |
| `moisture_change` | **Covered:** [`MoistureChangeEvent`][paper-MoistureChangeEvent] | The cancellable soil-moisture change is the same server-side before-change moment; the block's current state and `getNewState()` give from/to, and cancelling keeps the old moisture. Key added. |
| `neighbor_changed` | **Gap:** none | The hook is the per-block `neighborChanged` delivery; the dossier's player/world/equipment candidates are unrelated, and its own note says `NeighborNotifyEvent` is a different broadcast. No key. |
| `net_from_client` | **Gap:** none | No cited event carries a player's custom channel payload at the hook's tick-delayed receive point; the listed entity/block events are unrelated. No key. |

[paper-EntityKnockbackEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/entity/EntityKnockbackEvent.html
[paper-LootGenerateEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/world/LootGenerateEvent.html
[paper-BlockCanBuildEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockCanBuildEvent.html
[paper-EntityPotionEffectEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityPotionEffectEvent.html
[paper-EntityDeathEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/EntityDeathEvent.html
[paper-CreatureSpawnEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/entity/CreatureSpawnEvent.html
[paper-MoistureChangeEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/MoistureChangeEvent.html

[paper-PlayerItemMendEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerItemMendEvent.html
[paper-InventoryClickEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/inventory/InventoryClickEvent.html
[paper-InventoryCloseEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/inventory/InventoryCloseEvent.html
[paper-EntityAddToWorldEvent]: https://jd.papermc.io/paper/26.3/com/destroystokyo/paper/event/entity/EntityAddToWorldEvent.html

## Event counterparts: base-on hooks (#1015)

Paper API 26.3 (paper-api `26.3.build.244-stable`), public Javadoc; Horizon cited by function name; scope = the 20 `base-on` hooks below.
Status is judged on moment, side, information and result; “No key” means no `paper` key is added.

| Horizon hook | Paper event / status | Evidence and porting boundary |
|---|---|---|
| `on_effect_added` | **Partial:** [`EntityPotionEffectEvent`][paper-EntityPotionEffectEvent] | The event is cancellable before the modification, while the hook is listen-only after `addEffect`; it does expose the entity and effect/amplifier facts. No key. |
| `on_equip` | **Partial:** [`EntityEquipmentChangedEvent`][paper-EntityEquipmentChangedEvent] | Non-cancellable and exposes each slot's old/new items through EquipmentChange, but detects changes after attribute/enchantment updates, including durability and login, rather than specifically before the equip sound. No key. |
| `on_use_tick` | **Gap:** none | No Paper event was provided for each tick of an in-progress item use with remaining ticks; interaction events fire on the initial click, not the use tick. No key. |
| `perform_bonemeal` | **Gap:** none | No Paper event for `BonemealableBlock.performBonemeal` was provided or cited; the vanilla growth moment has no counterpart in the dossier. No key. |
| `pickup_xp` | **Partial:** [`PlayerPickupExperienceEvent`][paper-PlayerPickupExperienceEvent] | Same attempt-to-pick-up moment and cancellation, and it exposes the orb, but it has no setters for the hook's `value` and `count` outputs. No key. |
| `piston_post` | **Gap:** none (nearest [`BlockPistonExtendEvent`][paper-BlockPistonExtendEvent] / [`BlockPistonRetractEvent`][paper-BlockPistonRetractEvent]) | Paper's piston events are the cancellable pre-move events; the hook fires after movement/destruction, so the post moment is uncovered. No key. |
| `piston_pre` | **Partial:** [`BlockPistonExtendEvent`][paper-BlockPistonExtendEvent] / [`BlockPistonRetractEvent`][paper-BlockPistonRetractEvent] | Cancellable pre-move events match the moment and side, but their `getBlocks()` describes moved blocks and does not expose the hook's separate `destroy` list. No key. |
| `player_advancement_done` | **Partial:** [`PlayerAdvancementDoneEvent`][paper-PlayerAdvancementDoneEvent] | Matches server-side completion and exposes the player and advancement, but message(Component) changes the announcement; the listen-only hook has no corresponding output. No key. |
| `player_advancement_progress` | **Partial:** [`PlayerAdvancementCriterionGrantEvent`][paper-PlayerAdvancementCriterionGrantEvent] | Covers the post-progress grant branch and exposes progress/criterion, but not revocation, and it is cancellable while the hook is listen-only. No key. |
| `player_arm_swing` | **Partial:** [`PlayerArmSwingEvent`][paper-PlayerArmSwingEvent] | Exposes the hand and is server-side, but it is cancellable (the hook only listens) and is the swing event rather than the hook's after-handling notification. No key. |
| `player_attack_cooldown_reset` | **Partial:** [`PlayerAttackEntityCooldownResetEvent`][paper-PlayerAttackEntityCooldownResetEvent] | Only fires for an attack on an entity, while the hook fires for every main-hand swing reset (including mining and clicking nothing); it is also deprecated and cancellable. No key. |
| `player_bed_enter` | **Partial:** [`PlayerBedEnterEvent`][paper-PlayerBedEnterEvent] | Matches the server-side pre-entry attempt and cancellation, but enterAction() exposes the default outcome and setUseBed(ALLOW) can force sleep despite it; the hook fires only after checks pass and can only cancel. No key. |
| `player_bed_leave` | **Partial:** [`PlayerBedLeaveEvent`][paper-PlayerBedLeaveEvent] | Same leaving-bed moment and bed information, but the event is cancellable (and can change spawn-setting behaviour) while the hook is listen-only. No key. |
| `player_bucket_entity` | **Partial:** [`PlayerBucketEntityEvent`][paper-PlayerBucketEntityEvent] | Exposes the captured entity and buckets server-side, but it is cancellable before the capture, while the hook is listen-only after the mob is removed and the bucket given. No key. |
| `player_bundle_item_select` | **Partial:** [`PlayerBundleItemSelectEvent`][paper-PlayerBundleItemSelectEvent] | Matches server-side non-cancellable selection and exposes player, slot and selected index, but also carries the previous index and previous/selected items, which the hook does not pass. No key. |
| `player_change_beacon_effect` | **Partial:** [`PlayerChangeBeaconEffectEvent`][paper-PlayerChangeBeaconEffectEvent] | Carries the beacon and primary/secondary effects, but it is cancellable with setters and fires when the player sets the effect, while the hook only listens after the effects are stored. No key. |
| `player_changed_dimension` | **Partial:** [`PlayerChangedWorldEvent`][paper-PlayerChangedWorldEvent] | Non-cancellable server-side notification when the player switches worlds; getFrom() and the player's current world supply the source and destination corresponding to the hook's dimension IDs. The hook is heard only on a dedicated server (not on an integrated or LAN-host server), not for a move that ends where it started, and not for a player who leaves before the move ends, while Paper's event fires on any server, so a port that relies on Paper's event in those cases loses them. No key. |
| `player_client_loaded_world` | **Partial:** [`PlayerClientLoadedWorldEvent`][paper-PlayerClientLoadedWorldEvent] | The non-cancellable client-notification branch matches, but Paper also fires after a 60-tick timeout and exposes isTimeout(); the hook only handles the client report and passes no timeout fact. No key. |
| `player_clone` | **Gap:** none | No Paper event for copying player data to the new player after death or End return was provided; no cited event covers the moment. No key. |
| `player_command` | **Partial:** [`PlayerCommandPreprocessEvent`][paper-PlayerCommandPreprocessEvent] | Matches the early server-side player-command moment and cancellation, with the player and command text, but setMessage(String) can rewrite the command while the hook has no command output. No key. |

[paper-EntityEquipmentChangedEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/entity/EntityEquipmentChangedEvent.html
[paper-PlayerPickupExperienceEvent]: https://jd.papermc.io/paper/26.3/com/destroystokyo/paper/event/player/PlayerPickupExperienceEvent.html
[paper-BlockPistonExtendEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockPistonExtendEvent.html
[paper-BlockPistonRetractEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockPistonRetractEvent.html
[paper-PlayerAdvancementDoneEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerAdvancementDoneEvent.html
[paper-PlayerAdvancementCriterionGrantEvent]: https://jd.papermc.io/paper/26.3/com/destroystokyo/paper/event/player/PlayerAdvancementCriterionGrantEvent.html
[paper-PlayerArmSwingEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/player/PlayerArmSwingEvent.html
[paper-PlayerAttackEntityCooldownResetEvent]: https://jd.papermc.io/paper/26.3/com/destroystokyo/paper/event/player/PlayerAttackEntityCooldownResetEvent.html
[paper-PlayerBedLeaveEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerBedLeaveEvent.html
[paper-PlayerBucketEntityEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerBucketEntityEvent.html
[paper-PlayerBundleItemSelectEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/inventory/PlayerBundleItemSelectEvent.html
[paper-PlayerChangeBeaconEffectEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/player/PlayerChangeBeaconEffectEvent.html
[paper-PlayerPortalEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerPortalEvent.html
[paper-PlayerClientLoadedWorldEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/player/PlayerClientLoadedWorldEvent.html
[paper-ServerCommandEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/server/ServerCommandEvent.html
[paper-UnknownCommandEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/command/UnknownCommandEvent.html
[paper-PlayerCommandSendEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerCommandSendEvent.html

[paper-PlayerChangedWorldEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerChangedWorldEvent.html

[paper-PlayerCommandPreprocessEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerCommandPreprocessEvent.html
