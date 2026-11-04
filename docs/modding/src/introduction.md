# Modding Horizon

Horizon is Minecraft: Java Edition 26.3 rewritten in Rust. A Horizon mod is one `.hmod` package. It is one of:

| Kind | Holds | Runs |
| --- | --- | --- |
| **Content mod** | `assets/` and `data/`: blocks, items, recipes, loot, tags, models, textures | read when the game starts |
| **Native mod** | a Rust crate (`Cargo.toml`, `src/`) beside the content, using the mod API | compiled into a game the launcher builds for the player |
| **WebAssembly mod** | a `mod.wasm` beside the content, built from the same Rust source | in the game's one sandboxed runtime, with no rebuild |

Native and WebAssembly mods are written against one API, `mod_api`, and share one dependency model: the only
difference between them is the build target.

## How this book is organised

The book follows [Diátaxis](https://diataxis.fr): each page is one of four kinds, and says so by where it lives.

The book is being filled in by kind: tutorials, concepts, how-to guides and reference. Today it holds the
[injections](injections.md) how-to; the pages in `docs/native-mods.md`, `docs/mods.md` and `docs/api/` move here as
they are rewritten. The reference is the generated rustdoc of the `horizon-mod-api` crate.

An [agents page](agents.md) and `llms.txt` give a coding agent the rules in one place.

## Where things are

- The API reference is `cargo doc -p horizon-mod-api`: every hook, event and `Host` method, with its side,
  its NeoForge 26.3 counterpart and its cancel contract.
- The end-to-end example mod is
  [horizon-machine-sample](https://github.com/arismoko/horizon-machine-sample).
- Design and milestones: [#26](https://github.com/arismoko/horizon/issues/26) and
  [#44](https://github.com/arismoko/horizon/issues/44).
