//! The custom spawner hooks: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// NeoForge's `ModifyCustomSpawnersEvent`: the level `dimension` is about to tick its custom
    /// spawners for the first time. `spawners` is the list `ServerLevel` ticks, in order, and it
    /// starts with vanilla's (`WanderingTrader`, `Patrol`, `Cat`, `VillageSiege` and `Phantom`,
    /// in the order Horizon ticks them). Take a vanilla spawner out with `remove` and it never
    /// ticks, so its random draws are gone with it; add a spawner of the mod's own with `add` and
    /// a `SpawnerKey` the mod owns, and `custom_spawner_tick` runs for it every tick, after the
    /// ones before it in the list. NeoForge adds a `CustomSpawner` object; a key stands for it
    /// because an object a guest implements cannot cross to a wasm mod (Beyond NeoForge). With
    /// no listener, or one that leaves the list alone, the level ticks exactly vanilla's
    /// spawners. The phantom spawner runs after the level's mobs have ticked, as the game runs
    /// it today, wherever it is in the list; moving the others to vanilla's order is a separate
    /// parity change. It fires once per level, on the server, and listeners run on replayed ticks,
    /// so they must be pure functions of their inputs. Not cancellable.
    #[hook(
        side = "server",
        contract = "listen",
        host = "read",
        neoforge = "ModifyCustomSpawnersEvent",
        cancellable = false
    )]
    fn modify_custom_spawners();

    /// `CustomSpawner.tick` for a spawner a mod added with `modify_custom_spawners`: the level
    /// `dimension` ticks the spawner `key`, once per tick in the list's order, with
    /// `spawn_enemies` as `ServerLevel.isSpawningMonsters` (the `spawn_monsters` rule, and not
    /// peaceful). Every mod gets the call, and the mod that owns `key` spawns through `host`
    /// (its spawns go through `finalize_spawn` and the spawn checks like any other); the rest
    /// ignore it. It is called only while the `spawn_mobs` rule is on, as vanilla's custom
    /// spawners are, and only for a key in the list, so a level with no mod spawner pays one
    /// check per tick. Beyond NeoForge: a key and this hook stand in for the `CustomSpawner`
    /// object. Server-side.
    #[hook(side = "server", contract = "listen")]
    fn custom_spawner_tick();
}
