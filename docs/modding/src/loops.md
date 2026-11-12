# Published loops: `[[native.loop]]`

A published loop is a hot loop of the game with a **slot**: before and after each entry, the loop asks your
mod's type what to do. You fill the slot with a `[[native.loop]]` row; the game publishes one loop so far,
`block_entities.tick`.

Use a slot to change how individual block entities tick, at tick speed. To run code once around the whole
block entity tick, an [`[[inject]]` listener](injections.md) or a `HEAD` mixin on the loop is enough.

## Example

This slot makes hoppers that are cooling down lose one tick of their cooldown and skip their tick. It is the
`throttle` row of the check mod `fnr_loop`, which CI builds. The rest of the manifest is as on
[Mixins](mixins.md#example).

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

The helper file may hold any Rust items, with no `use` lines. It sees what the module
`horizon_client::game::block_entity_ticks` sees: `BlockEntityTickSlot`, `BlockEntityTick`, `LoopFlow`,
`TickedKind`, `Game`, `BlockEntity` and the rest. Like a [mixin helper](mixins.md#writing-a-helper), it may
call your mod's crate (`fnr_loop::note`).

## Fields

| Field | Required | Meaning |
| --- | --- | --- |
| `id` | yes | The row's name, unique in the mod. |
| `loop` | yes | A loop the game publishes: `block_entities.tick`. |
| `helper` | yes | A `.rs` file under `src/` with the slot's code. |
| `with` | yes | The slot type the helper defines. It implements the loop's slot trait and `Default`, and is `'static`. |
| `priority` | no | Lower is asked first; ties go by mod id. The default is 1000. |

## What you will see

The build log shows `Published loops: 1 filled by 1 mod slots`, with your numbers, beside the
`Function replacement:` line.

## The `block_entities.tick` slot

This is the server's block entity tick: hoppers, crafters, jukeboxes, beacons, creaking hearts, spawners, trial
spawners, vaults, beehives, furnaces, brewing stands, daylight detectors, end gateways, campfires, potent
sulfur and mods' ticking block entities. It runs on a dedicated server, the single-player server thread and a
LAN host; never on the window's client side.

The slot trait `BlockEntityTickSlot` has two methods, and both do nothing by default:

- `fn before(&mut self, tick: &mut BlockEntityTick<'_, '_>) -> LoopFlow` runs just before each block entity
  ticks;
- `fn after(&mut self, tick: &mut BlockEntityTick<'_, '_>)` runs just after it ticked.

`before` sees only the block entities vanilla would tick: in a block-ticking chunk, inside the world border,
live and not asleep. The cursor `tick` has:

| Member | What it is |
| --- | --- |
| `game` | `&mut Game`: the whole game, including its level and its `hoppers`. |
| `pos` | The block entity's position. |
| `kind` | A `TickedKind`: `Hopper`, `Crafter`, `Jukebox`, `Beacon`, `CreakingHeart`, `Spawner`, `TrialSpawner`, `Vault`, `Beehive`, `Furnace`, `BrewingStand`, `DaylightDetector`, `EndGateway`, `Campfire`, `PotentSulfur`, or `Mod` for any mod's block entity. |
| `block_entity()` | The block entity itself, `Option<&mut BlockEntity>`. |

| `before` returns | Effect |
| --- | --- |
| `LoopFlow::Continue` | The entity ticks, once every other mod's slot also continues. |
| `LoopFlow::Skip` | The entity does not tick this tick, and no `after` runs for it. |
| `LoopFlow::Stop` | No further entity ticks this tick. The work after the loop still runs, such as shulker box lids. |

**Your own mod's block entities.** `TickedKind::Mod` covers every mod's, and today there is no cheap way to
pick out yours ([#3492](https://github.com/arismoko/horizon/issues/3492)). The workaround: when `tick.kind`
is `TickedKind::Mod`, match `tick.block_entity()` against `Some(BlockEntity::Mod(entity))`, then compare
`entity.id()` with your type id, such as `"mymod:machine"`. Its data is `entity.nbt`, `entity.items` and `entity.data`.

## What you can rely on

- Slots are asked in priority order. The first answer that is not `Continue` wins, and later slots are not
  asked about that entity. `after` runs for every slot.
- Your slot value is made with `Default` on the loop's first run, once per `Game`, and kept. It sees every
  dimension in turn: read `tick.game.dim` to keep per-dimension state.
- A slot may add, remove or replace block entities. One it removes or replaces does not tick in this pass.
  One it adds ticks from the next pass.
- Running the loop again from inside a slot runs the game's own loop, without any slots, as a whole pass.
- A panic in a slot is reported naming your mod and the loop: `in native mod <id>, hook loop
  block_entities.tick, part <id>::<Type>`. A dedicated server writes it in a crash report. Single player
  only logs it today ([#3494](https://github.com/arismoko/horizon/issues/3494)).

## Mixins on the loop

The loop's entry is `horizon_client::game::block_entity_ticks::Game<'_>::tick_block_entities`. A `HEAD` or
`RETURN` mixin and an `[[inject]]` listener on it run around the loop. A mixin at a place inside the loop
targets the loop's body, `…::tick_block_entities_with`; it applies only while some mod fills a slot.

## Errors

| Code | Message | Fix |
| --- | --- | --- |
| `E_LOOP_WASM` | `<mod>:<row>: <loop>: [[native.loop]] compiles game code; only native mods can fill a loop` | Build the mod native. |
| `E_LOOP_INVALID` | `<mod>:<row>: invalid loop; name a published loop of the game, such as block_entities.tick` | Name the loop. |
| | `… invalid helper; name a Rust file under src/ in this package` | Put the helper under `src/`, ending in `.rs`. |
| | `… invalid with; name the slot type the helper defines` | Write the type's name, a plain identifier. |
| `E_LOOP_UNKNOWN` | ``<mod>:<row>: the game publishes no loop `<loop>`; its loops: block_entities.tick`` | Use a listed loop. |
| `E_LOOP_HELPER` | `<entry>: <mod>:<row>: its helper: <parse error>`, or `with: <parse error>` | Fix the Rust error shown. |
| `E_LOOP_ENTRY_REPLACED` | `<mod>:<row>: the <KIND> mixin on <target> would replace or bypass the published loop …; only HEAD and RETURN mixins compose with it` | On the entry, use `HEAD` or `RETURN`. Never `OVERWRITE` the body. |

A helper that does not compile is reported with the compiler's error under your mod's id.
