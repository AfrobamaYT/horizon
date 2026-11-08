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

## Event counterparts: base block hooks (#1015)

Paper API 26.3 (paper-api `26.3.build.244-stable`), public Javadoc; Horizon cited by function name; scope = the hooks below.

| Horizon hook | Paper event / status | Evidence and porting boundary |
|---|---|---|
| `block_entity_tick` | **Gap:** none | No Paper/Bukkit event covers a block entity's server tick (`Level.tickBlockEntities`); NeoForge has no counterpart either. No key. |
| `block_entity_ticks` | **Gap:** none | A world-open query (`EntityBlock.getTicker`), asked for each block state, for whether a block entity ticks; no Paper event or API hook exists for it. No key. |
| `block_entity_update_tag` | **Gap:** none | Server-side serialisation of a block entity's client update tag (`BlockEntity.getUpdateTag`) has no Paper event. No key. |
| `block_fade` | **Covered:** [`BlockFadeEvent`][paper-BlockFadeEvent] | Both fire before a block fades, for melting and coral death, and both can be cancelled so the old state stays; the event gives the block (position and current state) and `getNewState()`, which is `to`. Paper also fires it for fire burn-out and turtle eggs, which this hook does not reach. Key added. |
| `block_failed_dispense` | **Partial:** [`BlockFailedDispenseEvent`][paper-BlockFailedDispenseEvent] | Paper fires only when the dispenser's inventory is empty, while the hook fires for any failed dispense and passes the `item` that was not dispensed; the event exposes no item. Paper can also change whether the failure effect plays, which the listen-only hook cannot express; the narrower moment and missing item also prevent coverage. No key. |
| `block_form` | **Covered:** [`BlockFormEvent`][paper-BlockFormEvent] | Both fire before a block forms from world conditions and can cancel it, with the position and the new state (`getNewState()`); snow and ice forming are the event's documented examples. Paper also covers water/lava and concrete forms, which this hook does not reach. Key added. |
| `block_ignite` | **Partial:** [`BlockIgniteEvent`][paper-BlockIgniteEvent] | Both fire server-side before ignition and can cancel it, with the target block, nullable player and cause. Paper also exposes the igniting entity and source block, which the hook does not pass; its Javadoc directs player fire placement to BlockPlaceEvent. No key. |
| `block_lock_check` | **Partial:** [`BlockLockCheckEvent`][paper-BlockLockCheckEvent] | Both check a player's container lock and can change the answer (`setResult` / `locked`). Paper additionally exposes and can replace the key item and customize denial feedback; the hook passes only the player, block and boolean result, for server guests. No key. |
| `block_receive_game` | **Partial:** [`BlockReceiveGameEvent`][paper-BlockReceiveGameEvent] | Both can suppress a sculk sensor's response to a received game event, with the receiving block and event. Paper additionally exposes the source entity, which the hook lacks, and its Javadoc documents sensors only while the hook also reaches shriekers. No key. |
| `block_shear_entity` | **Partial:** [`BlockShearEntityEvent`][paper-BlockShearEntityEvent] | Paper fires when a dispenser shears a nearby entity and is cancellable, exposing the tool and a mutable drop list; the hook fires only after the shearing effects, is listen-only, and passes neither tool nor drops. No key. |
| `block_signal` | **Gap:** none | Redstone signal computation (`getSignal`, `getDirectSignal`) has no Paper/Bukkit event; this is a query, not a moment. No key. |
| `block_spread` | **Partial:** [`BlockSpreadEvent`][paper-BlockSpreadEvent] | Paper's cancellable event covers fire spreading from a source block, matching the hook's `fire` half, with `getSource()` as `from`. It has no fluid-flow case and no `kind`, so the hook's `fluid` half is not covered by this event. No key. |
| `block_step_on` | **Gap:** none | `Block.stepOn` has no Paper/Bukkit event; general movement events do not describe a block's own step callback. No key. |
| `blocks` | **Gap:** none | Paper has no event for declaring or registering new block types at startup; this is a registration collect hook, not a runtime moment. No key. |
| `break_block` | **Partial:** [`BlockBreakEvent`][paper-BlockBreakEvent] | Both fire server-side before a player breaks a block and can cancel it or override dropped experience. Paper exposes the sampled experience and an independent `setDropItems` control; the hook starts with an optional experience override and has no independent item-drop control. No key. |
| `brewing_complete` | **Partial:** [`BrewEvent`][paper-BrewEvent] | Paper documents brewing completion, but `getContents()` still holds the pre-finalization items and the event can cancel or change the result list; the hook fires after brewing, is listen-only and passes only the stand position. Paper also exposes the remaining fuel level. No key. |
| `brewing_fuel` | **Partial:** [`BrewingStandFuelEvent`][paper-BrewingStandFuelEvent] | Paper fires before a fuel item raises the fuel level and can cancel or change the fuel power; the hook fires after the fuel was consumed and gained, is listen-only, and cannot change the amount. Same fuel item, different moment and result. No key. |
| `brewing_start` | **Partial:** [`BrewingStartEvent`][paper-BrewingStartEvent] | Both concern a stand starting to brew, but the hook fires after fuel and the timer are set and is listen-only, while the event exposes setters for the brew time. No key. |
| `broadcast_message` | **Partial:** [`BroadcastMessageEvent`][paper-BroadcastMessageEvent] | Both fire before a server broadcast, let listeners change the message and cancel it. Paper additionally exposes the recipient set, which the hook lacks; Paper notes that this event is async when fired from an async thread. No key. |
| `bucket_empty` | **Partial:** [`PlayerBucketEmptyEvent`][paper-PlayerBucketEmptyEvent] | Both concern a player emptying a bucket server-side and can cancel placement, with the player and target block. Paper additionally exposes the clicked block, face, hand and exact bucket material and can replace the resulting held item; the hook passes only the fluid, collapsing aquatic-entity buckets to water, and cannot replace the result item. No key. |

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
