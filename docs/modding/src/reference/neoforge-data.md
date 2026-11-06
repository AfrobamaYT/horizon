# NeoForge data files

This is a scoped audit of server data, not a claim that carrying a file into a
package executes it. The paths and examples below identify the readers and
fixtures checked; the modifier gaps are tracked separately.

| Data | Result and reader | Example or evidence |
| --- | --- | --- |
| Convention item tags | Loaded for content roots: `mod_content::build` in `crates/horizon-client/src/mod_content.rs` merges tag files; `content_namespaces::tag_namespaces` in `crates/horizon-items/src/content_namespaces.rs` includes `c` for content roots, and `items::load_tag_lists` in `crates/horizon-items/src/items.rs` reads and resolves them. Unknown item entries are filtered by `Known::keeps` in `mod_content.rs`. | `data/c/tags/item/gems/glint.json`: `{"values":["minecraft:diamond"]}`. The `glint_mod` fixture and `a_data_mod_loads_unloads_and_loads_again` test in `crates/horizon-client/src/mod_content_tests.rs` check the resulting `c:gems/glint` membership. |
| Structure modifiers | Not loaded by any reader found today. `mod_content::build` copies ordinary files to its overlay; `worldgen_entry` in that file recognizes worldgen bundles, not structure modifiers. A workspace search for `structure_modifier` found no reader. Copying is not applying. | No verified in-tree example. [#1269](https://github.com/arismoko/horizon/issues/1269) tracks loading and applying them. |
| Global loot modifiers | Not loaded by any reader found today. `mod_content::build` copies ordinary files to its overlay, but a workspace search for `loot_modifiers` found no loader. The `native_loot_drops` code hook in `crates/horizon-mod-api/src/hooks/base.rs` is not a data-file reader. | No verified current-format in-tree example. [#1270](https://github.com/arismoko/horizon/issues/1270) tracks loading and applying them. The old `global.json` in `carry::tests::claims_the_namespaces_its_files_name` is a carry classification fixture, not proof of current-format loading. |

Recipes and tag layers use `conditions::conditions_hold` from
`crates/horizon-world/src/conditions.rs`. Tag files are checked individually before
merge or `replace`, both while generating content roots and while loading world
pack stacks. A false or unknown condition omits that layer; it cannot clear lower
entries. Supported forms are `neoforge:mod_loaded`, `neoforge:true`, `neoforge:not`,
`neoforge:and` and `neoforge:or`, inside `neoforge:conditions` arrays.

`mod_loaded` reads the owning generation's server-enabled mod IDs, including
code-only native and WebAssembly mods; client-only, disabled and refused packages
are absent. Namespaces do not imply a loaded mod. Changed enabled IDs select a new
content-cache root and are published with successful reloads. The conditional-tag
cache and reload fixtures cover this contract. Conditions on other non-recipe data
readers remain in [#814](https://github.com/arismoko/horizon/issues/814); this does
not add readers for the two modifier kinds above.
