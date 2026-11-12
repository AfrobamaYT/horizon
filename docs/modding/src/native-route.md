# The native route

The native route lets a native mod change the game's own code. You declare it with tables under
`[native]` in `horizon-mod.toml`:

| Table | What it does | Page |
| --- | --- | --- |
| `[[native.mixin]]` | Changes one game function: runs code at its start or end, changes a value or a call inside it, or replaces its body. | [Mixins](mixins.md) |
| `[[native.loop]]` | Puts your type into a slot of a hot loop the game publishes, such as the block entity tick. | [Published loops](loops.md) |
| `[[native.bridge]]` | Gives your mixin code a getter, setter or caller for a private field or method of a game type. | [Bridges](bridges.md) |
| `[[native.module]]` | Adds a module of your Rust code inside a game source file, for mixin code to share. | [Support modules](support-modules.md) |
| `[[native.state]]` | Would add a field to a game type. Refused: use attachments. | [Native state](native-state.md) |

## Try the stable route first

Most mods never need this page. The stable route is the same on native and wasm and survives game updates:

1. A **hook** (a method of `mod_api::Hooks`) or a **Part** method, when one covers the moment
   ([Your first native mod](start/first-mod.md)).
2. An **`[[inject]]` listener**: your function runs at the start or end of an engine function and gets its
   arguments as `mod_api` values ([Injections](injections.md)). Ask for a hook at the same time.

Take the native route when neither can do it: you need the game's real types and private fields, a value
or a call in the middle of a function, a whole function replaced, or a change inside a hot loop.

Don't take it when:

- the mod must also run as wasm: a wasm mod cannot declare `[native]` tables;
- you want the mod to keep working on the next game build without a rebuild;
- you want per-object data: that is an [attachment](concepts/trait-mods.md#server-attachments), on native
  and wasm alike ([Native state](native-state.md) says why).

## Unstable on purpose

Code on this route is compiled into copies of the game's functions, so it sees the game exactly as it is in
one build:

- It is pinned to the exact game commit in `[native] horizon`. A mod built for one commit is rebuilt for the
  next. Each build copies the targeted functions from the new source. When a target was renamed, moved, or
  its selected call or types changed, the build fails with a refusal or a compiler error under your mod's id.
  A change elsewhere in a target's body is not reported: your mixin applies to the new body, so reread the
  functions you target when you move to a new commit.
- It gets the game's own types (`Game`, its fields, its block entity map), not `mod_api` values. Those types
  change between builds without notice.
- It is trusted native code with the player's rights, not a sandbox.

## The `[native]` table

Every native mod already has it. The tables on this route go below it:

| Field | Value |
| --- | --- |
| `game` | The Minecraft version, `"26.3"`. |
| `horizon` | The exact Horizon commit the mod builds against, 40 lowercase hex characters. |
| `mixins` | The exact commit of the [horizon-mixins](https://github.com/horizon-rust/horizon-mixins) tool that applies `[[native.mixin]]` rows, 40 lowercase hex characters. |

`cargo horizon-mod new` writes all three. The rows of every `[[native.*]]` table and every `[[inject]]` share
one id space: two rows of one mod never share an `id`.

## The mod's crate

A mod on this route is an ordinary native mod ([Your first native mod](start/first-mod.md)): a Cargo package
beside `horizon-mod.toml`, depending on `horizon-mod-api`, with a type implementing `mod_api::Hooks` (it may
implement no hook at all). The examples in this section come from the **check mods**: small native mods in
`tools/ci/fn-replace-check/` that CI builds against the prebuilt game on every change to this route, so the
examples stay true. This is the `Cargo.toml` of the check mod `fnr_alpha` (the check fills `rev` with the
game commit; yours names your `horizon` commit):

```toml
[package]
name = "fnr-alpha"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
horizon-mod-api = { git = "https://github.com/arismoko/horizon", rev = "HEAD" }
```

and its `src/lib.rs`:

```rust,ignore
//! The mod's own crate. Its `[[native.mixin]]` helpers (`src/mixins/`) are compiled into copies of the
//! game functions they patch, not into this crate.
#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl mod_api::Hooks for Mod {}

/// Called by a helper: a helper may use its mod's crate.
pub fn note(what: &str) {
    eprintln!("fnr: alpha {what}");
}
```

The code your `[[native.*]]` rows name (helpers, slots, modules) lives in files under `src/` that this crate
does not declare as modules: the launcher compiles them into the copies of the game's functions. They call
the mod's crate by its Rust name, the package name with `-` turned into `_` (`fnr_alpha::note`); the crate
name need not match the mod's `id`.

## What a build does

The game itself is never rebuilt. When you build the mod (the launcher's **Build**, or
`horizon-launcher native-build path/to/mod --trust-native-code`), the launcher downloads the prebuilt game
for your `horizon` commit and its source. It copies each function your rows target out of that source,
applies your mixins to the copies with horizon-mixins, adds your loop slots, bridges and modules, compiles the
copies against the prebuilt game together with your mod's crate, and swaps each copy in for the original
through a **replacement cell**: a pointer the prebuilt game checks at the start of every replaceable function,
which runs the installed copy instead when one is set. What can't be done this
way is refused by name before anything compiles
([What the launcher does](how-to/native-mods/028-what-the-launcher-does.md#what-the-launcher-does)).

The launcher's build log (`native/build.log`) shows the route and how long each phase took:

```text
Prebuilt game: only the mods compile
Function replacement: 1 functions patched by 1 mixins, without rebuilding the game (copy … ms, horizon-mixins … ms, compile … ms, total … ms)
```

The first build with a `[[native.mixin]]` row also downloads the horizon-mixins tool, once per pinned tool (a
mod with only loops, bridges and modules never needs it). It comes prebuilt from this repository's
`mixins-tool-…` release, checked against that release's `SHA256SUMS` and against the commit, patch and platform
the launcher pins; that is protection against a damaged download, the same as the prebuilt game's, not a
signature. Where the release has no build for your platform (an Intel Mac, a pin not released yet), the launcher
downloads the tool's source and builds it instead, which took about a minute on a test machine, and the build log
says so: `horizon-mixins tool: no prebuilt for <target>; building from source (once per pin)`. A download that
fails is an error to retry, never a source build. A newly pinned tool's prebuilt is published by a run on
main; a pin replaced while its run waited gets its prebuilt from a manual dispatch, and until then the launcher
builds the tool from source. Later builds reuse the tool, and a build with a mixin, a loop
slot, a bridge and a module took a few seconds on that machine.
