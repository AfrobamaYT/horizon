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

## Event counterparts: base campfire hooks (#1015)

Paper API 26.3 (paper-api `26.3.build.244-stable`), public Javadoc; Horizon cited by function name; scope is the 20 base-campfire hooks below.
A key is added only where moment, side, information and result all agree; otherwise the row records the mismatch.

| Horizon hook | Paper event / status | Evidence and porting boundary |
|---|---|---|
| `campfire_start` | **Covered:** [`CampfireStartEvent`][paper-CampfireStartEvent] | Both fire server-side when an item placed on a campfire begins cooking; the block event gives the position and the recipe identifies the item, and the event is non-cancellable like the listen-only hook. Key added. |
| `can_eat` | **Gap:** none (nearest [`BlockCanBuildEvent`][paper-BlockCanBuildEvent]) | The supplied candidate decides block placement, not whether a player can eat a food stack; no Paper event exposes the vanilla `canEat` answer for the hook's server half. No key. |
| `can_survive` | **Partial:** [`BlockCanBuildEvent`][paper-BlockCanBuildEvent] | The placement check and buildable setter match the hook's placement case, but Paper does not fire this for the shape-update path and the event is not the general `canSurvive` question. No key. |
| `cauldron_level_changed` | **Covered:** [`CauldronLevelChangeEvent`][paper-CauldronLevelChangeEvent] | Both fire server-side immediately before a cauldron level changes, exposing the old/new levels, reason, block position and entity; Paper is cancellable, matching the hook's cancel out. Key added. |
| `chunk_load` | **Covered:** [`ChunkLoadEvent`][paper-ChunkLoadEvent] | Both fire server-side when a chunk finishes loading; `isNewChunk()` matches the hook's `new` flag and the chunk/world identify dimension and x/z. Non-cancellable, matching the listen-only hook. Key added. |
| `chunk_unload` | **Covered:** [`ChunkUnloadEvent`][paper-ChunkUnloadEvent] | Both fire server-side when a chunk leaves the world, with the chunk identifying dimension and x/z, and the event is non-cancellable like the listen-only hook. Key added. |
| `component_types` | **Gap:** none | Registering namespaced NBT component types is loader/registration work, not a Paper event moment; no supplied Javadoc event covers it. No key. |
| `compost_item` | **Partial:** [`EntityCompostItemEvent`][paper-EntityCompostItemEvent] | The entity/player compost event exposes the item, but it fires before composting while the hook fires after `composter_add`; it is also cancellable, which the listen-only hook cannot express. No key. |
| `consume_effect` | **Gap:** none | No Paper event covers a mod-defined `ConsumeEffect.apply` carrying an arbitrary kind and JSON; the supplied potion events only cover potion effects. No key. |
| `container_opened` | **Gap:** none | The dossier supplies no verified Paper menu-open event; no Javadoc candidate covers the server-side `openMenu` moment with its cancel result. No key. |
| `crafter_craft` | **Partial:** [`CrafterCraftEvent`][paper-CrafterCraftEvent] | Same crafter/recipe subject on the server, but Paper fires before the craft and is cancellable/settable, whereas the hook fires after a completed craft and only listens. No key. |
| `creative_slot` | **Covered:** [`InventoryCreativeEvent`][paper-InventoryCreativeEvent] | Both cover the server-side creative inventory slot action; the event exposes the player, slot and cursor `ItemStack` (including count), and its cancellable result matches the hook's cancel out. Key added. |
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
