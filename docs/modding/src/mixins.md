# Mixins: `[[native.mixin]]`

A mixin changes one game function. You name the function, say where in it your code goes (its start, its
return value, one call inside it, or the whole body), and write that code as a small Rust function, the
**helper**. The launcher copies the game function, puts your helper into the copy, and installs the copy in
place of the original. Read [The native route](native-route.md) first: this is the unstable route, pinned to
one game build.

## A first mixin

This mixin runs at the start of the game's `on_tick_rate_changed` (the server's `/tick rate` handler), reads two
of `Game`'s private fields, and can cancel the rest of the function. It is the `head` row of the check mod
`fnr_alpha`, which CI builds against the prebuilt game.

`horizon-mod.toml` (the check fills `horizon` and `mixins` with exact commits; yours has the 40-character
commits `cargo horizon-mod new` wrote):

```toml
id = "fnr_alpha"
name = "Function replacement check: alpha"
version = "0.1.0"
description = "fn-replace: HEAD with cancel and a private field, a value chain, RETURN, OVERWRITE"
authors = []
api = "0.2"
side = "both"
capabilities = ["registries"]

[native]
game = "26.3"
horizon = "HEAD"
mixins = "MIXINS"

# HEAD, cancellable, reaching a private field (`ticks_until_autosave`) and the clock.
[[native.mixin]]
id = "head"
target = "horizon_client::game::autosave::Game<'_>::on_tick_rate_changed"
kind = "HEAD"
cancellable = true
helper = "src/mixins/head.rs"
bindings = [{ name = "game", ty = "&mut Game<'_>", value = "&mut *self" }]
```

`src/mixins/head.rs`, the helper:

```rust,ignore
fn head(game: &mut Game<'_>) -> ::core::ops::ControlFlow<()> {
    let rate = game.clock.rate();
    fnr_alpha::note(&format!(
        "HEAD on_tick_rate_changed: rate {rate}, ticks_until_autosave was {}",
        game.ticks_until_autosave
    ));
    if rate == 13.0 {
        fnr_alpha::note("HEAD cancels on_tick_rate_changed at rate 13");
        return ::core::ops::ControlFlow::Break(());
    }
    game.ticks_until_autosave = 4242;
    ::core::ops::ControlFlow::Continue(())
}
```

`src/lib.rs`, the mod's own crate (package `fnr-alpha`, so `fnr_alpha` in Rust), which the helper calls:

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

What happens:

- `bindings` passes the target's `self` to the helper's `game` parameter. The helper is compiled inside the
  copy of `on_tick_rate_changed`, in the game's module `game::autosave`, so `Game` and its private fields
  `clock` and `ticks_until_autosave` resolve exactly as they do in the game's source.
- `cancellable = true` makes the helper return `ControlFlow<()>`: `Break(())` returns from the function before
  its first line, `Continue(())` lets it run.
- On success the build log prints `Function replacement: 1 functions patched by 1 mixins, without rebuilding
  the game (…)`, with the time of each phase.

## A helper

A helper file holds exactly one private, safe, non-generic function with no attributes. Its parameters are
filled by `bindings`, one entry per parameter, in order. It is compiled inside the copied function's module:

- It sees what the target's module sees: the game's types, private fields and functions visible there. A
  member it can't see is reached through a [bridge](bridges.md).
- `crate::` is the game crate (`horizon_client`), as in the game's own source.
- It calls your mod's crate by its crate name (`fnr_alpha::note`), so longer code lives there. Code that
  needs the game's private items and is shared by several helpers goes in a [support module](support-modules.md).
- It cannot see the target's local variables unless a binding captures them (`$local:NAME:ORDINAL`).

A helper with a wrong type is an ordinary compiler error, reported under your mod's id.

## Where the code goes: kinds

`kind` is one of horizon-mixins' kinds, written in capitals:

| Kind | Your helper runs | Helper returns |
| --- | --- | --- |
| `HEAD` | at the start of the function | `()`, or `ControlFlow<()>` with `cancellable = true` |
| `RETURN` | on the value the function returns (`$value`) | the value to return |
| `OVERWRITE` | instead of the whole body; one mod per function | the function's return type |
| `BEFORE_CALL`, `AFTER_CALL` | before or after one call in the body | `()` |
| `AFTER_ASSIGN` | after one assignment in the body | `()` |
| `MODIFY_ARG`, `MODIFY_ARGS` | on one argument, or all arguments as a tuple, of one call | the new argument(s) |
| `MODIFY_CONSTANT` | on one literal in the body | the new value |
| `MODIFY_VARIABLE` | on one read or write of a local or parameter (`variable = { … }`) | the new value |
| `MODIFY_EXPRESSION_VALUE` | on the value of one call in the body (`$value`) | the new value |
| `WRAP_OPERATION` | around one call: you get the call as `$operation` and decide whether to run it | the call's type |

