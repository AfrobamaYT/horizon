pub trait Notes {
    #[notes = r#"## World and spawnable bounds

`LevelRef::in_world_bounds(host, pos)` and `in_spawnable_bounds(host, pos)` answer
`Level.isInWorldBounds` and `Level.isInSpawnableBounds` without loading a chunk. The first also
needs the position inside the level's build height; the second is the wider limit `/summon` and
`/tp` use (30,000,000 on x and z, 20,000,000 on y, whatever the level's height). A mod checks them
before reading or writing a far position, rather than learning it from a failed write."#]
    fn level_reads_note0();
}
