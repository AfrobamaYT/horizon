# Injections

An `[[inject]]` attaches a typed listener to an injection point without compiling game code. Prefer a named hook or Part method when one exists. API points work on native and wasm, on engine functions.

## Declaration

```toml
[[inject]]
id = "count"
at = "horizon_client::game::Game<'_>::tick"
kind = "head"
with = "count"
access = "api"
priority = 0
```

`id`, `at`, `kind` and `with` identify the declaration, function, point kind and listener. `access` defaults to `api`; `priority` defaults to zero. An optional signature `hash` pins the target.

| Kind | Contract |
| --- | --- |
| `head` | Before the body; mutable argument record and cancellation/result |
| `return` | After the body; mutable returned value |
| `replace` | Owns the function and returns its result without running the body |

Listeners receive API records. Mutable parameters are copied back. A `Game` method's API point also supplies the side's Host. A parameter without an API/Wire form refuses registration, naming its type, on both targets.

A staged SDK build supplies these sites. A plain, unstaged game has no point sites and
refuses API listeners at load by name. The launcher selects the SDK build when any
enabled mod, native or wasm, declares an `[[inject]]` row. Each point is a replaceable
engine function with an API record: its head loads one replacement cell, and the
dispatcher for a listened point is installed through that cell.

## Ordering

Head cancellation skips the body and return point, but every head listener still runs and may clear cancellation. Return listeners run only after the body. Replacement listeners run after other API listeners; otherwise priority descends, then dependency registration order, with inject ID breaking ties within a mod. Only one replacement may own a point; disabling it restores the original body. Calling the replaced function from its replacement re-enters the point, not the original body.

Native code that needs game internals uses [`[[native.mixin]]`](mixins.md); `access = "internals"` on `[[inject]]` is refused by name.

A library mod publishes its own hooks and functions for other mods to use.

## Ask for the hook

If no suitable API point is available, request a named hook.
