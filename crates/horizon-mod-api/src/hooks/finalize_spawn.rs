//! The `finalize_spawn` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// NeoForge's `FinalizeSpawnEvent`: the new mob of type `kind` is at `pos` and `Mob.finalizeSpawn` (equipment, attributes, variants, the class's own setup) is about to run for `reason`. `spawner` is the spawner block for a spawner or trial spawner spawn, none otherwise (a minecart spawner too). `difficulty` is the local difficulty finalization reads: change it to change what the mob gets. There are two cancels, as in NeoForge. Return `Flow::Cancel` to skip finalization: no finalizeSpawn runs, and the mob still spawns uninitialised (no equipment, attributes or class setup). Set `spawn_cancelled` to stop the spawn: no mob is added and finalization does not run. It draws nothing itself; skipped finalization skips its draws (NeoForge's contract), and with no listener or an untouched event the draws are vanilla's. Fired only where vanilla 26.3 calls `finalizeSpawn` on a mob the game makes, with vanilla's `reason` at that call: natural spawning, chunk generation's animals, spawners, trial spawners, patrols, raids and sieges, zombie reinforcements, a cat spawner's cat, phantoms, jockeys, `/summon` and `/raid`, spawn eggs and dispensers, the nether portal's piglin (`Structure`), a skeleton trap's horse and riders, an evoker's vexes and wandering traders. A conversion fires with `Conversion`, and a cancel there leaves the converted mob as it is. Not fired where vanilla creates the mob without `finalizeSpawn`: breeding and a baby from a spawn egg, an infested block's or effect's silverfish, a thrown egg's chick, a thrown pearl's endermite, a hatching egg's turtle, sniffer or tadpole, a copper golem built from blocks, a slime from the oozing effect and a lightning trap's horse. Nor for entities loaded from disk or structure mobs placed by world generation. Group data (`SpawnGroupData`) is left out: the game's finalization has no group to give. The host is the read-only one; listeners run on replayed ticks, so they must be pure functions of their inputs. Server-side.
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
