# Native state: `[[native.state]]` is refused

horizon-mixins can add a field to a game type with a `[[state]]` table. Horizon refuses it: a mod that
declares `[[native.state]]` does not build.

```text
E_MIXIN_STATE_LAYOUT: <mod>:<row>: native state changes the prebuilt game's layout; use an attachment instead
```

## Why

The [native route](native-route.md) never rebuilds the game. Your mod's code is compiled against the prebuilt
game, and only the functions you target are copied and swapped in. A new field changes the type's size and
the place of its other fields, but every function of the prebuilt game that you did not copy was compiled
with the old layout and would keep reading the old places. The only way to add a field safely would be to
rebuild the whole game for each set of mods, which is what this route exists to avoid.

## What to use instead

Per-object data is an **attachment**: a typed value the game keeps for you on an entity, block entity, chunk,
level or player, saved with it when you give it a codec. Attachments work the same on native and wasm mods,
and through the stable API, so they also survive game updates. See
[server attachments](concepts/trait-mods.md#server-attachments) and
[querying attachment holders](how-to/attachments.md).

Data for the whole game rather than one object belongs in your mod's own crate (a `static`, which Rust requires to be `Sync`: single player runs a client and a server thread), or in a [published loop](loops.md)'s slot value, which the game keeps once per
`Game`.