The kinds that act on one place in the body find it with a `selector`, the call as written in the source
(`autosave_interval`, `self.next_autosave_interval`), plus `require` (how many matches the original body has;
the build stops if it differs) and `ordinal` (which one, from 0). This is the check mod `fnr_beta`, at two
places in `next_autosave_interval`, which ends in `autosave_interval(rate)`:

```toml
[[native.mixin]]
id = "plus_one"
target = "horizon_client::game::autosave::Game<'_>::next_autosave_interval"
kind = "MODIFY_EXPRESSION_VALUE"
selector = "autosave_interval(rate)"
require = 1
ordinal = 0
priority = 100
helper = "src/mixins/plus_one.rs"
bindings = [{ name = "value", ty = "i64", value = "$value" }]

[[native.mixin]]
id = "wrapped"
target = "horizon_client::game::autosave::Game<'_>::next_autosave_interval"
kind = "WRAP_OPERATION"
selector = "autosave_interval"
require = 1
ordinal = 0
helper = "src/mixins/wrapped.rs"
bindings = [
  { name = "rate", ty = "f32", value = "$arg0" },
  { name = "operation", ty = "impl FnOnce(f32) -> i64", value = "$operation" },
]
```

```rust,ignore
fn plus_one(value: i64) -> i64 {
    fnr_beta::note(&format!("MODIFY_EXPRESSION_VALUE (priority 100): {value} -> {}", value + 1));
    value + 1
}
```

```rust,ignore
fn wrapped(rate: f32, operation: impl FnOnce(f32) -> i64) -> i64 {
    fnr_beta::note(&format!("WRAP_OPERATION autosave_interval({rate})"));
    operation(rate)
}
```

