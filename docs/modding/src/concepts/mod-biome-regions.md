# Design: a mod's biomes inside the Overworld and Nether

Scope 2 of [#2554](https://github.com/arismoko/horizon/issues/2554): how a mod's biomes generate among vanilla's in the Overworld and the Nether. The design below was ruled on in [#2554](https://github.com/arismoko/horizon/issues/2554#issuecomment-6803604498) and built in the stages listed at the end. One part is not built yet and is marked so. Scope 1, a mod's own dimension with its own climate points (`BiomeSourceKind::Points`), is separate and does not need this note.

## Starting points

Ari's answer on #2554 set two starting points:

- **Regions with weights.** Each mod lays out its biomes in its own region. A hidden extra parameter decides which region owns an area, and vanilla's layout is one region among them. Ordering between mods is settled by the regions, not by competing for the same climate points.
- **The mix is fixed when the world is created and saved with the world.** Terrain in an existing world changes only by a recorded, per-world choice.

The same answer says NeoForge has no API for this: its biome modifiers change biomes that already generate and cannot add a biome to a dimension's layout, and the region mechanism is a third-party library. So there is no NeoForge contract to mirror. This note did not read that library's source and claims nothing about its behaviour.

## Horizon today

| Fact | Where |
| --- | --- |
| A level's biome source is one of fixed, checkerboard, multi-noise by preset, the End, or (scope 1) a mod's own points. | `BiomeSourceKind` in `crates/horizon-mod-vocab/src/world/dimension.rs` |
| The game writes the level stem's `biome_source` JSON; the generator decodes and resolves it once per dimension. | `biome_source` in `game/modding/native_dimensions.rs`; `worldgen/biome_source/codec.rs`, `runtime.rs` |
| A multi-noise source is a list of `(ParameterPoint, biome)` searched for the nearest point to the six climate values and a zero offset. | `worldgen/climate/index.rs` (`ClimateIndex`, `ClimateSearch`) |
| The Overworld's and the Nether's lists are named presets read from the prepared data, not data a mod can extend. | `GeneratorData::load`, `MultiNoiseBiomeSource::from_quantized_json` |
| A chunk stores its biomes per section, so a chunk that already exists keeps them whatever the source says later. A saved biome the game lacks is kept by name once #3132 lands. | `decode_column` / `encode_with` in `crates/horizon-client/src/save.rs` |
| Terrain shape never reads the biome: density and the router read only climate. Surface rules and `carver_biome` do read it. | `worldgen/density.rs`, `router.rs`, `generator.rs` |
| Seeded Overworld and Nether preset output is already compared with the Java oracle. | `tools/test_biome_generation_parity.py` |

## Regions

**Region.** A region is `(owner mod, region id, weight, points)`, where `points` are the `BiomePoint`s of scope 1. Vanilla's own list is the region `minecraft:vanilla`. A mod declares one at load with `Registrar::declare_biome_region(path, region)`, where the `BiomeRegion` holds the dimension (the Overworld or the Nether), the weight and the points. The id is namespaced to the declaring mod. A malformed or duplicate id, another dimension, a weight outside 1 to 1000, an empty point list or an invalid point refuses, naming the mod. In the mod API the record is `BiomeRegion`. The engine's own `worldgen::biome_chunk::BiomeRegion` is a different type that mods never see; where both meet in one engine file, alias one at the import.

**Weights.** Integers. A mod region's weight is 1 to 1000. Vanilla's region is 10 by default, so a weight of 10 means "as common as vanilla's whole layout". A zero weight and an overflowing total are refused. A mod's declared weight is only the default written into a world's record when the world is created. After that, the record's weights are the ones generation uses, and nothing reads a weight from the mod. Players and server owners can then change them without a format change. A setting that edits them at creation is a separate follow-up.

**Order.** Regions are sorted by owner mod id, then region id, the same rule the attribute catalog uses. The same mods give the same order on every side, so the list can be saved and compared.

## Choosing a region

- **Seed stream.** `root.from_hash_of("horizon:biome_region")`, where `root` is the generator's existing `random.fork_positional()`. `from_hash_of` is a pure function of root and name and advances nothing, so no vanilla stream moves.
- **Jittered cells.** Horizontal space is split into square cells of 128 quarts (512 blocks). Each cell gets one uniform draw from the stream at that cell, which gives a jitter point and a region chosen by cumulative weight. A quart belongs to the nearest jitter point among the 3×3 cells around it. Shares then follow the weights in expectation, and borders are irregular. A thresholded `NormalNoise` would not give the declared shares, because its values are not uniformly distributed.
- **x and z only.** The owner depends on x and z only, so a column never mixes regions vertically, and `carver_biome` (sampled at y = 0) agrees with the chunk fill.
- **Search.** The nearest-point search then runs in the chosen region's own `ClimateIndex`. Vanilla's region uses the preset index exactly as today.
- **Pinned by the record.** `selector_version`, the cell size, the salt and the 3×3 rule are read from the world's saved record, never from constants at generation time. A version 2 can then exist beside version 1, and old worlds keep version 1.
- **No regions, no change.** With no record, or a record that holds only vanilla, generation takes today's path unchanged. The existing seeded preset oracle proves this for the Overworld and the Nether.

**What changes at a border.** Surface material and carvers change at a region border; terrain shape does not, since density reads only climate. A structure whose biome check straddles a border follows vanilla's own check at its sample position.

## The saved record

One durable world-level file, `data/horizon/biome_regions.dat`, separate from `data/minecraft/world_gen_settings.dat`:

```
{ version: 1,
  dimensions: { "minecraft:overworld": {
      selector: { version: 1, cell: 128, salt: "horizon:biome_region", vanilla_weight: 10 },
      regions: [ { owner: "my_mod", id: "my_mod:highlands", weight: 5,
                   points: [ { biome: "my_mod:highland", temperature: [min, max], …, offset } ],
                   substituted: { with: "minecraft:vanilla", missing_mod: "my_mod" } /* only after a choice */ } ] } } }
```

- **Contents.** Regions are stored in sorted order with their full points, not hashes. A new world writes the record before its first chunk is generated.
- **Absent record.** Region selection is off and every dimension keeps its existing source, custom dimensions included.
- **Malformed record.** The open is refused with the reason, and the file is left untouched on disk: no rewrite, no rename, no default record in its place.
- **Changed mods.** An installed mod whose declared regions differ from the record (points, biomes, or a region added or removed) never changes an existing world silently. The record wins, the difference is reported once by name, and the new layout is taken only through the opt-in below.

## Missing mods and opting in

- **A saved region's mod is absent.** The world names each missing mod and region and requires a choice: do not open, or open and resolve that region through vanilla's index from then on. The region keeps its slot and weight, so the other regions' borders do not move. The choice is recorded on the region with the missing mod's id.
- **Client.** The question is asked in the existing pre-load prompt, where the backup questions are.
- **Dedicated server.** `server.toml` gets `[biome_regions] missing = "refuse" | "vanilla"`, default `"refuse"`. Without the setting the server refuses to start, naming the mod and region.
- **The mod returns.** The substitution is reversible. When the mod returns, the world says so once and keeps the substitution until the owner opts back in, since the chunks generated in between are vanilla there.
- **Opt-in.** "Use the installed regions for new chunks" is a recorded per-world choice, off by default: on a server `adopt_installed = false` under `[biome_regions]`. Adopting keeps the saved weight of every region that is still declared, takes the declared weight for a new one, and drops the regions of mods that are not installed. *Not built:* an opt-in in the client; today only a dedicated server can adopt, with that setting. The new layout is written durably before any chunk is generated with it. Chunks that already exist keep their biomes and blocks.

## Acceptance matrix

Each row names the test that covers it. "Not covered" means no test checks it yet.

| Fixture | Expected | Covered by |
| --- | --- | --- |
| No region declared | Overworld and Nether biome output identical to today for fixed seeds (seeded preset oracle), negative coordinates and chunk edges included. | `tools/test_biome_generation_parity.py` (the seeded preset oracle, unchanged path); `horizon-world` `worldgen::biome_source::region_selector_tests::without_regions_the_source_is_vanilla_unchanged` (no record, an empty record and a never-chosen region all give vanilla's biomes over negative and positive quarts, scalar and prepared) |
| Two mods, weights 1 and 3, vanilla 10 | Over a large sample, region ownership follows the weights within a stated tolerance; the same seed gives the same map twice. | `horizon-world` `worldgen::biome_source::region_selector_tests::cells_split_by_weight` (20 000 cells, each share within 0.02 of its weight / 14); `horizon-world` `worldgen::biome_source::region_selector_tests::same_seed_and_layout_give_the_same_owners` |
| Same mods installed in a different order | Same sorted list, same map. | `horizon-mod-vocab` `parts::biome_regions::tests::declared_regions_install_in_mod_then_id_order` (the list); `horizon-world` `worldgen::biome_source::region_selector_tests::selection_reads_only_the_saved_weights_in_order` (the map follows the saved list only) |
| A structure whose biome check straddles a border | Placement follows vanilla's check at its sample position. | `horizon-world` `worldgen::generator::region_tests::structure_biome_checks_read_the_region_at_their_own_quart` |
| World created with mod A, mod A removed | The missing mod and region are named; do-not-open or a recorded vanilla substitution is required. Dedicated servers refuse without the setting. Existing chunks are never touched. | `horizon-client` `save::biome_regions::tests::a_missing_mods_region_refuses_the_open_naming_mod_and_region`; `horizon-client` `save::biome_regions::tests::choosing_vanilla_is_saved_and_opens_from_then_on`; `horizon-client` `title::missing_biome_regions_tests::a_missing_biome_mod_asks_before_the_world_opens` (the prompt); `horizon-client` `game::interface::dedicated::biome_regions::tests::missing_mods_refuse_and_nothing_is_adopted_unless_set` and `horizon-server-launch` `config::tests::biome_regions_refuse_missing_mods_unless_their_section_says` (the server); `horizon-world` `worldgen::biome_source::region_selector_tests::a_substituted_region_resolves_through_vanilla_and_moves_no_border`. Existing chunks untouched: not covered by a test of its own; chunks are read from the region files, which the choice does not write. |
| Mod A returns after a substitution | Reported once; the substitution stays until the owner opts back in. | `horizon-client` `save::biome_regions::tests::choosing_vanilla_is_saved_and_opens_from_then_on` (the return is a difference and the record is unchanged). "Once" is not met: the report is a log warning at every open. |
| Mod A changes its climate points or weight | The saved points and weights win until a recorded opt-in; the difference is reported once. | `horizon-client` `save::biome_regions::tests::changed_added_and_removed_regions_are_reported_and_the_record_wins` |
| Mod B added to an existing world, no opt-in | No change in any generated or new chunk. | `horizon-client` `save::biome_regions::tests::changed_added_and_removed_regions_are_reported_and_the_record_wins` (the record, which generation reads, is unchanged). Not covered end to end through generated chunks. |
| Opt-in on an existing world | New chunks may use B's region; existing chunks unchanged; the saved list includes B afterwards. | `horizon-client` `save::biome_regions::tests::adopting_takes_installed_regions_and_keeps_saved_weights` (the saved list); the server setting in the dedicated test above. Existing chunks unchanged: not covered. The client opt-in is not built. |
| Malformed record | The open is refused with the reason; the file is unchanged on disk. | `horizon-client` `save::biome_regions::tests::a_malformed_record_is_refused_and_left_untouched` |
| A region with invalid points, a bad weight or a duplicate id | Refused at registration, naming the mod; no region installed. | `horizon-mod-vocab` `world::biome_region::tests::a_region_needs_a_preset_dimension_a_weight_and_valid_points`; `parts::biome_regions::tests::biome_region_declarations_validate_and_refuse_duplicates` |
| A wasm mod's region | Reaches the game as declared. | `horizon-mod-wasm` `wire_loopback_biome_region_declaration_reaches_the_host_bit_exact` |

## Stages

1. Saved biome ids kept when the game lacks them (#3132).
2. `BiomeRegion`, `declare_biome_region` and validation, with unit tests and a wire round trip. No generator change. **Built.**
3. Region selection in the biome source (`ResolvedBiomeSource`), behind "no record means today's path", with the oracle, distribution, order and structure fixtures. **Built.**
4. The saved record (`crates/horizon-client/src/save/biome_regions.rs`), world create and open, the missing-mod prompt, the server settings and the opt-in, with this acceptance matrix. **Built**, except the client opt-in button.
5. User docs ([biome regions](../how-to/biome-regions.md), [hosting](../../../hosting.md)), an example mod (`docs/modding/examples/src/biome_regions.rs`, one Overworld and one Nether region) and the tests named in the matrix. **Built.**
