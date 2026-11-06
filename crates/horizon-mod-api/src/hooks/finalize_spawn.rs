//! The `finalize_spawn` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// NeoForge's `FinalizeSpawnEvent`: the new mob of type `kind` is at `pos` and `Mob.finalizeSpawn` (equipment, attributes, variants, the class's own setup) is about to run for `reason`. `spawner` is the spawner block for a spawner or trial spawner spawn, none otherwise (a minecart spawner too). `difficulty` is the local difficulty finalization reads: change it to change what the mob gets. There are two cancels, as in NeoForge. Return `Flow::Cancel` to skip finalization: no finalizeSpawn runs, and the mob still spawns uninitialised (no equipment, attributes or class setup). Set `spawn_cancelled` to stop the spawn: no mob is added and finalization does not run. It draws nothing itself; skipped finalization skips its draws (NeoForge's contract), and with no listener or an untouched event the draws are vanilla's. Fired only where vanilla 26.3 calls `finalizeSpawn` on a mob the game makes, with vanilla's `reason` at that call: natural spawning, chunk generation's animals, spawners, trial spawners, patrols, raids and sieges, zombie reinforcements, a cat spawner's cat, phantoms, jockeys, `/summon` and `/raid`, spawn eggs and dispensers, the nether portal's piglin (`Structure`), a skeleton trap's horse and riders, an evoker's vexes and wandering traders. A conversion fires with `Conversion`, and a cancel there leaves the converted mob as it is. Not fired where vanilla creates the mob without `finalizeSpawn`: breeding and a baby from a spawn egg, an infested block's or effect's silverfish, a thrown egg's chick, a thrown pearl's endermite, a hatching egg's turtle, sniffer or tadpole, a copper golem built from blocks, a slime from the oozing effect and a lightning trap's horse. Nor for entities loaded from disk or structure mobs placed by world generation. Group data (`SpawnGroupData`) is left out: the game's finalization has no group to give. The host is the read-only one; listeners run on replayed ticks, so they must be pure functions of their inputs. Server-side.
    #[notes = r#"## Finalize spawn

- **`finalize_spawn(dimension, kind, reason, pos, spawner, difficulty, spawn_cancelled, cancel)`** (NeoForge's `FinalizeSpawnEvent`): fires where the game calls `Mob.finalizeSpawn` for a mob it made, before it runs. There are two cancels, as in NeoForge: return `Flow::Cancel` to skip finalization (no equipment, attributes or class setup; the mob still spawns uninitialised, and finalization's random draws are skipped), or set `spawn_cancelled` so no mob is added. `difficulty` (`NativeDifficulty`) is settable and is what finalization reads. `spawner` is `NativeSpawner::Block` for a spawner or trial spawner, none otherwise. Fired only where vanilla 26.3 calls `finalizeSpawn`, with vanilla's reason at that call: natural spawning, chunk generation, spawners, trial spawners, patrols, raids and sieges, reinforcements, cat spawners, phantoms, jockeys, `/summon`, `/raid`, spawn eggs, dispensers, the nether portal's piglin (`Structure`), skeleton traps, evoker vexes, wandering traders and conversions. Not fired where vanilla creates the mob without it (breeding, an infested block's silverfish, a thrown egg's chick, a thrown pearl's endermite, hatching eggs, built golems, the oozing effect's slimes, a lightning trap's horse), for loads (`persist.rs` finalizes a placed entity only through its `horizon:finalize` marker, which is a load) or for structure mobs placed by world generation. Group data is left out: it lives only in the worldgen `finalize` path, which has no host. Both new types are plain data (`NativeDifficulty` and `NativeSpawner`, `#[non_exhaustive]`). Read-only host; replayed ticks, so listeners are pure functions of their inputs. Tick-time effect: one `listened` branch with no listeners. Beyond NeoForge: none.

`window_resize` notifies client mods after a window resize or applying a changed GUI scale, fullscreen, or Unicode font setting. Its `width` and `height` are physical pixels, and `gui_width` and `gui_height` are scaled GUI pixels. It fires only when physical or scaled dimensions change.

### Structure template snapshots

`ServerHost::structure_template(id, at, rotation, mirror)` reads a template without placing it.
It searches the world's saved templates before data packs and returns the rotated size and blocks
in vanilla placement order, with each transformed state and its saved block-entity data. It uses
the palette selected at `at`; processors and entities are not included. Invalid or missing names
return `None`. A builder can retain this snapshot and place its blocks over time."#]
    #[hook(
        side = "server",
        contract = "cancel",
        out = "cancel",
        host = "read",
        neoforge = "FinalizeSpawnEvent",
        cancellable = true
    )]
    fn finalize_spawn(
        dimension: &str,
        kind: &str,
        reason: NativeSpawnReason,
        pos: [f64; 3],
        spawner: Option<NativeSpawner>,
        difficulty: &mut NativeDifficulty,
        spawn_cancelled: &mut bool,
        cancel: &mut bool,
    );
}
