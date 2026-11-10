# Biomes

What a mod can do with biomes, by what it wants. Each row names the call or file; the pages it links to give the detail.

| I want to | Use | Side |
|---|---|---|
| Add a biome | `data/<ns>/worldgen/biome/<name>.json` in a content mod or data pack ([content mods](content-mods.md)) | Content |
| Add my features or spawns to vanilla's biomes | A biome modifier file in `data/<ns>/neoforge/biome_modifier/` ([content mods](content-mods.md)) | Content |
| Give my dimension its own biomes | `DimensionSettings::biome_source`: `Fixed`, `Checkerboard`, `MultiNoise` by preset, or `End` ([dimension shape](native-mods/008-dimension-shape.md)) | Both |
| Know the biome at a position | `LevelRef::biome_at` on a server level, `LevelRef::client_biome` on a client | Server, client |
| Find the nearest biome or tag | `LevelRef::find_closest_biome` with a `BiomeSearch` | Server |
| Change a biome | `LevelRef::set_biome`, one 4x4x4 cell, as `/fillbiome` does | Server |
| Run bone meal at a position | `LevelRef::bone_meal` | Server |
| Tint foliage, grass or water by biome | The biome colour resolvers a mod registers ([client effects](client-effects.md)) | Client |

## What is not here yet

- Placing a mod's own biomes among the Overworld's or the Nether's: a design note, not code ([arismoko/horizon#2554](https://github.com/arismoko/horizon/issues/2554)).
- Biome modifiers that change carvers, climate or effects, and explicit phases between modifiers ([#2732](https://github.com/arismoko/horizon/issues/2732)).
- A mod's own worldgen registry loaded into the world ([#811](https://github.com/arismoko/horizon/issues/811)), and per-biome client effects ([#2734](https://github.com/arismoko/horizon/issues/2734)).
- Features and placers defined in code ([#815](https://github.com/arismoko/horizon/issues/815)).

Some of these have a PR open; a page for each lands with it.

## Reads and writes are limited on purpose

- `biome_at` never loads or generates a chunk: an unloaded column answers `None`.
- `find_closest_biome` follows vanilla's sampled spiral, so it is not a search through cells a mod edited, and it refuses a search whose sample count passes `BiomeSearch::MAX_SAMPLES`.
- `set_biome` refuses a client, an unknown biome and an unchanged cell, and returns false rather than an error.
