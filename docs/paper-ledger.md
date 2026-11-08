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

## Event counterparts: base world, entity and dragon server hooks (#1015)

Paper API 26.3 (paper-api `26.3.build.244-stable`), public Javadoc only; Horizon hooks are cited by function
name and scope is the base/dragon server hooks below. Rows without a key differ in moment, side, information or result.

| Horizon hook | Paper event / status | Evidence and porting boundary |
|---|---|---|
| `dragon_egg_form` | **Covered:** [`DragonEggFormEvent`][paper-DragonEggFormEvent] | The same before-placement moment: the dragon is defeated and its first egg is about to appear, and `isCancelled()` decides whether the block is placed, exactly like the hook's `cancel`. Server-side, and the `DragonBattle` handle identifies the fight whose egg is forming. Key added. |
| `enchant_effect` | **Gap:** none (nearest [`EntityPotionEffectEvent`][paper-EntityPotionEffectEvent]) | The hook reports a mod-declared `post_attack` enchantment entry on a hit mob (enchantment id, kind, effect JSON, level, victim), which no Paper event describes. The potion-effect events cover effects being applied or ticking, not enchantment entries. No key. |
| `enchant_item` | **Partial:** [`EnchantItemEvent`][paper-EnchantItemEvent] | Same enchant-table moment and the same facts: enchanter, enchant block, finished item and level cost are all exposed. But the event is cancellable and its setters change the outcome, while the hook only listens after the item was applied and has no `out` for that. No key. |
| `ender_pearl_teleport` | **Partial:** [`PlayerTeleportEvent`][paper-PlayerTeleportEvent] | The pearl teleport is a `PlayerTeleportEvent` (told apart by `getCause()`), before the teleport like the hook, and the player and from/to come through `PlayerMoveEvent`. The pearl entity and the fall-damage amount are missing, so `pearl` and `damage` can neither be read nor changed. No key. |
| `entities_load` | **Covered:** [`EntitiesLoadEvent`][paper-EntitiesLoadEvent] | Entities of a chunk have just been loaded, server-side; the chunk and its `getEntities()` give the dimension, chunk coordinates and the record count the hook passes. Non-cancellable, matching a listen-only hook. Key added. |
| `entities_unload` | **Covered:** [`EntitiesUnloadEvent`][paper-EntitiesUnloadEvent] | The same moment (entities of a chunk left the active world for saving); the chunk's entity list gives the dimension, coordinates and count, and the event is non-cancellable like the listen-only hook. Key added. |
| `entity_break_door` | **Partial:** [`EntityBreakDoorEvent`][paper-EntityBreakDoorEvent] | Same entity and door, but the event is cancellable (cancelling delays the break) and is raised as the door goes, while the hook only hears after the block was removed and cannot express that cancel. No key. |
| `entity_breed` | **Partial:** [`EntityBreedEvent`][paper-EntityBreedEvent] | Both parents (`getMother()`, `getFather()`) and the child (`getEntity()`) are exposed, matching the hook. The event is cancellable before the breed completes, though, and the hook only listens after `breed` returned, so the cancel has no `out`. No key. |
| `entity_change_block` | **Gap:** none | The dossier lists no Paper candidate and gives no Javadoc for a mob-driven block change, so an "after the new state was applied" moment with both state names cannot be confirmed. No key. |
| `entity_drop_item` | **Partial:** [`EntityDropItemEvent`][paper-EntityDropItemEvent] | Fires as an entity creates a drop, before the item entity exists, exposing the entity and the dropped `Item` (stack, amount, location), matching `item`, `count` and `pos`. It is cancellable, while the hook is listen-only and cannot stop the drop. No key. |
| `entity_enter_block` | **Partial:** [`EntityEnterBlockEvent`][paper-EntityEnterBlockEvent] | The same bee-into-hive moment with the entity and `getBlock()`. The event is cancellable before storage, while the hook only hears after `bee_enter_hive` stored and removed the mob, so its cancel has no `out`. No key. |
| `entity_enter_love_mode` | **Partial:** [`EntityEnterLoveModeEvent`][paper-EntityEnterLoveModeEvent] | The same interaction-driven love mode, with the `Animals` entity, the feeding `HumanEntity` and ticks in love. The event is cancellable and can shorten the love time (`setTicksInLove`), while the hook listens after love mode was set and cannot express either. No key. |
| `entity_interact` | **Covered:** [`PlayerInteractEntityEvent`][paper-PlayerInteractEntityEvent] | The same moment (right-click on an entity before its own interaction), once per hand and server-side; the player, `getRightClicked()` entity and `getHand()` carry the hook's facts, with the stack read from that hand. Cancelling is the hook's `claimed`. Key added. |
| `entity_join_level` | **Gap:** none (nearest [`PlayerJoinEvent`][paper-PlayerJoinEvent]) | The only candidate covers players only, fires non-cancellably once they are placed, and knows nothing about fresh versus disk-loaded entities. No Paper event in the dossier matches an arbitrary entity joining a level with a cancel. No key. |
| `entity_leave_level` | **Gap:** none (nearest [`PlayerBedLeaveEvent`][paper-PlayerBedLeaveEvent]) | The candidates are a player leaving a bed and decaying leaves; neither is an entity being removed from the level, so no event covers this moment. No key. |
| `entity_mount` | **Covered:** [`EntityMountEvent`][paper-EntityMountEvent] | The cancellable attempt to ride: rider is `getEntity()`, vehicle is `getMount()`, and cancelling is the hook's `canceled`; the hook's `mounting` direction and dimension follow from the rider's current vehicle and world. Key added. |
| `entity_portal` | **Partial:** [`EntityPortalEvent`][paper-EntityPortalEvent] | The same pre-teleport moment for a non-player entity, with the entity, from/to and the portal rules. It is cancellable (and lets plugins change the destination), while the hook only listens once the portal wait finished, so no cancel is expressible. No key. |
| `entity_pose_change` | **Partial:** [`EntityPoseChangeEvent`][paper-EntityPoseChangeEvent] | Called when the pose changes and non-cancellable, matching side and result. `getPose()` returns only the new pose, so the hook's `from` is not exposed; the hook also covers guest players only. No key. |
| `entity_shoot_bow` | **Partial:** [`EntityShootBowEvent`][paper-EntityShootBowEvent] | Same shooter and shot, exposing the shooter (`LivingEntity`), bow, `getProjectile()` and `getForce()`, matching the hook's facts. It fires before the arrow is launched and is cancellable, while the hook listens after the projectile entered the level and cannot express that cancel. No key. |
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
