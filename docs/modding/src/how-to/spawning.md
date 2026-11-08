# Controlling spawns

Four hooks stand in for the checks and setup around a mob. All run on the server, get the read-only host, and fire on replayed ticks, so a listener must be a pure function of its inputs. `finalize_spawn` can also change the mob it is about to finalize, through the editor it is handed, and the pack's group data; it changes nothing else.

| Moment | Hook | Answer |
|---|---|---|
| The spawn predicate for a type and position | `mob_spawn_placement_check` | `NativeSpawnCheck::{Default, Succeed, Fail}` |
| The mob's own position and obstruction checks | `mob_spawn_position_check` | `NativeSpawnCheck::{Default, Succeed, Fail}` |
| Just before `finalizeSpawn` | `finalize_spawn` | `Flow::Cancel`, `spawn_cancelled`, a new `difficulty`, edits to `mob`, a new `group` |
| Each tick, before a mob's `checkDespawn` | `mob_despawn` | `NativeDespawn::{Default, Allow, Deny}` |

## Stopping a spawn or leaving a mob plain

`finalize_spawn` has two cancels, as in NeoForge. Returning `Flow::Cancel` skips finalization: the mob still spawns, but with no equipment, attributes or class setup. Setting `event.spawn_cancelled = true` adds no mob at all.

```rust,ignore
fn finalize_spawn(&self, _: &dyn mod_api::Host, event: &mut mod_api::FinalizeSpawnEvent<'_>) -> mod_api::Flow {
    if event.kind == "minecraft:zombie" && event.reason == mod_api::NativeSpawnReason::Natural {
        event.spawn_cancelled = true;
    }
    mod_api::Flow::Continue
}
```

`finalize_spawn` fires only where vanilla calls `finalizeSpawn`, so a breeding, an infested block's silverfish or a thrown egg's chick does not tell it.

For `/summon minecraft:ender_dragon` without NBT, command placement starts at zero
rotation and consumes no random yaw draw. Skipping finalization also skips its
follow-range and handedness draws. Spawn eggs retain their random placement yaw;
an explicit NBT compound follows the load route without finalization.

## Changing the mob before it is finalized

`event.mob` is the mob itself, an `UnspawnedMobRef`: it is in no level yet, so it is not an `EntityRef`, and only this
hook call can use it. Read it through the host (`kind`, `position`, `equipment`, `attribute_base`, `attribute_value`,
`tags`, `custom_name`) and change it through `event.edits`, which reaches only this mob: equipment (an API-owned `ItemStack`), an attribute's base or a permanent modifier, tags and the custom name. An edit applies at once, so the
reads and the next listener see it. Once the hook returns, the handle answers `UnspawnedMobError::Expired`, as does a
handle from another call passed to this call's editor.

```rust,ignore
fn finalize_spawn(&self, host: &dyn mod_api::Host, event: &mut mod_api::FinalizeSpawnEvent<'_>) -> mod_api::Flow {
    if event.mob.kind(host).is_ok_and(|kind| kind.path() == "zombie") {
        let health = mod_api::Identifier::parse("minecraft:max_health").unwrap();
        let _ = event.edits.set_attribute_base(event.mob, &health, 40.0);
        let _ = event.edits.add_tag(event.mob, "mymod:tough");
    }
    mod_api::Flow::Continue
}
```

Listeners run before finalization, as in NeoForge. Finalization's own rolls still follow them: equipment it rolls for a
slot replaces what a listener put there, and the attribute bonuses it adds apply on top of a listener's base. Return
`Flow::Cancel` to keep exactly what the listener left, since no finalization runs. Attribute edits reach the
attributes a mob keeps itself (max health, movement speed, follow range, attack damage, knockback resistance,
spawn reinforcements, jump strength, burning time, flying speed); the others answer `Unsupported` until the mob has
joined, when a `LivingRef` writes them. The ender dragon's handle answers its kind and position, and `Unsupported`
for the rest. A mob finalization itself creates (a chicken jockey's chicken, a zombie horse's rider) fires the hook
again with a handle of its own.

The hook still fires on replayed ticks with the read-only host, so the editor and `group` are the only things a
listener changes, and it must change them the same way every time: a pure function of the event.

`event.group` is the pack's `SpawnGroupData` as plain data (`NativeSpawnGroup`, variants named as vanilla saves them):
read it, change it, set it or clear it, and finalization reads what is left. Every site the hook fires at hands it
`None` today: the game does not yet thread group data from one member of a natural spawning pack to the next, and
chunk generation's animals share it only inside the generator, out of the hook's reach. Setting a group still reaches this
mob's finalization; carrying it on to the pack's next member comes with that threading.

