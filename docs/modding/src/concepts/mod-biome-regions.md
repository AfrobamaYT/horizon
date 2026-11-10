# Design: a mod's biomes inside the Overworld and Nether

Provisional answer to scope 2 of [#2554](https://github.com/arismoko/horizon/issues/2554): how a mod's biomes can generate among vanilla's in the Overworld and the Nether. This is a proposal, not an implemented API. Scope 1, a mod's own dimension with its own climate points (`BiomeSourceKind::Points`), is separate and does not need this note.

## Decision from the issue

Ari's answer on #2554 sets two starting points, and this note keeps both:

- **Regions with weights.** Each mod lays out its biomes in its own region. A hidden extra parameter decides which region owns an area, and vanilla's layout is one region among them. Ordering between mods is settled by the regions, not by competing for the same climate points.
- **The mix is fixed when the world is created and saved with the world.** Installing or removing a mod never changes terrain in an existing world unless the player opts in for that world.

The same answer says NeoForge has no API for this: its biome modifiers change biomes that already generate and cannot add a biome to a dimension's layout, and the region mechanism is a third-party library. So there is no NeoForge contract to mirror. That statement comes from the issue; this note did not verify it, and it did not read the third-party library's source. Behaviour of that library is therefore not claimed anywhere below.

## Horizon today

| Fact | Where |
| --- | --- |
| A level's biome source is one of fixed, checkerboard, multi-noise by preset, the End, or (scope 1) a mod's own points. | `BiomeSourceKind` in `crates/horizon-mod-api/src/world/dimension.rs` |
| The game writes the level stem's `biome_source` JSON; the generator decodes it when a world is prepared. | `biome_source` in `game/modding/native_dimensions.rs`; `worldgen/biome_source/codec.rs` |
| A multi-noise source is a list of `(ParameterPoint, biome)` searched for the nearest point to the six climate values and a zero offset. | `worldgen/climate/index.rs` (`ClimateIndex`, `ClimateSearch`) |
| The Overworld's and the Nether's lists are named presets, not data a mod can extend. | `BiomeSourceKind::MultiNoise`, `biome_preset` |
| A chunk stores its biomes, so a chunk that already exists keeps them whatever the source says later. | not checked in code for this note; vanilla stores biomes per chunk section, treat as **unverified here** |

Not verified for this note: how vanilla's `MultiNoiseBiomeSource` presets are built from parameter lists, whether terrain depends on a chunk's biomes or only on the climate router, and how the existing preset lists are cached.

## Proposal

**Region.** A region is `(owner mod, region id, integer weight, points)`, where `points` are the `BiomePoint`s of scope 1. Vanilla's own list is the region `minecraft:vanilla` with a fixed default weight. The declaration is a load-time registrar call, `Registrar::declare_biome_region(path, weight, points)`, namespaced to the declaring mod and refused for a malformed or duplicate id, a weight of 0, or invalid points, as `declare_attribute` is.

**Order.** The region list is sorted by owner mod id, then region id, the rule the attribute catalog uses (#2191). The same mods give the same order on every side, so it can be saved and compared.

**Choosing a region.** A low-frequency noise of the world seed, sampled per biome cell, picks a region by cumulative weight. The nearest-point search then runs inside that region's own index only. A region never competes with another region's points, which is what makes ordering between mods unnecessary. The noise's exact form, frequency and its random-stream position are open (see questions) and must not disturb any vanilla stream: a world with no regions declared must generate exactly as today.

**Fixed at creation.** When a world is created the ordered region list (ids, weights and a hash of each region's points) is written into the world's data. On load the game compares it with the installed regions:

- the same list: nothing changes;
- a mod removed or its points changed: areas not yet generated use the saved list's region, which no longer exists, so they fall back to the vanilla region, and the game says so in the log and on the world's entry. Generated chunks keep their stored biomes;
- a mod added: ignored for that world unless the player opts in.

**Opt-in for an existing world.** A per-world switch, off by default, adds the installed regions to the saved list. Chunks that already exist keep their stored biomes, so the border between old and new chunks can show a biome seam. Whether terrain shape can also seam depends on whether terrain reads the biome (open, see questions).

**API surface.** One registrar method, one `BiomeRegion` record with `Wire`, and a read of the saved list (`level.biome_regions(host)`) for mods that want to know what the world was created with. No new hook.

## Acceptance matrix (proposed, not reported passing)

| Fixture | Expected |
| --- | --- |
| No region declared | Biome output of the vanilla presets is identical to today for a fixed seed, at several coordinates. |
| Two mods, weights 1 and 3, vanilla default | Over a large sample, region ownership follows the weights within a stated tolerance; the same seed gives the same map twice. |
| Same mods installed in a different order | Same sorted list, same map. |
| World created with mod A, mod A removed | Existing chunks keep their biomes; ungenerated areas use vanilla; the log names the missing region. |
| Mod B added to an existing world, no opt-in | No change in any generated or new chunk. |
| Opt-in on an existing world | New chunks may use B's region; existing chunks unchanged; the list saved afterwards includes B. |
| A region with invalid points or a duplicate id | Refused at registration, naming the mod; no region installed. |

## Open questions

- What noise picks the region, and how is its seed offset kept from moving any vanilla random stream? Recommendation: a dedicated noise instance seeded from the world seed and a fixed salt, with the same sampling grid as the biome source, verified against the vanilla presets as an oracle.
- Does terrain shape read the biome, or only the climate router? Recommendation: check before the opt-in border is promised to be seam-free.
- Is falling back to vanilla for a removed mod's areas right, or should a world with a missing region refuse to open? Recommendation: fall back and warn; a refusal punishes removing a mod.
- Should weights be integers with a fixed default for vanilla, or relative to it? Recommendation: integers, vanilla at a fixed default, written down in the API docs.
- Nether, Overworld and the End: should the End be included? Recommendation: Overworld and Nether first, as the issue asks.
- Where is the saved list stored (level data or a separate file), and how does it round-trip a mod's version change? Recommendation: decide with the world-save owners before any code.

## Stages

1. Settle the open questions above, with oracle fixtures for the presets.
2. The registrar, `BiomeRegion` and validation, with unit tests and a wire round trip (no generator change).
3. Region selection inside the biome source, behind "no region declared means identical output".
4. Saved list, load comparison and the per-world opt-in.
5. Docs, an example mod and the acceptance matrix.
