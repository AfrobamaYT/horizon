# Machine-readable SDK catalog

Use `mod-catalog.json` to inspect the hooks and injection points for the game build
you target. The catalog ships in the prebuilt SDK and as a standalone asset on the
[Nightly release](https://github.com/arismoko/horizon/releases/tag/nightly).
Download the [catalog asset](https://github.com/arismoko/horizon/releases/download/nightly/mod-catalog.json)
and verify it against that release's signed `SHA256SUMS` and build-specific signature.
For an older pinned build, use its dated Nightly release rather than the latest catalog.
The catalog is generated from the release's exact published commit and follows the
release's retention policy.

## Document fields

| Field | Contents |
| --- | --- |
| `schema_version` | `1`. Reject schema versions your tool does not support. |
| `api_version` | The mod manifest API version, independent of the catalog format version. |
| `hooks` | Hook records, sorted by `id`. |
| `points` | Injection point records, sorted by `name`, then `target`. |

The JSON contains no timestamps or absolute paths.

## Hook records

| Field | Contents |
| --- | --- |
| `id` | Hook name. |
| `side` | `server`, `client` or `both`. |
| `scope` | Part scope with `kind` and `key`, or `null`. Kinds are `block`, `blockentity`, `item`, `menu` and `fluid`. |
| `contract` | `listen`, `cancel`, `collect` or `replace`. |
| `arguments` | Ordered argument objects with `name` and `type`. |
| `output` | Output object with `name` and `type`, or `null`. |

Types are Rust API event field types, including their borrow lifetimes.

## Injection point records

| Field | Contents |
| --- | --- |
| `name` | Point name. |
| `target` | Function path used by an injection's `at`. |
| `side` | Point side; `none` for functions outside `Game`. |
| `host` | Listener host kind: `host`, the side's writing host, or `none`. |
| `record` | API listener record name in `mod_points`. |
| `args` | Ordered pairs of parameter name and type. |
| `returns` | Return control: `cancel`, `result` or `none`. |
| `return_ty` | Function return type, with `()` for no return value. |
| `at_return` | Whether API listeners can run at the return. |
| `api_refusal` | Reason API listeners are refused at registration, or `null` when an API record is available. |

## Checking a mod against a build's surface

The SDK catalog above and a build's surface export (`surface.json`) are separate documents. Given the surface
of the build a mod was made for and the mod's folder, write the mod's uses table, then check it against
another build's surface:

```sh
horizon-stage uses MADE_FOR_SURFACE MOD_DIR --json uses.json --made-for BUILD
horizon-stage fits uses.json TARGET_SURFACE
```

`uses` prints each API item the mod uses with the place that brought it in, then how the mod builds each struct
it builds with a literal. The table it writes records, per item, its hash at the made-for build. It also records
the type's trait impls and the public paths the mod writes it by. For each struct the mod names, it records
whether the mod builds it in full, with `..`, or not at all, and the field set when built in full. For a trait's
member, it records every host trait in the mod's list that leads to the member's trait.

`fits` exits 0 when the mod fits (`fits (N items, made for BUILD)`). It exits 1 when it does not, printing
`does not fit: K of the N items it uses changed` and one line per changed item, for example
``the method `PlayersHost::player` (removed)`` or
``the type `DimensionSettings` gained the field `ultrawarm`, and `src/world.rs:12` builds it field by field``.
It exits 2 when an input cannot be read or judged.

The check is a scan of the mod's source, not a compiler. A name it cannot resolve with certainty counts as used
and as built field by field. A struct literal written with a path
(`world::DimensionSettings { … }`) counts for the API's struct of that name; an unqualified one
counts as the mod's own only in the file that defines it, when nothing in that file (a `use`, a
glob, an inline module, a macro or attribute that is not standard or the API's) could change
what the name means. When the scan cannot be sure, for example because a macro it cannot expand
may bring names into scope, it refuses instead of guessing. The real compile on the player's machine is always the final
check. Some sources cannot be judged at
all: a `use` statement nested deeper than 64 groups, a symbolic link, or a `#[path]` or `include!` that leaves
the mod's folder or is computed. A mod with a `[native.*]` table or internals is made for exactly one build, and
this check does not apply to it. A mod that fits is still compiled against the target build, and the binary
built is exact to that build.
