//! The `mob_spawn_checks` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// NeoForge's `MobSpawnEvent.SpawnPlacementCheck`: may the entity type `kind` spawn at `pos` for `reason`? It is asked after vanilla's own spawn predicate (`SpawnPlacements.checkSpawnRules`: light, the ground it needs, the biome, slime chunks, the peaceful rule) has run: `default_result` is what it answered, and `result` starts as `Default`, which keeps that answer. Set `Succeed` or `Fail` to force the check. It runs wherever the game calls `checkSpawnRules` with the game's level: natural spawning, a village siege's zombies, a patrol, a zombie's reinforcements, a spawner and a trial spawner. A spawner whose spawn data carries custom spawn rules never reaches it, as in NeoForge. The predicate always runs first, as `default_result` is its answer, so its random draws are vanilla's whatever a mod answers; a listener should be a pure function of its inputs, because the checks run again when a tick is replayed. The host is the read-only one: a listener reads the world and writes nothing. Not told of `/summon` or a spawn egg, which do not check placements. Server-side. Beyond NeoForge: one `NativeSpawnCheck` answers this event and `mob_spawn_position_check`, which NeoForge gives two nested enums with the same three constants.
    #[notes = r#"## Spawn checks

- **`mob_spawn_placement_check(dimension, kind, reason, pos, default_result, result)`** (NeoForge's `MobSpawnEvent.SpawnPlacementCheck`) and **`mob_spawn_position_check(dimension, kind, reason, pos, result)`** (`MobSpawnEvent.PositionCheck`): set `result` to `NativeSpawnCheck::Succeed` or `Fail` to force a spawn check; `Default` keeps vanilla's. The placement check runs after the type's spawn predicate (`default_result` is its answer), so its random draws are vanilla's whatever a mod answers. The position check stands in for `Mob.checkSpawnRules` and `checkSpawnObstruction`: with no listener or `Default` they run and draw as vanilla, while `Succeed` and `Fail` skip them and their draws. Fired wherever the game checks a spawn: natural spawning, chunk generation, spawners, trial spawners (obstruction only), patrols, sieges and reinforcements for the placement check. Both get the read-only host and run on replayed ticks, so a listener is a pure function of its inputs. Tick-time effect: one `listened` branch with no listeners, before any `NativeSpawnReason` is built. One `NativeSpawnCheck` serves both (NeoForge's two enums have the same constants)."#]
    #[hook(
        side = "server",
        contract = "listen",
        host = "read",
        neoforge = "MobSpawnEvent.SpawnPlacementCheck",
        cancellable = false
    )]
    fn mob_spawn_placement_check();
    /// NeoForge's `MobSpawnEvent.PositionCheck`: may the freshly created mob of type `kind` stand at `pos` (its position, a mob's centre) for `reason`? Vanilla's two checks are `Mob.checkSpawnRules` (the type's own rules) and `Mob.checkSpawnObstruction` (inside blocks or fluids, an ocelot's grass and sea level). `result` starts as `Default`, which runs them; `Succeed` or `Fail` answers instead and they are skipped. It runs on natural spawning, chunk generation's creature packs and spawner blocks (where a spawner with custom spawn rules skips `Mob.checkSpawnRules` on `Default`, as vanilla and NeoForge do).  A trial spawner runs only `checkSpawnObstruction`, so only that is skipped. Server-side. With no listener or `Default` the checks and their random draws are vanilla's; `Succeed` and `Fail` skip the checks and so their draws (NeoForge's contract), and the level random's order changes only then; a listener should be a pure function of its inputs, because ticks are replayed.
    #[hook(
        side = "server",
        contract = "listen",
        host = "read",
        neoforge = "MobSpawnEvent.PositionCheck",
        cancellable = false
    )]
    fn mob_spawn_position_check();
}