See [the hook notes](../reference/hook-notes.md#finalize-spawn) for what each spawn site covers and what is left out (breeding, loads, worldgen structure mobs).

## Keeping or removing a mob

`mob_despawn` fires once per tick for every mob in a loaded chunk, just before vanilla's `Mob.checkDespawn`, persistent mobs and mobs in peaceful mode included. `Allow` discards the mob, `Deny` keeps it, `Default` leaves vanilla's rules to decide. An answer other than `Default` skips all of vanilla's checks, so a `Deny` keeps a hostile mob in peaceful mode and neither answer takes the idle-despawn `nextInt(800)` draw.

```rust,ignore
fn mob_despawn(&self, _: &dyn mod_api::Host, event: &mut mod_api::MobDespawnEvent<'_>) {
    if event.kind == "minecraft:zombie" {
        *event.result = mod_api::NativeDespawn::Deny;
    }
}
```

The wither and the ender dragon override `checkDespawn` in vanilla, so the event is not fired for them.

## Adding or removing a custom spawner

The level's custom spawners (phantoms, patrols, cats, village sieges, the wandering trader) are a list a mod edits once, in `modify_custom_spawners`. A mod's own spawner is a key plus `custom_spawner_tick`, which runs every tick while mob spawning is on:

```rust,ignore
fn modify_custom_spawners(&self, _: &dyn mod_api::Host, event: &mut mod_api::ModifyCustomSpawnersEvent<'_>) {
    // no phantoms in this world
    event.spawners.remove(&mod_api::SpawnerEntry::Builtin(mod_api::SpawnerKind::Phantom));
    event.spawners.add(mod_api::SpawnerKey::new("mymod:rat_swarm").unwrap());
}

fn custom_spawner_tick(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::CustomSpawnerTickEvent<'_>) {
    if event.key.as_str() == "mymod:rat_swarm" && event.spawn_enemies {
        // spawn through `host`
    }
}
```

Every mod is asked about every key, so a mod ignores keys it does not own. See [the hook notes](../reference/hook-notes.md#custom-spawners).

## Spawning from a level

`LevelRef::spawn_mob(host, kind, pos, reason)` is `EntityType.spawn`: the mob is made as any of its type is, Parts, attachments and `entity_constructing` included, then `finalize_spawn` and `finalizeSpawn` run with `reason`, and it joins the level (`entity_join_level`), not persistent. It returns the mob's handle, or `None` with nothing spawned: a kind that isn't a mob, a position that isn't finite, outside `Level.isInSpawnableBounds` or in an unloaded chunk, a level not in place, a client, or a mod that cancelled the spawn or kept the mob out. The level's spawn limits, flags and spawn rules don't apply, as they don't to `/summon`. Items, experience and block drops have their own calls (`spawn_item`, `award_experience`, `pop_resource`), and any other entity `summon`.

A level's natural spawning (`NaturalSpawner.spawnForChunk`) also has a policy a mod can change:

- `spawn_limits(host)` and `set_spawn_limits(host, limits)`: a `SpawnLimits` with each spawning category's `getMaxInstancesPerChunk`, `SpawnLimits::VANILLA` (70 monsters, 10 creatures, 15 ambient, 5 axolotls, underground water and water creatures each, 20 water ambient) until a mod sets others. A limit caps the category twice, as vanilla's does: over the level, under `limit * spawnable chunks / 289`, and around each player near the chunk, under `limit`. A limit of 0 stops the category.
- `spawn_flags(host)` and `set_spawn_flags(host, flags)`: a `SpawnFlags` with `monsters` and `friendlies`, both on until a mod turns one off. Off keeps that side out of the level's natural spawning; neither turns monsters on against the `spawn_monsters` rule or anything on with `spawn_mobs` off.
- `can_spawn_entities_in_chunk(host, [x, z])`: `ServerLevel.canSpawnEntitiesInChunk`, the question natural spawning asks of a spawn position outside the chunk it started in. Horizon's spawner answers it as "the chunk is loaded", and so does this call.

```rust,ignore
fn level_load(&self, host: &mut dyn mod_api::ServerHost, event: &mut mod_api::LevelLoadEvent<'_>) {
    let Some(level) = host.level(event.dimension) else { return };
    // twice the monsters, no bats
    level.set_spawn_limits(host, mod_api::SpawnLimits { monster: 140, ambient: 0, ..mod_api::SpawnLimits::VANILLA });
}
```

Only natural spawning reads the policy: spawners, trial spawners, patrols, sieges, phantoms, a new chunk's creatures and custom spawners keep their own rules. A change counts from the level's next spawn pass, and mobs already there stay. The policy is not saved: it holds until the world closes, while the level is parked and across its closing and opening again, and the next open starts from vanilla's, so set it in `level_load`. Reads work on any host and answer `None` on a client or for a stale handle; the writes need a `ServerHost` and return `false` there.
