# Text boundary traversal probe

This small crate compiles the production game Text conversion module directly, without linking Game.
It declares only the two internal conversion trait interfaces. The tests and conversion behavior are
included from `game/modding/api_values/text_boundary.rs`, not copied into this crate.

Run:

```sh
cargo test --manifest-path tools/refactors/api-pr3/text-boundary-tests/Cargo.toml
```

## Scope of this commit

The capture and JSON emission visitors use an explicit bottom-up work list for all six recursive Text
relations. Retained engine source is borrowed rather than recursively cloned before capture. Object
identity capture does not render its fallback. Translation argument emission shares the engine's
numeric-argument normalization, without constructing and rendering intermediate engine Text trees.

The 256 KiB probe compares flat `TextForm` values and deliberately forgets recursive trees. It tests
the JSON-to-API and API-to-JSON visitors, **not** final engine span construction. Existing fidelity and
untrusted file-action tests are compiled in this probe as well.

**This is preparatory work, not completion of X2's boundary guarantee.** Remaining work:

- Integrate typed `TooDeep` refusal into the conversion traits, generated crossings, host methods and
  their callers. They currently still assume Text conversion is infallible; the generator emits
  `unwrap()` for Text. Simply changing the trait error would turn refusal into a panic.
- Make the flat form the production crossing representation and validate before metadata cloning.
- Bound or replace final engine span construction (`Value -> Text` still calls recursive `build`).
- Cover all trait entry points at 513 nodes, refusing before engine writes.
- Prove host-built clone/drop at actual thread stack sizes with vanilla resources and measure subscribed
  hot-path timing and allocation costs. The current traversal visits each shallow node twice.

## Ownership versus crossing safety

Recursive `Clone`, `Eq` and `Drop` of a tree the mod itself built are ordinary Rust ownership and are
not part of the boundary guarantee. A guest overflowing its own stack on a 512-deep tree it received
traps within its dependency group. Neither statement excuses recursive boundary traversal.

## Stack audit

The Windows main stack is explicitly 8 MiB in `crates/horizon-client/build.rs`. This shell reports
`ulimit -s` = 8192 KiB. Integrated server and network builders do not set an explicit stack size in
this base. Their actual size is not established by Rust's common 2 MiB default; `RUST_MIN_STACK` and
platform defaults matter. No host-built clone/drop stack measurement is claimed here.
