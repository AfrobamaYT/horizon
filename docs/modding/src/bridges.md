# Bridges: `[[native.bridge]]`

A [mixin helper](mixins.md#a-helper) sees what its target function's module sees. A bridge reaches a field or
method that module can't: the launcher generates a method on the game type, and your helpers call it like any
other method. It is part of the [native route](native-route.md), so it is pinned to one game build.

## Example

The check mod `fnr_support`, which CI builds against the prebuilt game, declares a mutable getter for
`Game::ticks_until_autosave` and uses it from a `HEAD` mixin. (That field is visible from the mixin's module
anyway; the check shows the shape.) It also uses a [support module](support-modules.md), `fnr_runtime`.

`horizon-mod.toml` (excerpt; the full header is as on [Mixins](mixins.md#a-first-mixin)):

```toml
[[native.module]]
id = "runtime"
file = "crates/horizon-client/src/game/world/autosave.rs"
name = "fnr_runtime"
source = "src/runtime.rs"

[[native.bridge]]
id = "ticks"
kind = "GET_MUT"
target = "horizon_client::game::Game"
member = "ticks_until_autosave"

[[native.mixin]]
id = "head"
target = "horizon_client::game::autosave::Game<'_>::on_tick_rate_changed"
kind = "HEAD"
helper = "src/mixins/head.rs"
bindings = [{ name = "game", ty = "&mut Game<'_>", value = "&mut *self" }]
```

`src/mixins/head.rs`:

```rust,ignore
fn head(game: &mut Game<'_>) {
    let ticks = game.__horizon_mixins_fnr_support_ticks();
    *ticks = crate::game::autosave::fnr_runtime::bumped(*ticks);
    fnr_support::note(&format!("module and bridge: ticks_until_autosave {}", *ticks));
}
```

The bridge `ticks` of the mod `fnr_support` becomes the method `__horizon_mixins_fnr_support_ticks`, returning
`&mut i64` because the field is an `i64`. Get the type from the game's source: a wrong guess is a compiler
error in your helper.

## The generated method

Its name is `__horizon_mixins_<mod>_<id>`: your mod's id and the row's id, lowercased, with `-` turned into
`_`. Its shape follows `kind`:

| `kind` | Method | `target` | `member` |
| --- | --- | --- | --- |
| `GET` | `fn(&self) -> &T` | The struct: `horizon_client::game::Game` | The field's name, or its index in a tuple struct |
| `GET_MUT` | `fn(&mut self) -> &mut T` | The struct | The field |
| `SET` | `fn(&mut self, value: T)` | The struct | The field |
| `INVOKE` | The method's own signature | The method, written like a [mixin target](mixins.md#naming-the-target) | none |

Write the struct as its module path and name, without lifetimes. Mixin helpers anywhere in the game crate can
call the method; your mod's own crate and support modules cannot.

## Fields

| Field | Required | Meaning |
| --- | --- | --- |
| `id` | yes | The row's name, unique in the mod across every `[[native.*]]` and `[[inject]]` row. It is part of the method's name. |
| `kind` | yes | `GET`, `GET_MUT`, `SET` or `INVOKE`. |
| `target` | yes | The struct (field bridges) or the method (`INVOKE`). |
| `member` | field bridges | The field. |

## Limits

- The struct or method is in the game crate, `horizon_client`, not a crate below it.
- A field bridge's struct is not generic over types or consts (lifetimes are fine: `Game<'a>`).
- An invoker's type and method are not generic.

## On success

A bridge is compiled with the mixins; the build log's `Function replacement:` line covers it. A bridge no helper
calls costs nothing.

## Refusals

Every refusal has the form `E_MIXIN_BRIDGE_TARGET: <mod>:<row>: <why>`. The reasons, and the fix:

| Reason | Fix |
| --- | --- |
| `a field bridge's target is a struct path` | Write `module::Struct`. |
| `no module <module> in the game crate` | Check the module path at your commit ([Naming the target](mixins.md#naming-the-target)). |
| `no struct <Struct> in <module>` | Name the module the struct is declared in. |
| `<Struct> is generic over types` | Not possible on this route. |
| ``a field bridge names its `member` `` | Add `member`. |
| `<Struct> has no field <member>` | Check the field's name at your commit. |
| `unknown bridge kind <kind>` | Use `GET`, `GET_MUT`, `SET` or `INVOKE`. |
| `an invoker's target is a method or associated function of a type` | Point `INVOKE` at a method, not a free function. |
| `an invoker's type and method are not generic` | Not possible on this route. |

An `INVOKE` target that names no function is refused like a mixin's, with `E_MIXIN_TARGET_MISSING` or
`E_MIXIN_TARGET_AMBIGUOUS` ([Mixins](mixins.md#refusals)).