The full contract of each kind, selector and binding is horizon-mixins' own, at the commit in
`[native] mixins`: its [Mixin API](https://github.com/horizon-rust/horizon-mixins/blob/e59da9094f0c2fdda6c627fe9eff22dec6a906f0/docs/mixin-api.md),
[advanced injections](https://github.com/horizon-rust/horizon-mixins/blob/e59da9094f0c2fdda6c627fe9eff22dec6a906f0/docs/advanced-injections.md)
and [scopes and bridges](https://github.com/horizon-rust/horizon-mixins/blob/e59da9094f0c2fdda6c627fe9eff22dec6a906f0/docs/scopes-and-bridges.md).
The launcher applies one small patch to that tool, `tools/mixins/narrow-reference-rule.patch`, which accepts a
call directly under `&` for the kinds that rewrite only that call.

## Fields

| Field | Required | Meaning |
| --- | --- | --- |
| `id` | yes | The row's name, unique in the mod across every `[[native.*]]` and `[[inject]]` row. Errors name it as `mod:id`. |
| `target` | yes | The game function, as a path (below). |
| `kind` | yes | One of the kinds above. |
| `helper` | yes | The helper's file, a `.rs` path under `src/`. |
| `bindings` | when the helper has parameters | One `{ name, ty, value }` per helper parameter, in order. `name` and `ty` repeat the parameter; `value` is a target parameter or its borrow (`self`, `&*self`, `&mut *self`, `rate`), or a tool value: `$value`, `$argN`, `$args`, `$operation`, `$local:NAME:N`, `$local_ref:NAME:N`, `$local_mut:NAME:N`. |
| `cancellable` | no | `HEAD` (and some call-site kinds): the helper returns `ControlFlow` and can end the function. |
| `selector`, `require`, `ordinal` | for one-place kinds | Which call, how many matches, which match. |
| `variable` | `MODIFY_VARIABLE` | `{ name, ordinal, at = "LOAD" or "STORE", parameter = true for a parameter }`. |
| `priority` | no | Orders rows of several mods at one place. |
| `slice`, `argument`, `result` | no | Narrow the search region, pick an argument, give a call's result type; see the tool's docs. |

## Naming the target

A target is the path of the function in the game's source:

- **A method:** the module its `impl` block is in, the type, the function:
  `horizon_client::game::autosave::Game<'_>::on_tick_rate_changed`.
- **A free function:** `horizon_client::game::commands::misc::millis`.
- **A trait method:** `horizon_client::game::<GameLevel<'_> as LevelAccess>::block_state`.

The module is the Rust module, not the folder. In the game crate, the folders under `src/game/` group files
but are not modules: `crates/horizon-client/src/game/world/autosave.rs` is the module
`horizon_client::game::autosave`. Read the source at your `horizon` commit to find a function.

## Which functions can take a mixin

- **Every function of the game crate, `horizon_client`,** without any marker. When the prebuilt game is built,
  each of them gets a replacement cell automatically; that is why the source you read has no annotation on
  them.
- **A function in a crate below the game** (`horizon_client_lib`, `horizon_client_render`, `horizon_world`
  and the others) only when its source carries `#[horizon_mod_api_macros::replaceable]`. Search for that
  marker at your commit; to get one added, ask in an issue or pull request on the game.

Either way the function must be one a copy can stand for. These are refused, with the reason named:
a function generic over types or consts or in such an `impl`; one taking `impl Trait` or returning `!`;
`const`, `async` or `unsafe` functions; `self` taken by value; `#[inline(always)]`; a function or `impl` that
exists only under a `cfg`; a body that declares a `static`, a thread-local, a `macro_rules!` or an `impl`, or
uses a macro defined elsewhere in its crate. The one generic function you can target is a published loop's,
covered on [Published loops](loops.md#what-composes-with-a-loop).

## With other mods and with `[[inject]]`

Rows of several mods on one function are applied to one copy, in the tool's order. Only one mod may
`OVERWRITE` a function. An `[[inject]]` listener and a mixin may share a function: the API head listeners run
first, then the patched copy, then the API return listeners.

## Refusals

A row the build can't apply stops the build and names the mod, the row and the reason. Nothing falls back to
another route. In the texts below, `<…>` is filled in.

| Code | Text | Fix |
| --- | --- | --- |
| `E_MIXIN_INVALID` | `horizon-mod.toml: E_MIXIN_INVALID: <mod>:<row>: invalid target; name the game function's path` | Write the target. |
| | `… invalid helper; name a Rust file under src/ in this package` | Put the helper under `src/`, ending in `.rs`. |
| | `… invalid hook; a named hook is a mod_api::Hooks method; a mixin names its target` | Use `target`, or implement the hook in `Hooks`. |
| | `… invalid target; a mixin targets a game function` | The target names one of your dependencies; a mod uses another mod's published functions instead. |
| `E_MIXIN_TARGET_MISSING` | ``E_MIXIN_TARGET_MISSING: <mod>:<row>: <target>: no function `<name>` in <module> on <Type>`` (or ``no crate `<crate>` in the game's source``) | Check the path against the source at your commit, module first (above). |
| `E_MIXIN_TARGET_AMBIGUOUS` | `E_MIXIN_TARGET_AMBIGUOUS: <mod>:<row>: <target>: several functions have this path (a cfg-gated pair, for one)` | Target another function. |
| `E_MIXIN_NOT_REPLACEABLE` | `E_MIXIN_NOT_REPLACEABLE: <mod>:<row>: <target>: a function below the game crate takes a replacement only when it carries #[horizon_mod_api_macros::replaceable]; ask for it in an issue or pull request on the game` | Target a game-crate function, or ask for the marker. |
| | `E_MIXIN_NOT_REPLACEABLE: <mod>:<row>: <target>: <reason>`, such as ``it is generic over types or takes `impl Trait`: one copy can't stand for every instance`` | Target a caller or callee that can be copied. |
| `E_MIXIN_SOURCE` | `E_MIXIN_SOURCE: <mod>:<row>: <target>: <why>` | The pinned source could not be read: retry the build. |
| `E_OVERWRITE_CONFLICT` | from the tool: `OVERWRITE must be exclusive on <function>; competing injections: <mod>/<row>, …` | Only one of the mods may stay installed. |
| `E_LOOP_ENTRY_REPLACED` | See [Published loops](loops.md#refusals). | |
| `E_MIXINS_PATCH` | `E_MIXINS_PATCH: tools/mixins/narrow-reference-rule.patch does not apply to horizon-mixins <commit>: <why>` | Not your mod: report it on the game. |

The tool's own codes (`E_MATCH_COUNT`, `E_UNSUPPORTED_SITE` and others) mean a selector, count or binding
does not fit the body; its docs above list them.
