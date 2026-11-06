<!-- chapter: Conditional data -->
<!-- separator: 1 -->

### Conditional data

Recipe files and tag layers handled by content-root composition and world-table
loading can carry a `neoforge:conditions` array. Every condition
must hold; a false or unknown condition omits the recipe or tag layer. Tags are
checked before merging or applying `replace`, so an omitted layer leaves lower
entries intact.

`neoforge:mod_loaded` asks the owning resource generation's enabled server-mod IDs,
including active code-only native and WebAssembly mods, plus `minecraft` always.
Client-only, disabled and refused packages are absent; resource namespaces are not
mod IDs. `neoforge:true`, `neoforge:not`, `neoforge:and` and `neoforge:or` combine
conditions. Successful reloads publish changed enabled IDs together with the data
generation. Other non-recipe readers still need their own condition integration, including
the separate tag readers used by loot contexts and damage-resource loading
([#814](https://github.com/arismoko/horizon/issues/814)).
