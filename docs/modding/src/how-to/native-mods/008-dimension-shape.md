<!-- chapter: Dimension shape -->
<!-- separator: 1 -->

### Dimension keys

`LevelRef::dimension` returns a `DimensionKey`, a small `Copy` value that names the dimension
for as long as the game runs, custom dimensions included. Compare it with another key or with
`DimensionKey::OVERWORLD`, `NETHER` and `END`; `level.dimension().id(host)` gives the name
(`minecraft:the_nether`). On a client, `host.level(name)` gives the level it shows under the
name the server calls it, whether or not that is a vanilla dimension.

### Dimension shape

`LevelRef::dimension_shape` reads the loaded server level's build range, logical height,
skylight and ceiling from its dimension shape. It accepts `&dyn Host`, including when the
level is parked, and returns `None` for a stale or unavailable level or on a client. It does
not load a level or allocate.
