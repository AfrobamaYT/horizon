# Building a mod to `mod.wasm`

A mod written against `mod_api` builds either way from the same source: native, compiled into
the game by the launcher ([Native mods](native-mods.md)), or to `mod.wasm`, which any game with
the same mod API runs with no rebuild. Only the build step differs: the hooks, Parts,
`[dependencies]` and the manifest mean the same for both.

```rust,ignore
{{#include ../../examples/wasm-hello/src/lib.rs:mod}}
```

## Build

```sh
cargo horizon-mod build --wasm
```

It builds the crate with the pinned Rust toolchain (installed through rustup the first time, with
`rust-src`), writes `mod.wasm` beside `horizon-mod.toml`, records it in a `[wasm]` section, and
packs `target/horizon-mod/<id>.hmod`. `[wasm]` is written by the tool, never by hand:

```toml
[wasm]
sha512 = "…"                      # mod.wasm's hash, checked when the game loads it
toolchain = "1.99.1 <commit>"     # every mod linked together needs the same one
[wasm.linked]                     # the exact builds of its wasm dependencies
my_library = "…"
```

A mod is `[native]` or `[wasm]`, not both. A wasm mod's id is words that each start with a
letter (`hello_wasm`, not `mod_2`).

## Dependencies

`[dependencies]` work as for native mods. List the library mod in `horizon-mod.toml` and its
crate in `Cargo.toml`; build the library with `--wasm` first. The game links a mod with its
dependencies at load, one copy of each library shared by every mod that uses it, and refuses a
mod whose library is missing or another build than the one it was linked against, naming it on
the Mods page.

## In the game

The game loads every `mod.wasm` turned on in the Mods page (or passed with `--dev-mod`) at start,
after the native mods. Each runs sandboxed with a fuel budget per call and a memory limit; a mod
that traps stops its group, not the game. `--wasm-interpreter` runs them interpreted, for hosts
that can't compile code at run time; it is much slower.
