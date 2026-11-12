# The native route

The native route lets a native mod change the game's own code: a function's body, a value or call inside
it, or a hot loop. You declare it with tables under `[native]` in `horizon-mod.toml`, and the launcher
applies them to the prebuilt game without rebuilding the game.

## When to use it

Try these first. They work on native and wasm, and they keep working on the next game build:

1. a **hook** or **Part** method ([Your first native mod](start/first-mod.md));
2. an **`[[inject]]` listener**, which runs your function at the start or end of an engine function
   ([Injections](injections.md)).

Take the native route when neither can do it: you need the game's real types or private fields, a change in
the middle of a function, a whole function replaced, or a change inside a hot loop. It is not for a wasm mod,
which cannot declare `[native]` tables. Per-object data is an [attachment](concepts/trait-mods.md#server-attachments).

## The tables

| Table | Use it to | Page |
| --- | --- | --- |
| `[[native.mixin]]` | run code at the start or end of a game function, change a value or a call inside it, or replace its body | [Mixins](mixins.md) |
| `[[native.loop]]` | run code before and after each entry of a hot loop the game publishes, such as the block entity tick | [Published loops](loops.md) |
| `[[native.bridge]]` | read, write or call a private field or method of a game type from your mixin code | [Bridges](bridges.md) |
| `[[native.module]]` | add a module of your own code inside a game source file, for your mixin code to share | [Support modules](support-modules.md) |
| `[[native.state]]` | add a field to a game type: refused, use an attachment | [Native state](native-state.md) |

Every row of these tables has an `id`. No two rows of one mod share an `id`, counting `[[inject]]` rows too.

## It is tied to one game build

A mod on this route builds against one exact game build: the commit in `[native] horizon`. Your code gets
the game's own types, such as `Game` and its fields, and these change between builds without notice.

- To move to a new build, change `horizon` and rebuild. A target that was renamed or moved fails the build
  with an error naming your mod.
- A change elsewhere in a target's body is not reported. Reread the functions you target on each new build.
- This code runs with the player's rights. It is not sandboxed.

## The `[native]` table

| Field | Value |
| --- | --- |
| `game` | The Minecraft version, `"26.3"`. |
| `horizon` | The game build: the exact Horizon commit, 40 lowercase hex characters. |
| `mixins` | The exact commit of the [horizon-mixins](https://github.com/horizon-rust/horizon-mixins) tool, which applies `[[native.mixin]]` rows; 40 lowercase hex characters. |

`cargo horizon-mod new` writes all three. Below them, the mod is an ordinary native mod
([Your first native mod](start/first-mod.md)).

## Building

Build as for any native mod: the launcher's **Build**, or
`horizon-launcher native-build path/to/mod --trust-native-code`.

The first build with a `[[native.mixin]]` row gets the horizon-mixins tool once for your `mixins` commit:
a prebuilt download where one is published, otherwise a build from source that takes about a minute. Later
builds take a few seconds.

When it works, the launcher's build log (`native/build.log`) shows:

```text
Prebuilt game: only the mods compile
Function replacement: 1 functions patched by 1 mixins, without rebuilding the game (copy … ms, horizon-mixins … ms, compile … ms, total … ms)
```

What the launcher does under the hood is on [How the native route works](native-how-it-works.md).
