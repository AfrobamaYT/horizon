# Injections

An injection is a `[[native.injection]]` table in `horizon-mod.toml`. It changes the game's Rust source
before it is compiled, using [horizon-mixins](https://github.com/horizon-rust/horizon-mixins). It is an
escape hatch for a moment the mod API has no hook for, and **every injection is a request for a hook**:
write it so it can become one by renaming its target, then [ask for the hook](#ask-for-the-hook).

Injections are **native-only**. A `mod.wasm` has no injections; a mod that needs one cannot be a wasm mod
until the hook exists.

## For agents

Apply these rules in order and stop at the first that fits.

1. A hook or a Part method exists for the moment: implement it. **No injection.**
2. No hook, but the moment is a catalog hook point (declared under
   `crates/horizon-mod-api/src/hooks/`): `hook = "<name>"`, `hook_version = <N>`, with `trait_method`.
3. No hook point: `trait_method`, `kind`, `target`, and one binding
   `{ name = "host", ty = "&mut dyn crate::mod_api::Host", value = "&mut *self" }`. The body uses only
   `mod_api` types (handles, attachments, services). This is the default injection.
4. You need private game internals: `helper = "src/<file>.rs"` bound to `&mut Game`. Last resort. It breaks
   on game updates and never runs as `mod.wasm`. File a hook request with it.

Exact fields (unknown fields are refused):

| Field | Rule |
| --- | --- |
| `id` | unique, nonempty |
| `kind` | `HEAD`, `RETURN`, `MODIFY_ARG`, `MODIFY_EXPRESSION_VALUE` for `trait_method`; any horizon-mixins kind for `helper` |
| `hook` + `hook_version` **or** `target` | exactly one of the two forms |
| `trait_method` **or** `helper` | exactly one of the two |
| `bindings` | `{ name, ty, value }`; `value = "$value"` names the value a `RETURN`, `MODIFY_ARG` or `MODIFY_EXPRESSION_VALUE` trait injection changes |
| `cancellable` | `true` only on a `HEAD` trait injection of a unit-returning target; the method returns `Flow` |
| `priority`, `selector`, `require`, `ordinal`, `argument` | horizon-mixins' own fields |

Errors: `E_HOOK_UNKNOWN` (the `hook` name is not in the catalog: use a name the error lists, or use `target`);
`E_HOOK_VERSION` (the catalog lacks that `hook_version`: use a version the error lists, and rebuild against the
game the mod pins).

Naming: events and methods follow NeoForge 26.3 names with no `on_` prefix; the event is `&mut`, handles come from
the event, and writes take `host`.

## The decision ladder

1. **A hook or Part method exists: use it.** Read [Hooks](concepts/trait-mods.md) and the
   [rustdoc](reference/api.md). A per-block, per-item or per-entity moment is a Part method. Nothing in this
   chapter applies.
2. **No hook, but the moment is a named hook point: `hook = "<name>"`.** The catalog names each hook point and
   the version of its contract. horizon-mixins resolves the name against the game it builds, so the hook
   point can move in the source without breaking your mod. The launcher tells you when a raw `target` is
   one (see [the build log](#the-build-log)).
3. **No hook point: a `trait_method` injection with a `Host` binding.** The default. Your method receives an
   event with the `Host` in it and uses only `mod_api` types.
4. **You truly need private internals: a `helper` bound to `Game`.** The last resort. It reads game state no
   API exposes, so it breaks when the game changes, can never run as `mod.wasm`, and should always come with
   a hook request.

## The forms

| Form | Kinds | Crates | Order | `mod.wasm` |
| --- | --- | --- | --- | --- |
| `trait_method` | `HEAD` (listen, or cancellable), `RETURN`, `MODIFY_ARG`, `MODIFY_EXPRESSION_VALUE` | `horizon-client` only | priority descending, then mod id, then injection id | no |
| `helper` | any | any crate | priority descending, then mod id, then injection id | no |
| `hook` instead of `target` | as the form it combines with | the hook point's | as the form | no |
| Hook or Part method (no injection) | — | — | dependency order, after every injection | yes |

A helper and a trait injection are one list at a place: they sort together. A trait injection's generated
helper calls only its own mod's method.

## Ordering

```text
a call site in the game
 │
 ├─ injections at the place (helpers and trait injections together)
 │    priority descending → mod id → injection id
 │    an early return, or a claimed cancel, ends here
 │
 ├─ trait-mod hooks (dependency order; ties in bundle order)
 │    the first Cancel stops the rest; replacement hooks: the first Some wins
 │    collection hooks call every mod
 │
 └─ the game's own body
```

Injections run before trait-mod hooks, and an injection that returns early skips them. Dependency order
never changes injection order. `priority` defaults to 0.

## Make your injection convertible

An injection becomes a hook when its body is a single call a hook can take. Keep to four rules:

- **Bind `Host`**, never `&mut Game`.
- **Never read `Game` fields.** Whatever you need comes from handles, attachments and services reached from the
  host.
- **One call.** Keep the body to a single call into your own code, so the hook's listener is that call.
- **One injection per moment.** Two injections at a moment are two hook requests.

The same behaviour, counting server ticks in an attachment, in the three shapes.

**1. A helper on `Game`** (works today, converts worst: it reads the game itself):

```toml
[[native.injection]]
id = "count"
target = "horizon_client::game::Game<'_>::tick_mobs"
kind = "HEAD"
helper = "src/count.rs"
bindings = [{ name = "game", ty = "&mut crate::game::Game<'_>", value = "&mut *self" }]
```

```rust,ignore
fn count(game: &mut crate::game::Game<'_>) {
    let host: &mut dyn mod_api::Host = game;
    if let Ok(old) = COUNTER.get(&mut *host, Holder::Level) {
        let _ = COUNTER.set(host, Holder::Level, old + 1);
    }
}
```

**2. A trait injection with a `Host` binding** (the default):

```toml
[[native.injection]]
id = "count"
target = "horizon_client::game::Game<'_>::tick_mobs"
kind = "HEAD"
trait_method = "count"
bindings = [{ name = "host", ty = "&mut dyn mod_api::Host", value = "&mut *self" }]
```

```rust,ignore
fn inject_example_count(&self, event: &mut InjectExampleCountEvent<'_>) {
    let _ = count(&mut *event.host);
}
```

**3. The hook it becomes.** Say the catalog has a hook at that moment (the `server_tick` hook, which fires every server tick, stands in here), so the injection disappears and the
mod implements the hook. The listener body is the same call:

```rust,ignore
{{#include ../examples/src/lib.rs:tick_hook}}
```

```rust,ignore
{{#include ../examples/src/lib.rs:count}}
```

The `COUNTER` attachment is the same in all three:

```rust,ignore
{{#include ../examples/src/lib.rs:counter}}
```

Step 2 to step 3 is renaming: the injection's `target` becomes the hook's name, and its event becomes the
hook's. That is what "convertible" means, and why shape 2 is the default.

## Ask for the hook

A gap is a hook request. File one as an issue on
[arismoko/horizon](https://github.com/arismoko/horizon/issues) in the form of a row of the API-gap table in #44.
Copy this, and fill every field:

```markdown
**Hook request: <short name>**

- **What the mod needs:** <one sentence: the moment, and what the mod does there>
- **NeoForge 26.3 counterpart:** <event or method name, or "none">
- **Vanilla class and method:** <`net.minecraft…Class.method`, the call the hook wraps>
- **The injection it replaces:** <the `[[native.injection]]` table, `target`, `kind` and binding>
- **Mod:** <name, repository or `.hmod`>
- **Side:** <server, client or both>
- **Contract wanted:** <listen, cancel, replace a value, collect; what a listener may change>
```

A hook with a NeoForge counterpart takes its name, side and contract. One without is welcome and names the
mod mixin or use it replaces. State the vanilla method: the hook fires where vanilla calls it.

## The build log

Before a build, the launcher reads each mod's injections and writes warnings to `native/build.log`. They never
stop the build:

- Two mods inject at one place: which runs first.
- Two mods wrap one function at its `HEAD` and its `RETURN`.
- A `cancellable` `HEAD` on a hook point: a `Break` skips the later injections and the function's body.
- A raw `target` that is a catalog hook point's function: ``use `hook = "frame"`, `hook_version = 1`, or implement
  `Hooks::frame` with no injection``.
  A scoped hook says "the `<name>` Part method" instead of `Hooks::<name>`.

A panic inside a trait injection reports the mod and the generated method
(`in native mod <id>, hook inject_<mod>_<method>`).

## Support modules

A `[[native.module]]` table contributes a whole Rust module to a game crate, beside the injections:
self-contained items that a helper or trait injection calls through ordinary paths. It is the shape
for support code a moment needs and one function cannot hold — Portizon's runtime is the case that
asked for it.

```toml
[[native.module]]
id = "runtime"
file = "crates/horizon-client/src/game.rs"
name = "portizon"
source = "src/runtime.rs"
visibility = "pub"          # optional: "pub" or "pub(crate)" (the default)
```

`source` holds the module's items without an outer wrapper; the tool emits them into `file` as
`mod name`. The destination file must carry no top-level glob import — horizon-mixins refuses one,
because it cannot prove which names the glob introduces — and the payload must be self-contained: no
`mod` items, `#[path]`, `include!` or file attributes; assemble multi-file code into one file first.
Ids are unique across a package's injections and modules together.

Modules are **native-only** and last-resort support code for helpers, outside the `mod_api`-only
line: a mod's behaviour still goes through hooks, handles and Parts. Any capability a module needs
that the API lacks is a hook request in #44's row form, not a reason to grow the module.

## Reference

- A trait injection's generated method is `inject_<mod_id>_<trait_method>`; its event is that name in CamelCase
  plus `Event`, with the bindings as fields. Implement it on your `Hooks` object: it is called only for your mod.
- Types in bindings must be public and available from both the target and `mod_api`.
- For `RETURN`, `MODIFY_ARG` and `MODIFY_EXPRESSION_VALUE`, the binding whose `value` is `$value` is a plain
  event field; change it in place and the changed value is what the target returns, passes or uses.
  `cancellable` is `HEAD`-only.
- Before the mods are installed, a value passes through unchanged. A trait injection runs in every `Game` that
  calls its target, and attachments answer `MissingHolder` outside the server's game.
- Do not specify `helper` on a `trait_method` entry.
- A trait injection reads `event.host` at any `Game` method horizon-mixins can inject into at HEAD (its README lists
  the limits).
- On a `GameLevel` method that takes `&mut self` (`native_random_tick`, `native_tick_block`,
  `native_perform_bonemeal`, `native_is_bonemeal_success`), bind
  `{ name = "level", ty = "&mut dyn crate::mod_api::LevelRandomHost", value = "&mut *self" }` for attachments and
  `random()`. Scheduling and `set_block_property` stay on the `LevelTicksHost` and `LevelWriteHost` the hook points
  pass.
- A hook point's contract is versioned: `hook_version` names the contract a mod built against. A contract
  never changes; an incompatible change adds a version.
