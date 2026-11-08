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
| `anvil_update` | **Partial:** [`PrepareAnvilEvent`][paper-PrepareAnvilEvent] | The event fires at anvil result preparation and exposes the inventory and result, but it is not cancellable and does not expose Horizon's XP and material cost out values. Cancel and cost edits cannot both be expressed. No key. |
| `armor_hurt` | **Partial:** [`EntityDamageItemEvent`][paper-EntityDamageItemEvent] | The event covers durability damage to an item from a hit, with a damage setter and cancel, but it is generic to any damaged item and does not expose the hit's damage type/source or Horizon's armor slot grouping. Per-item cancel cannot express the hook's single cancel for all pieces. No key. |
| `arrow_nock` | **Partial:** [`PlayerReadyArrowEvent`][paper-PlayerReadyArrowEvent] | The event fires while a bow's projectile is chosen and exposes the bow, arrow and cancel; Horizon also passes the hand and `has_ammo`, and still fires when no ammunition was found, which the arrow event cannot represent. No key. |
| `baby_entity_spawn` | **Gap:** none | No Paper event in the dossier covers the bred baby's creation with both parents, the causing player and the baby kind; the listed spawn/raid events are unrelated. No key. |
| `block_drops` | **Partial:** [`BlockDropItemEvent`][paper-BlockDropItemEvent] | `BlockDropItemEvent` fires after a player breaks a block and exposes the block state, player and item entities with cancel. Horizon also serves non-player breakers and passes the tool and experience, which the event does not. No key. |
| `block_jump_factor` | **Gap:** none | Paper's supplied jump events describe an entity jumping, not reads of a block's jump factor (`Block.getJumpFactor`). No event has the hook's per-block-state moment. No key. |
| `block_speed_factor` | **Gap:** none | No Paper event covers reads of a block's speed factor; the supplied movement events concern entity outcomes, not per-block factor lookups. No key. |
| `block_grow_feature` | **Partial:** [`StructureGrowEvent`][paper-StructureGrowEvent] | `StructureGrowEvent` matches a sapling or mushroom attempting to grow and is cancellable, but it exposes a `TreeType` species and block list rather than Horizon's configurable feature key, and does not document replacing the feature. No key. |
| `bounce_on` | **Gap:** none | No Paper event covers a block's `bounceOn` response with the post-bounce movement vector; the hook's moment is inside entity movement. No key. |
| `fall_on` | **Gap:** none | No Paper event exposes the landed block state, the calculated fall damage and the requested landing writes together at the hook's moment. No key. |
| `block_tool_modification` | **Gap:** none | No Paper event covers a tool transforming a block with an old/new block state and a cancelable final-state replacement; the hook also runs for simulated tool uses. No key. |
| `use_item_on` | **Partial:** [`PlayerInteractEvent`][paper-PlayerInteractEvent] | `PlayerInteractEvent` fires on the same server right-click-block path and exposes the block, item, hand, face and clicked position, but it only cancels or allows the interaction. It has no value for Horizon's replacement answer (`TryWithEmptyHand`, `Pass`, and the other results). No key. |
| `use_without_item` | **Partial:** [`PlayerInteractEvent`][paper-PlayerInteractEvent] | The same generic right-click event covers the empty-hand path and can cancel it, but Horizon's empty-hand `useWithoutItem` answer can replace the block behaviour and is not expressible through the event. No key. |
| `bonemeal` | **Gap:** none | No Paper event in the dossier covers bone meal use on a block with the stack, player-or-dispenser cause, validity and success/cancel outcome. No key. |
| `can_continue_sleeping` | **Gap:** none | No Paper event covers the per-tick check that a sleeping player, villager or wandering trader may continue sleeping; the supplied block-build event is unrelated. No key. |
| `can_player_sleep` | **Partial:** [`PlayerBedEnterEvent`][paper-PlayerBedEnterEvent] | `PlayerBedEnterEvent` fires as a player is about to enter a bed and exposes the bed and a vanilla result, but its `BedEnterResult` is read-only and only allow/deny can be set. Horizon lets mods replace the specific problem, including `other_problem`. No key. |
| `chunk_sent` | **Gap:** none | No Paper event covers a chunk packet being sent to a specific player; the supplied chunk load, unload and populate events are world-chunk lifecycle events. No key. |

[paper-PlayerResourcePackStatusEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerResourcePackStatusEvent.html
[paper-PrepareAnvilEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/inventory/PrepareAnvilEvent.html
[paper-EntityDamageItemEvent]: https://jd.papermc.io/paper/26.3/io/papermc/paper/event/entity/EntityDamageItemEvent.html
[paper-PlayerReadyArrowEvent]: https://jd.papermc.io/paper/26.3/com/destroystokyo/paper/event/player/PlayerReadyArrowEvent.html
[paper-BlockDropItemEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/block/BlockDropItemEvent.html
[paper-StructureGrowEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/world/StructureGrowEvent.html
[paper-PlayerInteractEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerInteractEvent.html
[paper-PlayerBedEnterEvent]: https://jd.papermc.io/paper/26.3/org/bukkit/event/player/PlayerBedEnterEvent.html
