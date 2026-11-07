# Text boundary traversal probe

This small crate compiles the production game Text conversion module directly, without linking Game.
It declares only the two internal conversion trait interfaces. The tests and conversion behavior are
included from `game/modding/api_values/text_boundary.rs`, not copied into this crate.

```sh
cargo test --manifest-path tools/refactors/api-pr3/text-boundary-tests/Cargo.toml --lib
```

The 256 KiB tests use the real conversion traits, including final engine span construction. They cover
512-node round trips and typed `TextFormError::TooDeep` refusals at 513 through children, translation
arguments, selector and NBT separators, atlas and player fallbacks, hover text and hover entity names.
An empty parent with a deep entity-hover name covers disposal of unused styles. Span-only hover
sources are validated before their iterative materialization. A separate test puts 4,096-container
opaque metadata beside an over-depth component to prove refusal precedes metadata cloning.
Comparisons use flat `TextForm` values; recursive trees are deliberately forgotten on the small stack.

The core component tests additionally cover callback order and lazy content selection, hover resolver
isolation, empty first-array style inheritance, structured hover translations and mixed fallback or
unused-argument temporaries containing deep hover trees. Temporary engine text and JSON disposal use
explicit work lists; neither the API nor engine Text type has a custom Drop or Clone implementation.

Native generated host calls and wasm adapter refusal have separate tests in the game and generator
crates. The generator stages all mutable event-field conversions before applying any field. Text-only
host methods return `Result<_, TextFormError>`; existing mixed error channels retain a typed Text case.
A wasm Text form refusal remains typed through guest lowering and host lifting; unrelated malformed
transport values remain transport refusals.

The per-node shallow JSON carrier and its 100-container restriction remain pending the typed-node
replacement. Component-depth preflight runs before payload cloning, and production conversion uses
`TextForm` as its crossing representation. Dynamic content, styles, metadata, object fallback and the
untrusted file-action policy have focused fidelity tests in this probe.

## Ownership versus crossing safety

Recursive `Clone`, `Eq` and `Drop` of a tree the mod itself built are ordinary Rust ownership and are
not part of the boundary guarantee. A guest overflowing its own stack on a 512-deep tree it received
traps within its dependency group. Neither statement excuses recursive boundary traversal.

## Host stack measurements

`stack_need.py` bisects abort-isolated debug processes running the real host builder, conversion,
recursive clone/drop and return conversion with prepared vanilla language data. It includes ordinary
ownership operations, unlike the 256 KiB boundary-only tests. Repeated 4 KiB-resolution measurements
for the iterative implementation passed at these stack sizes:

| Shape | Largest failing bytes | Smallest passing bytes |
| --- | ---: | ---: |
| Extra | 1,249,280 | 1,253,376 |
| Translation argument | 1,536,000 | 1,540,096 |
| Hover text | 1,105,920 | 1,110,016 |
| Selector separator | 561,152 | 565,248 |
| Atlas fallback | 913,408 | 917,504 |
| Entity hover name | 1,105,920 | 1,110,016 |
| NBT separator | 561,152 | 565,248 |
| Player fallback | 913,408 | 917,504 |

These are observed debug-build thresholds, not portable stack guarantees or an entire tick-frame
margin proof. No thread-stack constant is installed here; X3 owns that follow-up. See [X3.md](X3.md)
for the earlier measurement procedure and [thread-inventory.md](thread-inventory.md) for the builders.
