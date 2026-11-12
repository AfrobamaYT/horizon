# Published loops: `[[native.loop]]`

A published loop is a hot loop the game owns and has written with a **slot** in it: a place where it asks
the installed mods about each entry before and after handling it. A native mod fills the slot with a type of
its own through a `[[native.loop]]` row. The launcher compiles one copy of the loop with the installed mods'
slot types in it and installs that copy; the game is not rebuilt. With no mod, the loop is the game's own and
costs nothing extra.

Like every `[[native.*]]` table this is the [unstable route](native-route.md): the slot gets the game's own
types, not `mod_api` values, and is pinned to the exact game build. Prefer a hook or a Part method when one
covers the behaviour.

The game publishes one loop so far: **`block_entities.tick`**.

## A first slot

This slot makes hoppers that are cooling down lose one tick of their cooldown and skip their tick. It is the
`throttle` row of the check mod `fnr_loop`, which CI builds against the prebuilt game.

`horizon-mod.toml` (excerpt; the full header is as on [Mixins](mixins.md#a-first-mixin)):

```toml
# Asked second: hoppers cooling down lose one tick of their real cooldown and skip their tick.
[[native.loop]]
id = "throttle"
loop = "block_entities.tick"
helper = "src/loops/throttle.rs"
with = "Throttle"
priority = 20
```

`src/loops/throttle.rs`:

```rust,ignore
/// While a hopper cools down, takes one tick off its real cooldown and skips its tick.
#[derive(Default)]
pub struct Throttle {
    skipped: u64,
}

impl BlockEntityTickSlot for Throttle {
    fn before(&mut self, tick: &mut BlockEntityTick<'_, '_>) -> LoopFlow {
        if tick.kind != TickedKind::Hopper {
            return LoopFlow::Continue;
        }
        match tick.game.hoppers.get_mut(&tick.pos) {
            Some(state) if state.cooldown > 1 => {
                let cooldown = state.cooldown;
                state.cooldown -= 1;
                self.skipped += 1;
                if self.skipped % 1000 == 1 {
                    fnr_loop::note(&format!("throttle skips #{}: cooldown {cooldown}", self.skipped));
                }
                LoopFlow::Skip
            }
            _ => LoopFlow::Continue,
        }
    }
}
```

The helper file is compiled beside the loop in the game's module, like a [mixin helper](mixins.md#a-helper):
`BlockEntityTickSlot`, `BlockEntityTick`, `LoopFlow`, `TickedKind` and the game's types are in scope without
imports, and it may call the mod's own crate (`fnr_loop::note`). Unlike a mixin helper it holds any Rust items
you like; one of them is the slot type `with` names.

On success the build log prints `Published loops: 1 filled by 1 mod slots` (with the number of loops and
slots), beside the `Function replacement:` line.

## Fields

| Field | Required | Meaning |
| --- | --- | --- |
| `id` | yes | The row's name, unique in the mod across every `[[native.*]]` and `[[inject]]` row. |
| `loop` | yes | A loop the pinned game publishes: `block_entities.tick`. |
| `helper` | yes | A `.rs` file under `src/` with the slot's code. |
| `with` | yes | The slot type the helper defines. It implements the loop's slot trait and `Default`, and is `'static`. |
| `priority` | no | Default 1000. Lower is asked first; ties go by mod id. |

## `block_entities.tick`

The server's block entity tick (vanilla's `Level.tickBlockEntities`) for hoppers, crafters, jukeboxes,
beacons, creaking hearts, spawners, trial spawners, vaults, beehives, furnaces, brewing stands, daylight
detectors, end gateways, campfires, potent sulfur and mods' ticking block entities. It runs on the `Game` that
simulates a world: a dedicated server, the single-player server thread, or a LAN host. The window's own
client `Game` never runs it.

The slot trait, `BlockEntityTickSlot`, has two methods, both with a default that does nothing:

- `before(&mut self, tick: &mut BlockEntityTick) -> LoopFlow` runs for each block entity vanilla would tick,
  after vanilla's own filters (in a block-ticking chunk, inside the world border, live, not asleep) and
  before its tick.
- `after(&mut self, tick: &mut BlockEntityTick)` runs once it has ticked; not for one a slot skipped.

The cursor, `BlockEntityTick`, lends for one call:

| Member | What it is |
| --- | --- |
| `game` | `&mut Game`: the whole game, its level, its `hoppers`, everything the loop has. |
| `pos` | The block entity's position. |
| `kind` | What it is, a `TickedKind`: `Hopper`, `Crafter`, `Jukebox`, `Beacon`, `CreakingHeart`, `Spawner`, `TrialSpawner`, `Vault`, `Beehive`, `Furnace`, `BrewingStand`, `DaylightDetector`, `EndGateway`, `Campfire`, `PotentSulfur`, or `Mod` for any mod's block entity. |
| `block_entity()` | The visited block entity itself, `Option<&mut BlockEntity>`. |

| Answer from `before` | Effect |
| --- | --- |
| `LoopFlow::Continue` | The entity ticks (after the next mod's slot is asked). |
| `LoopFlow::Skip` | This entity does not tick this tick. |
| `LoopFlow::Stop` | No further entity ticks this tick. The work after the loop still runs: song volumes, creaking hearts, client effects, shulker box lids. |

### Your own mod's block entities

`TickedKind::Mod` covers every mod's block entities, not only yours. The game has no per-mod kind. To act
only on your own, look at the block entity: a mod's is `BlockEntity::Mod(entity)`, and `entity.id()` is the
type id it was registered with, such as `mymod:machine`. That costs a map lookup and a string comparison for
each mod block entity, each tick, so test `tick.kind == TickedKind::Mod` first.

### Changing block entities from a slot

The cursor's `game` is the whole game, so a slot can add, remove or replace block entities:

- After every slot answered `Continue`, the loop checks the visited entity again. If it was removed, or
  replaced by one of another `kind`, it does not tick and no `after` runs for it.
- The pass visits the list it took when it started. An entity a slot adds is not visited in this pass and
  ticks from the next, as one placed by a tick does.
- An entity a slot removes later in the list is passed over when the loop reaches it, as one removed by an
  earlier tick is.

## Several mods, state, panics

- Rows on one loop are sorted by priority, then mod id. In `before`, the first answer that is not `Continue`
  wins and later mods are not asked for that entity. `after` runs for every mod.
- The slot value is made with `Default` on the loop's first run, once per `Game`, and kept across ticks. Key
  per-dimension state on the game's dimension.
- A run started from inside a slot (the slot calls something that ticks block entities again) is the game's
  own loop, without slots, and a whole pass: every entity ticks again and the work after the loop runs again.
  The enclosing pass's working state is set aside and restored, so it carries on as before. The server logs
  once: `Published loop block_entities.tick: it was entered again while it ran; it runs without its mods' slots`.
- A panic in a slot's `before`, `after` or `Default` unwinds out of the tick like any game panic: the server
  writes its crash report and saves the world. The report names the mod and the loop:
  `in native mod <id>, hook loop block_entities.tick, part <id>::<Type>`. That `Game` then runs the loop
  without slots and logs once: `Published loop block_entities.tick: a mod's slot panicked; it runs without
  its mods' slots`. The slots already made are dropped, each attributed to its own mod.
- Each mod's slot is dropped on its own. If one mod's `Drop` panics, the others are still dropped and the
  process does not abort.

## What composes with a loop

The loop is the function `horizon_client::game::block_entity_ticks::Game<'_>::tick_block_entities` (the
**entry**), which calls the generic `…::tick_block_entities_with` (the **body**) with the slots.

- A `HEAD` or `RETURN` mixin on the entry runs around the loop.
- An `[[inject]]` listener on the entry runs around it too.
- A mixin on a function the loop calls applies as anywhere else.
- Call-site mixins inside the loop target the body, `…::tick_block_entities_with`. The body is generic, but
  it is copied for the installed slots, so these rows apply there. They do not apply to the run without slots.

The check mod `fnr_loop` has one of each:

```toml
# A HEAD mixin on the loop's entry composes around the loop's copy.
[[native.mixin]]
id = "entry"
target = "horizon_client::game::block_entity_ticks::Game<'_>::tick_block_entities"
kind = "HEAD"
helper = "src/mixins/entry.rs"
bindings = [{ name = "game", ty = "&mut Game<'_>", value = "&mut *self" }]

# A call-site row inside the loop body, run for each chunk the loop reaches: the loop's generic
# function is copied for the slots' type, and this row is applied to that copy.
[[native.mixin]]
id = "inside"
target = "horizon_client::game::block_entity_ticks::Game<'_>::tick_block_entities_with"
kind = "MODIFY_EXPRESSION_VALUE"
selector = "ticking(pos.0, pos.2)"
require = 1
ordinal = 0
helper = "src/mixins/inside.rs"
bindings = [{ name = "value", ty = "bool", value = "$value" }]
```

## Refusals

| Code | Text | Fix |
| --- | --- | --- |
| `E_LOOP_WASM` | `horizon-mod.toml: E_LOOP_WASM: <mod>:<row>: <loop>: [[native.loop]] compiles game code; only native mods can fill a loop` | Build the mod native. |
| `E_LOOP_INVALID` | `horizon-mod.toml: E_LOOP_INVALID: <mod>:<row>: invalid loop; name a published loop of the game, such as block_entities.tick` | Name the loop. |
| | `… invalid helper; name a Rust file under src/ in this package` | Put the helper under `src/`, ending in `.rs`. |
| | `… invalid with; name the slot type the helper defines` | Write the type's name, a plain identifier. |
| `E_LOOP_UNKNOWN` | ``E_LOOP_UNKNOWN: <mod>:<row>: the game publishes no loop `<loop>`; its loops: block_entities.tick`` | Use a listed loop. |
| `E_LOOP_HELPER` | `E_LOOP_HELPER: <entry>: <mod>:<row>: its helper: <parse error>` (or `with: <parse error>`) | The helper is not valid Rust items; fix the error shown. |
| `E_LOOP_ENTRY_REPLACED` | ``E_LOOP_ENTRY_REPLACED: <mod>:<row>: the <KIND> mixin on <target> would replace or bypass the published loop `block_entities.tick`, filled by <mods>; only HEAD and RETURN mixins compose with it`` | On the entry use `HEAD` or `RETURN`; never `OVERWRITE` the body. |

A helper that does not compile against the pinned game is reported with the compiler's error under the mod's
id.
