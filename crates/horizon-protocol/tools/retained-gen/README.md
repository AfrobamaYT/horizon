# Retained packet allocation traversal

Run from the repository root:

```sh
CARGO_TARGET_DIR="$PWD/target" cargo run --manifest-path crates/horizon-protocol/tools/retained-gen/Cargo.toml
rustfmt --edition 2024 $(git diff --name-only -- '*.rs')
python3 crates/horizon-protocol/tools/audit_retained_std.py
```

`packet!` and `packet_enum!` generate traversal from their existing field and
variant declarations. This tool handles manually declared protocol values with
`syn`; it emits exhaustive struct destructuring and enum matches. Adding a field
or variant without regenerating fails compilation. New field representations
must implement `retained::OwnedHeap`; there is no default-zero escape hatch.
Generic `TrimHolder` and the `move_entity!` macro have explicit traversal beside
their declarations. Test code and custom-payload codec registries are excluded.

## Charge policy

`Clientbound::owned_heap_bytes()` and `Serverbound::owned_heap_bytes()` report a
safe retained-allocation **charge**, not encoded length or an allocator census.
They exclude inline packet/enum storage, allocator metadata and rounding beyond
requested allocation layouts. Callers must charge their own queue/receipt nodes.

Vectors and strings charge capacity, not length, and recursively visit live
values. Boxes charge the boxed inline layout plus descendants. Arc allocations
are charged in full per reference with conservative header/alignment padding,
including item wire-patch caches retained by that packet; sharing can therefore
be overcounted. No global registry or unrelated shared cache is traversed.

NBT compounds use opaque std B-trees. Their conservative node-storage bound was
audited against Rust **1.99.1**, `alloc/src/collections/btree/{map,node,remove}.rs`.
This is an explicit std-layout dependency, not an exact reading of tree capacity:

- `B = 6`, maximum 11 key/value slots per node, at least 5 live entries in each
  non-root node (including after removal rebalancing).
- At most `1 + len / 5` nodes, allowing a retained empty root even for an empty
  map. A fresh empty map is intentionally overcharged too.
- Charge every possible node as an internal node: leaf header (parent pointer,
  two `u16`s), 11 keys, 11 values and 12 child pointers. Eight maximum-alignment
  gaps conservatively cover the five leaf fields, two internal fields and final
  layout rounding regardless of Rust field reordering.
- Live key/value descendants are charged recursively in addition to node slots.

`audit_retained_std.py` checks this layout contract in the installed `rust-src`.
Toolchain upgrades must run this audit and re-examine changed invariants; this
repository's general CI follows stable rather than freezing std's private layout.
No safe public std API exposes a B-tree allocation census, so none is claimed.
Tests exercise insertion/removal histories and retained empty roots instead.
There are no HashMap/HashSet fields in the decoded packet representations; the
custom-payload codec registry uses these but is not retained by a packet and is
not part of this traversal.
