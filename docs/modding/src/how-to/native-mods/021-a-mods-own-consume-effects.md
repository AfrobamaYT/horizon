<!-- chapter: A mod's own consume effects -->
<!-- separator: 1 -->

### A mod's own consume effects

A consumable's `on_consume_effects` entry of a type in a namespace of its own reaches `native_consume_effect(kind, data)` when the item is consumed, on the server
thread, for the acting player. The kind and the JSON the file wrote are there; the game's own effects,
sounds, teleports and drops are open to the hook. An unclaimed kind does nothing.
