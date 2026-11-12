# Support modules: `[[native.module]]`

A [mixin helper](mixins.md#a-helper) is one function. Code that several helpers share, or that is too long for
one, usually goes in your mod's own crate. A support module is for shared code that needs the game's private
items: the launcher adds your file as a module inside one of the game's source files, in the copy it compiles,
so the code sits in the game's module tree. It is part of the [native route](native-route.md), so it is pinned
to one game build.

## Example

The check mod `fnr_support`, which CI builds against the prebuilt game, adds the module `fnr_runtime` to the
game's autosave file and calls it from a mixin helper.

`horizon-mod.toml` (excerpt):

```toml
[[native.module]]
id = "runtime"
file = "crates/horizon-client/src/game/world/autosave.rs"
name = "fnr_runtime"
source = "src/runtime.rs"
```

`src/runtime.rs`, the module's items, with no `mod` wrapper:

```rust,ignore
pub fn bumped(ticks: i64) -> i64 {
    ticks + 1
}
```

A helper calls it by the module's path in the game crate:

```rust,ignore
fn head(game: &mut Game<'_>) {
    let ticks = game.__horizon_mixins_fnr_support_ticks();
    *ticks = crate::game::autosave::fnr_runtime::bumped(*ticks);
    fnr_support::note(&format!("module and bridge: ticks_until_autosave {}", *ticks));
}
```

The file `crates/horizon-client/src/game/world/autosave.rs` is the module `game::autosave` (folders under
`src/game/` are not modules, see [Naming the target](mixins.md#naming-the-target)), so the module is
`crate::game::autosave::fnr_runtime`. The full mod, with the bridge this helper also uses, is on
[Bridges](bridges.md#example).

## Fields

| Field | Required | Meaning |
| --- | --- | --- |
| `id` | yes | The row's name, unique in the mod across every `[[native.*]]` and `[[inject]]` row. |
| `file` | yes | The game source file that receives the module, from the repository root: `crates/horizon-client/src/…`. A file of the game crate only. |
| `name` | yes | The module's Rust name, an identifier. Pick one the file does not already use; prefixing your mod's id is safe. |
| `source` | yes | Your file under `src/` holding the module's items. |
| `visibility` | no | `"pub(crate)"` (the default) or `"pub"`. |

Inside the module, `super::` is the game module that received it, so the game's types and private items there
are reachable as `super::Game` and so on. Make the items helpers call `pub`. The module's code may call your
mod's crate by its Rust name, as a helper does; your mod's crate cannot call the module, because that crate is
compiled before the copies.

## On success

A module is compiled with the mixins; the build log's `Function replacement:` line covers it.

## Refusals

| Text | Fix |
| --- | --- |
| ``E_MIXIN_MODULE_DESTINATION: <mod>:<row>: <file> is not a file of the game crate (`horizon_client`); a support module goes into one of its files`` | Name a file under `crates/horizon-client/src/`. |
| `E_MIXIN_MODULE_SOURCE: <mod>:<row>: <parse error>` | `source` is not valid Rust items: fix the error shown. |
| `horizon-mod.toml: module file must be a plain path` | Write `file` as a relative path without `..`. |
| `horizon-mod.toml: module name must be an identifier` | Use a Rust identifier. |
| `horizon-mod.toml: module source must be a file under src/` | Move the file under `src/`. |
| `horizon-mod.toml: module visibility must be pub or pub(crate)` | Use one of the two, or leave it out. |
| `horizon-mod.toml: empty or duplicate module id` | Give the row an id no other row uses. |
