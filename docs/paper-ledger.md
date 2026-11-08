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

## Event counterparts: base-analog hooks (#1015)

Paper API 26.3 (paper-api `26.3.build.244-stable`), public Javadoc only; Horizon hooks are cited by function name.
Scope: the twenty `base-analog` hooks below.

| Horizon hook | Paper event / status | Evidence and porting boundary |
|---|---|---|
| `analog_output` | **Gap:** none | The hook answers vanilla's `BlockState.getAnalogOutputSignal` for a comparator read, a pure query whose `output` a mod may replace; no Paper event reports or replaces a block's comparator signal answer. No key. |
| `anvil_craft_post` | **Gap:** none (nearest [`PrepareAnvilEvent`][paper-PrepareAnvilEvent]) | The preparation event fires when an item enters a slot and can replace the shown result, not when the player takes the result after the levels, inputs and anvil wear are settled. No Paper event reports the completed take. No key. |
| `anvil_craft_pre` | **Gap:** none (nearest [`PrepareAnvilEvent`][paper-PrepareAnvilEvent]) | The event covers result preparation before the take, is not cancellable and does not report the take itself; Horizon's `cancel` refuses the click and leaves the result, inputs and cursor as before. No key. |
| `anvil_damaged` | **Partial:** [`AnvilDamagedEvent`][paper-AnvilDamagedEvent] | The event covers an anvil damaged from use, but fires before the change with a settable damage state and breaking flag and a cancel; Horizon fires after the block already changed from `from` to `to` and is listen-only, so it can express neither. No key. |
| `apply_effect_tick` | **Partial:** [`EntityEffectTickEvent`][paper-EntityEffectTickEvent] | Both are server-side and fire on a tick when an effect acts on an entity; the event's cancel skips that tick's application while Horizon's cleared `keep` ends the effect, and the event lacks Horizon's tick count, holder health and food. No key. |
| `armor_stand_manipulate` | **Partial:** [`PlayerArmorStandManipulateEvent`][paper-PlayerArmorStandManipulateEvent] | Same pre-move swap, retrieve or place moment and cancel, with the stand, slot and hand; the event also exposes the player's and the stand's item stacks, which Horizon's hook does not pass. No key. |
| `arrow_loose` | **Partial:** [`PlayerReadyArrowEvent`][paper-PlayerReadyArrowEvent] | The event fires while a bow is fired and the server picks an arrow and can cancel that arrow; Horizon fires at the release with the charge, lets mods change the charge, and also fires when no ammunition was found, none of which the event expresses. No key. |
| `attack_entity` | **Partial:** [`PrePlayerAttackEntityEvent`][paper-PrePlayerAttackEntityEvent] | The event fires when a player tries to attack an entity before damage logic and cancelling prevents the blow, matching Horizon's moment and `canceled`; it additionally reports `willAttack` and starts cancelled for entities vanilla will not attack. No key. |
| `beacon_state_change` | **Partial:** [`BeaconActivatedEvent`][paper-BeaconActivatedEvent], [`BeaconDeactivatedEvent`][paper-BeaconDeactivatedEvent] | Both are server-side, non-cancellable and carry the beacon block, matching Horizon's listen-only result, but activation and deactivation are separate events where one Horizon hook reports either transition with `active`. No key. |
| `bell_ring` | **Partial:** [`BellRingEvent`][paper-BellRingEvent] | The cancellable event fires as the bell is being rung, with the block and the rung entity, matching Horizon's moment, side and cancel; it also reports the rung `BlockFace` direction, which the hook does not pass. No key. |
| `block_break_progress_update` | **Covered:** [`BlockBreakProgressUpdateEvent`][paper-BlockBreakProgressUpdateEvent] | The non-cancellable server event reports the breaking entity, the block and progress from 0.0 to 1.0 (Horizon's 0–9 tenths) whenever a block break's progress changes, matching `guest_dig_feedback`'s notification and listen-only result. Key added. |
| `block_burn` | **Partial:** [`BlockBurnEvent`][paper-BlockBurnEvent] | The event fires as fire destroys a block and its cancel keeps it, matching Horizon's moment and `cancel`; it also names the igniting block, which the hook does not pass. No key. |
| `block_changed` | **Gap:** none | The hook observes every committed `Level.setBlock` change before side effects and neighbour updates; Paper has no general block-state-changed event, and NeoForge's `onBlockStateChange` also runs after the change. No key. |
| `block_cook` | **Partial:** [`BlockCookEvent`][paper-BlockCookEvent] | The event reports a block successfully cooking a source into a result, with a settable result, a recipe and a cancel; Horizon's `block_cook` is a listen-only notice after the cook finishes and passes no recipe. No key. |
| `block_damage_abort` | **Partial:** [`BlockDamageAbortEvent`][paper-BlockDamageAbortEvent] | The non-cancellable event reports a player stopping damage to a block, matching Horizon's notification after the abort; it also exposes the item in hand, which the hook does not pass. No key. |
| `block_dispense_armor` | **Partial:** [`BlockDispenseArmorEvent`][paper-BlockDispenseArmorEvent] | The event covers a dispenser equipping an item on a nearby entity and can cancel it before the equip; Horizon fires after the equip, consumption, sound and game event completed as a listen-only notice. No key. |
| `block_entity` | **Gap:** none | The hook supplies a declared block entity's initial saved data inside the write that creates it; Paper has no event for block-entity creation or its initial data. No key. |
| `block_entity_container` | **Gap:** none | The hook declares a mod block entity's item slots and menu once when the ticking table is built; Paper's inventory model has no event at that moment. No key. |
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
