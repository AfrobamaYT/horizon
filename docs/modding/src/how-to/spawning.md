# Controlling spawns

Three hooks stand in for the checks and setup around a new mob. All run on the server, get the read-only host, and fire on replayed ticks, so a listener must be a pure function of its inputs.

| Moment | Hook | Answer |
|---|---|---|
| The spawn predicate for a type and position | `mob_spawn_placement_check` | `NativeSpawnCheck::{Default, Succeed, Fail}` |
| The mob's own position and obstruction checks | `mob_spawn_position_check` | `NativeSpawnCheck::{Default, Succeed, Fail}` |
| Just before `finalizeSpawn` | `finalize_spawn` | `Flow::Cancel`, `spawn_cancelled`, a new `difficulty` |

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

See [the hook notes](../reference/hook-notes.md#finalize-spawn) for what each spawn site covers and what is left out (group data, breeding, loads, worldgen structure mobs).
