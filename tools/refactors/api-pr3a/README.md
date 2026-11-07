# API PR 3a: item boundary

Part of #1608. This directory holds the repeatable proof and edit log for the
item type swap. The host manifest tool is reused from API PR 1, not duplicated.
`manifest_gate.py` compares method names, owners, documentation and hidden flags
while allowing signatures to change. Every deliberate service change needs an
exact old/new line exception; missing or stale exceptions fail the gate.

## Boundary decisions

`ItemKey` remains a process-local index in the existing runtime key table.
Registration can return a forward reference before the game loads its registry;
a copied key must not carry unresolved item metadata.

The API-owned stack guarantees a nonzero count and represents damage as `u32`.
Its constructors and mutators do not promise registry-dependent validity. The
game owns the item registry and checks the effective max count and max damage
(including per-stack component overrides) at the write boundary, returning a
typed error rather than clamping. Mods can query metadata before attempting a
write. This gives native and wasm mods the same implementation and validation
contract without target-specific globals.

Component data remains an optional, owned NBT patch. Plain stacks need no heap
allocation. Component access is through typed descriptors rather than the
engine's component struct. The wasm record is generated from a declared data
form, not a hand-written stack codec.

## Proof commands

The focused gate tests are independent of Cargo:

```sh
python3 -m unittest discover -s tools/refactors/api-pr3a -p 'test_*.py'
```

The final gate consumes manifests generated at the base and the new head.
Signature changes, WIT record changes and hook dispatch equivalence require
separate review/proofs; the manifest comparison does not claim those are equal.
